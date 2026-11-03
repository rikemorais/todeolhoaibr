//! A `TarefaDeBronze`: lê um recurso do `raw`, decodifica e grava Parquet no `bronze`.
//!
//! Implementa o trait [`Tarefa`] do núcleo, então roda no mesmo executor de DAG e aparece
//! no mesmo painel da ingestão. Reaproveita os helpers de procedência do `raw` (manifesto,
//! ponteiro) contra o bucket `bronze` — o layout é o mesmo, só muda o bucket.
//!
//! Procedência: cada part do bronze registra, no manifesto, a tabela de origem e o
//! **validador da versão do raw** (`versao`), para a idempotência: reexecutar só reprocessa
//! quando o raw mudou.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sha2::{Digest, Sha256};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw,
    ParteRaw, ResumoDaTarefa, Tarefa, UltimaVersao, estado_na_raw, manifesto_vigente,
    publicar_ponteiro,
};

use crate::decodificador::Decodificador;

/// `Content-Type` registrado para as partes em Parquet do bronze.
const TIPO_PARQUET: &str = "application/vnd.apache.parquet";

/// Decodifica um recurso do `raw` para o `bronze` (Parquet).
///
/// Lê do bucket `raw` pela porta `origem`; grava no bucket `bronze` pela porta
/// `contexto.armazenamento` injetada pelo executor.
pub struct TarefaDeBronze {
    origem: Arc<dyn ArmazenamentoRaw>,
    fonte: String,
    recurso: String,
    decodificador: Arc<dyn Decodificador>,
}

impl TarefaDeBronze {
    /// Cria a tarefa. `origem` é o armazenamento do `raw` (leitura); o `bronze` (escrita)
    /// vem no contexto do executor.
    pub fn nova(
        origem: Arc<dyn ArmazenamentoRaw>,
        fonte: impl Into<String>,
        recurso: impl Into<String>,
        decodificador: Arc<dyn Decodificador>,
    ) -> Self {
        Self {
            origem,
            fonte: fonte.into(),
            recurso: recurso.into(),
            decodificador,
        }
    }

    fn erro_manifesto(&self, detalhe: String) -> ErroDeIngestao {
        ErroDeIngestao::Manifesto {
            recurso: self.recurso.clone(),
            detalhe,
        }
    }

    /// Validador da versão do raw: o `versao` do ponteiro, ou o id da execução como reserva.
    async fn versao_de_origem(
        &self,
        manifesto_raw: &ManifestoRaw,
    ) -> Result<String, ErroDeIngestao> {
        let chave = LocalizacaoRaw::ultima_versao(&self.fonte, &self.recurso);
        if let Some(bytes) = self.origem.ler(&chave).await?
            && let Ok(ponteiro) = serde_json::from_slice::<UltimaVersao>(&bytes)
            && let Some(versao) = ponteiro.versao
        {
            return Ok(versao);
        }
        Ok(manifesto_raw.id_execucao.clone())
    }
}

#[async_trait]
impl Tarefa for TarefaDeBronze {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let bronze = contexto.armazenamento.as_ref();

        // 1. Procedência do raw. Sem ingestão na origem não há o que refinar.
        let manifesto_raw = manifesto_vigente(self.origem.as_ref(), &self.fonte, &self.recurso)
            .await?
            .ok_or_else(|| ErroDeIngestao::NaoEncontrado {
                url: format!("raw/{}/{}", self.fonte, self.recurso),
            })?;
        let versao_origem = self.versao_de_origem(&manifesto_raw).await?;

        // 2. Idempotência: bronze íntegro e na mesma versão do raw é reaproveitado.
        if let Some(estado) = estado_na_raw(bronze, &self.fonte, &self.recurso).await?
            && estado.completa
            && estado.integra
            && estado.versao.as_deref() == Some(versao_origem.as_str())
        {
            return Ok(ResumoDaTarefa {
                reaproveitada: true,
                ..ResumoDaTarefa::default()
            });
        }

        // 3. Decodifica cada part do raw em tabelas.
        let mut tabelas = Vec::new();
        for parte in &manifesto_raw.partes {
            let bytes = self.origem.ler(&parte.chave).await?.ok_or_else(|| {
                ErroDeIngestao::NaoEncontrado {
                    url: parte.chave.clone(),
                }
            })?;
            let decodificadas = self
                .decodificador
                .decodificar(&bytes, &parte.tipo_de_conteudo)
                .map_err(|erro| self.erro_manifesto(erro.to_string()))?;
            tabelas.extend(decodificadas);
        }

        // 4. Grava uma parte Parquet por tabela no bronze.
        let local = LocalizacaoRaw::nova(
            &self.fonte,
            &self.recurso,
            contexto.iniciada_em,
            &contexto.id_execucao,
        );
        let mut partes = Vec::with_capacity(tabelas.len());
        let mut total_bytes = 0u64;
        for (indice, tabela) in tabelas.iter().enumerate() {
            let parquet = tabela
                .tabela
                .para_parquet()
                .map_err(|erro| self.erro_manifesto(erro.to_string()))?;
            let chave = local.parte(indice + 1, "parquet");
            let bytes = parquet.len() as u64;
            total_bytes += bytes;
            let sha256 = hex::encode(Sha256::digest(&parquet));
            bronze
                .gravar(ObjetoRaw {
                    chave: chave.clone(),
                    conteudo: parquet,
                    tipo_de_conteudo: TIPO_PARQUET.to_owned(),
                })
                .await?;
            partes.push(ParteRaw {
                chave,
                url: tabela.nome.clone(),
                bytes,
                sha256,
                tipo_de_conteudo: TIPO_PARQUET.to_owned(),
                versao: Some(versao_origem.clone()),
            });
        }

        // 5. Manifesto do bronze (procedência aponta para o recurso do raw).
        let manifesto = ManifestoRaw {
            fonte: self.fonte.clone(),
            recurso: self.recurso.clone(),
            id_execucao: contexto.id_execucao.clone(),
            url_inicial: format!("raw/{}/{}", self.fonte, self.recurso),
            iniciada_em: contexto.iniciada_em,
            concluida_em: Utc::now(),
            partes: partes.clone(),
        };
        let conteudo = serde_json::to_vec_pretty(&manifesto)
            .map_err(|erro| self.erro_manifesto(erro.to_string()))?;
        bronze
            .gravar(ObjetoRaw {
                chave: local.manifesto(),
                conteudo,
                tipo_de_conteudo: "application/json".to_owned(),
            })
            .await?;

        // 6. Ponteiro do bronze, com a versão do raw para a idempotência da próxima rodada.
        let ponteiro = UltimaVersao {
            versao: Some(versao_origem),
            manifesto: local.manifesto(),
            iniciada_em: Some(contexto.iniciada_em),
        };
        publicar_ponteiro(bronze, &self.fonte, &self.recurso, &ponteiro).await?;

        Ok(ResumoDaTarefa {
            partes: partes.len(),
            bytes: total_bytes,
            reaproveitada: false,
            motivo: None,
        })
    }
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use chrono::{TimeZone, Utc};
    use todeolho_nucleo::{ClienteHttp, DownloadHttp, InformacoesDoArquivo, RespostaHttp};

    use super::*;
    use crate::decodificador::TabelaNomeada;
    use crate::escritor::{ErroDeRefino, Tabela};

    #[derive(Default)]
    struct RawEmMemoria {
        objetos: Mutex<HashMap<String, Vec<u8>>>,
    }

    #[async_trait]
    impl ArmazenamentoRaw for RawEmMemoria {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            self.objetos
                .lock()
                .unwrap()
                .insert(objeto.chave, objeto.conteudo);
            Ok(())
        }
        async fn gravar_fluxo(
            &self,
            _: &str,
            _: &str,
            _: &mut (dyn tokio::io::AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            unimplemented!("o bronze não grava por fluxo")
        }
        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.objetos.lock().unwrap().get(chave).cloned())
        }
    }

    struct HttpInerte;

    #[async_trait]
    impl ClienteHttp for HttpInerte {
        async fn obter(&self, _: &str, _: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            unimplemented!("o bronze não usa HTTP")
        }
        async fn informacoes(&self, _: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            unimplemented!("o bronze não usa HTTP")
        }
        async fn baixar(&self, _: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            unimplemented!("o bronze não usa HTTP")
        }
    }

    /// Decodificador de teste: ignora os bytes e devolve uma tabela fixa.
    struct DecodificadorFixo;
    impl Decodificador for DecodificadorFixo {
        fn decodificar(&self, _: &[u8], _: &str) -> Result<Vec<TabelaNomeada>, ErroDeRefino> {
            Ok(vec![TabelaNomeada::nova(
                "despesas",
                Tabela::nova(["id", "valor"])
                    .com_linha([Some("1".into()), Some("10,00".into())])
                    .com_linha([Some("2".into()), None]),
            )])
        }
    }

    const FONTE: &str = "camara";
    const RECURSO: &str = "despesas/ano=2024";

    /// Semeia o raw com ponteiro + manifesto + uma parte, como se já tivesse sido ingerido.
    fn semear_raw(raw: &RawEmMemoria, versao: &str) {
        let dt = Utc.with_ymd_and_hms(2026, 11, 3, 10, 0, 0).unwrap();
        let local = LocalizacaoRaw::nova(FONTE, RECURSO, dt, "RAWEXEC");
        let chave_parte = local.parte(1, "zip");
        {
            let mut o = raw.objetos.lock().unwrap();
            o.insert(chave_parte.clone(), b"zip-bytes".to_vec());
        }
        let manifesto = ManifestoRaw {
            fonte: FONTE.into(),
            recurso: RECURSO.into(),
            id_execucao: "RAWEXEC".into(),
            url_inicial: "https://origem/despesas".into(),
            iniciada_em: dt,
            concluida_em: dt,
            partes: vec![ParteRaw {
                chave: chave_parte,
                url: "https://origem/despesas".into(),
                bytes: 9,
                sha256: "x".into(),
                tipo_de_conteudo: "application/zip".into(),
                versao: Some(versao.into()),
            }],
        };
        let ponteiro = UltimaVersao {
            versao: Some(versao.into()),
            manifesto: local.manifesto(),
            iniciada_em: Some(dt),
        };
        let mut o = raw.objetos.lock().unwrap();
        o.insert(local.manifesto(), serde_json::to_vec(&manifesto).unwrap());
        o.insert(
            LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
            serde_json::to_vec(&ponteiro).unwrap(),
        );
    }

    fn contexto(bronze: Arc<dyn ArmazenamentoRaw>, id: &str) -> ContextoDeExecucao {
        ContextoDeExecucao {
            id_execucao: id.into(),
            iniciada_em: Utc.with_ymd_and_hms(2026, 11, 3, 12, 0, 0).unwrap(),
            armazenamento: bronze,
            http: Arc::new(HttpInerte),
            progresso: None,
        }
    }

    fn tarefa(raw: Arc<dyn ArmazenamentoRaw>) -> TarefaDeBronze {
        TarefaDeBronze::nova(raw, FONTE, RECURSO, Arc::new(DecodificadorFixo))
    }

    #[tokio::test]
    async fn decodifica_o_raw_e_grava_parquet_no_bronze() {
        let raw = Arc::new(RawEmMemoria::default());
        semear_raw(&raw, "etag-1");
        let bronze = Arc::new(RawEmMemoria::default());

        let resumo = tarefa(raw.clone())
            .executar(&contexto(bronze.clone(), "BR1"))
            .await
            .unwrap();

        assert!(!resumo.reaproveitada);
        assert_eq!(resumo.partes, 1);
        assert!(resumo.bytes > 0);

        let objetos = bronze.objetos.lock().unwrap();
        assert!(
            objetos.keys().any(|c| c.ends_with("parte-00001.parquet")),
            "faltou a parte parquet: {:?}",
            objetos.keys().collect::<Vec<_>>()
        );
        assert!(objetos.contains_key(&LocalizacaoRaw::ultima_versao(FONTE, RECURSO)));
    }

    #[tokio::test]
    async fn reaproveita_quando_o_bronze_ja_esta_na_versao_do_raw() {
        let raw = Arc::new(RawEmMemoria::default());
        semear_raw(&raw, "etag-1");
        let bronze = Arc::new(RawEmMemoria::default());

        tarefa(raw.clone())
            .executar(&contexto(bronze.clone(), "BR1"))
            .await
            .unwrap();
        let segunda = tarefa(raw.clone())
            .executar(&contexto(bronze.clone(), "BR2"))
            .await
            .unwrap();

        assert!(
            segunda.reaproveitada,
            "deveria reaproveitar o bronze íntegro"
        );
        assert_eq!(segunda.partes, 0);
    }

    #[tokio::test]
    async fn reprocessa_quando_o_raw_muda_de_versao() {
        let raw = Arc::new(RawEmMemoria::default());
        semear_raw(&raw, "etag-1");
        let bronze = Arc::new(RawEmMemoria::default());
        tarefa(raw.clone())
            .executar(&contexto(bronze.clone(), "BR1"))
            .await
            .unwrap();

        // O raw é reingerido com um validador novo.
        semear_raw(&raw, "etag-2");
        let depois = tarefa(raw.clone())
            .executar(&contexto(bronze.clone(), "BR3"))
            .await
            .unwrap();

        assert!(!depois.reaproveitada, "raw mudou: deveria reprocessar");
        assert_eq!(depois.partes, 1);
    }

    #[tokio::test]
    async fn falha_quando_nao_ha_recurso_no_raw() {
        let raw = Arc::new(RawEmMemoria::default());
        let bronze = Arc::new(RawEmMemoria::default());
        let erro = tarefa(raw)
            .executar(&contexto(bronze, "BR1"))
            .await
            .unwrap_err();
        assert!(
            matches!(erro, ErroDeIngestao::NaoEncontrado { .. }),
            "{erro:?}"
        );
    }
}
