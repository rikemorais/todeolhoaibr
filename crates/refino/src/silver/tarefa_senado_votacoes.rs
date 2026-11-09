//! Tarefa Silver para as votações anuais do Senado.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql},
    senado_votos::esquema_senado_votos,
};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

#[cfg(test)]
use super::senado_votos::transformar_senado_votos;

const REGRA: &str = "senado-votacao-v2";
const REGRA_VOTOS: &str = "senado-voto-nominal-v2";
const SQL_VOTACOES: &str = include_str!("../../sql/silver/senado_votacoes.sql");
const SQL_VOTOS: &str = include_str!("../../sql/silver/senado_votos.sql");

/// Tarefa para publicar um recurso anual de votações do Senado.
pub struct TarefaDeSilverSenadoVotacoes {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    destino: String,
    votos: bool,
    ambas: bool,
}

impl TarefaDeSilverSenadoVotacoes {
    /// Cria a tarefa legada que publica sessões e votos do ano.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Self {
        let recurso = recurso.into();
        Self {
            bronze,
            snapshot,
            destino: recurso.clone(),
            recurso,
            votos: false,
            ambas: true,
        }
    }

    /// Cria somente a tarefa de sessões para execução com estado por tabela.
    pub fn nova_sessoes(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Self {
        let mut tarefa = Self::nova(bronze, snapshot, recurso);
        tarefa.ambas = false;
        tarefa
    }

    /// Cria a tarefa da tabela filha de votos, com nome próprio no DAG.
    pub fn nova_votos(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Result<Self, ErroDeIngestao> {
        let recurso = recurso.into();
        let destino = recurso_votos(&recurso).ok_or_else(|| ErroDeIngestao::Manifesto {
            recurso: recurso.clone(),
            detalhe: "recurso de votações sem ano no caminho".into(),
        })?;
        Ok(Self {
            bronze,
            snapshot,
            recurso,
            destino,
            votos: true,
            ambas: false,
        })
    }

    /// Indica se a Silver acompanha o snapshot e a regra de transformação atuais.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let execucao = &self.snapshot.manifesto().id_execucao;
        let regra = if self.votos { REGRA_VOTOS } else { REGRA };
        let versao = format!("{execucao}|regra={regra}");
        let atual = matches!(
            estado_na_raw(silver, "senado", &self.destino).await?,
            Some(ref estado) if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        );
        if self.ambas {
            let destino_votos =
                recurso_votos(&self.recurso).ok_or_else(|| ErroDeIngestao::Manifesto {
                    recurso: self.recurso.clone(),
                    detalhe: "recurso de votações sem ano no caminho".into(),
                })?;
            let versao_votos = format!("{execucao}|regra={REGRA_VOTOS}");
            let votos_atuais = matches!(
                estado_na_raw(silver, "senado", &destino_votos).await?,
                Some(ref estado) if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao_votos.as_str())
            );
            return Ok(!atual || !votos_atuais);
        }
        Ok(!atual)
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverSenadoVotacoes {
    fn nome(&self) -> &str {
        &self.destino
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        if self.votos {
            return publicar_senado_votos(
                self.bronze.as_ref(),
                contexto.armazenamento.as_ref(),
                &self.snapshot,
                &self.recurso,
                contexto.iniciada_em,
                &contexto.id_execucao,
            )
            .await;
        }
        let sessoes = publicar_senado_votacoes(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            &self.recurso,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await?;
        if self.ambas {
            let votos = publicar_senado_votos(
                self.bronze.as_ref(),
                contexto.armazenamento.as_ref(),
                &self.snapshot,
                &self.recurso,
                contexto.iniciada_em,
                &contexto.id_execucao,
            )
            .await?;
            return Ok(ResumoDaTarefa {
                partes: sessoes.partes + votos.partes,
                bytes: sessoes.bytes + votos.bytes,
                reaproveitada: sessoes.reaproveitada && votos.reaproveitada,
                motivo: None,
            });
        }
        Ok(sessoes)
    }
}

fn recurso_votos(recurso: &str) -> Option<String> {
    let ano = recurso.strip_prefix("votacoes/ano=")?;
    if ano.len() != 4 || !ano.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(format!("votos/ano={ano}"))
}

/// Expande e publica a relação filha com um registro por voto parlamentar.
pub async fn publicar_senado_votos(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    let destino = recurso_votos(recurso).ok_or_else(|| ErroDeIngestao::Manifesto {
        recurso: recurso.to_owned(),
        detalhe: "recurso de votações sem ano no caminho".into(),
    })?;
    publicar_transformacao_sql(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "senado",
            recurso,
            recurso_destino: Some(&destino),
            regra: REGRA_VOTOS,
            esquema: Some(esquema_senado_votos()),
            iniciada_em,
            id_execucao,
        },
        SQL_VOTOS,
        &ConfiguracaoExecutorSql::default(),
    )
    .await
}

/// Valida a chave da sessão e publica a estrutura original, mantendo os votos aninhados.
pub async fn publicar_senado_votacoes(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    if !recurso.starts_with("votacoes/ano=") {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: "recurso de votações sem ano no caminho".into(),
        });
    }
    publicar_transformacao_sql(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "senado",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: None,
            iniciada_em,
            id_execucao,
        },
        SQL_VOTACOES,
        &ConfiguracaoExecutorSql::default(),
    )
    .await
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;

    use arrow::{array::StringArray, record_batch::RecordBatch};
    use async_trait::async_trait;
    use bytes::Bytes;
    use chrono::Utc;
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use sha2::{Digest, Sha256};
    use todeolho_nucleo::{
        ClienteHttp, DownloadHttp, InformacoesDoArquivo, LocalizacaoRaw, ManifestoRaw, ObjetoRaw,
        ParteRaw, RespostaHttp, UltimaVersao,
    };
    use tokio::io::{AsyncRead, AsyncReadExt};

    use super::*;

    #[derive(Default)]
    struct Memoria(std::sync::Mutex<HashMap<String, Vec<u8>>>);

    struct HttpInerte;

    #[async_trait]
    impl ClienteHttp for HttpInerte {
        async fn obter(&self, _: &str, _: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            unreachable!()
        }
        async fn informacoes(&self, _: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            unreachable!()
        }
        async fn baixar(&self, _: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            unreachable!()
        }
    }

    #[async_trait]
    impl ArmazenamentoRaw for Memoria {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            self.0.lock().unwrap().insert(objeto.chave, objeto.conteudo);
            Ok(())
        }
        async fn gravar_fluxo(
            &self,
            chave: &str,
            _: &str,
            fluxo: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            let mut bytes = Vec::new();
            fluxo
                .read_to_end(&mut bytes)
                .await
                .map_err(|erro| ErroDeIngestao::Armazenamento {
                    chave: chave.into(),
                    detalhe: erro.to_string(),
                })?;
            self.0.lock().unwrap().insert(chave.into(), bytes);
            Ok(())
        }
        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.0.lock().unwrap().get(chave).cloned())
        }
    }

    async fn fonte(ids: &[&str]) -> (Memoria, Arc<SnapshotBronze>) {
        let votos = vec![Some(r#"[{"codigoParlamentar":"7","voto":"Sim"}]"#); ids.len()];
        fonte_com_votos(ids, &votos).await
    }

    async fn fonte_com_votos(
        ids: &[&str],
        json_votos: &[Option<&str>],
    ) -> (Memoria, Arc<SnapshotBronze>) {
        assert_eq!(ids.len(), json_votos.len());
        let bronze = Memoria::default();
        let inicio = Utc::now();
        let recurso = "votacoes/ano=2024";
        let local = LocalizacaoRaw::nova("senado", recurso, inicio, "bronze-1");
        let datas = vec!["2024-02-01"; ids.len()];
        let lote = RecordBatch::try_from_iter(vec![
            (
                "codigoSessaoVotacao",
                Arc::new(StringArray::from(ids.to_vec())) as _,
            ),
            ("dataSessao", Arc::new(StringArray::from(datas)) as _),
            (
                "votos",
                Arc::new(StringArray::from(json_votos.to_vec())) as _,
            ),
        ])
        .unwrap();
        let mut bytes = Vec::new();
        let mut writer =
            parquet::arrow::ArrowWriter::try_new(&mut bytes, lote.schema(), None).unwrap();
        writer.write(&lote).unwrap();
        writer.close().unwrap();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "votacao?ano=2024".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "senado".into(),
            recurso: recurso.into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "https://legis.senado.leg.br/dadosabertos/votacao?ano=2024".into(),
            iniciada_em: inicio,
            concluida_em: Utc::now(),
            partes: vec![parte.clone()],
        };
        for objeto in [
            ObjetoRaw {
                chave: parte.chave,
                conteudo: bytes,
                tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            },
            ObjetoRaw {
                chave: local.manifesto(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            },
            ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao("senado", recurso),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some("bronze-1".into()),
                    manifesto: local.manifesto(),
                    iniciada_em: Some(inicio),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            },
        ] {
            bronze.gravar(objeto).await.unwrap();
        }
        let snapshot = SnapshotBronze::vigente(&bronze, "senado", recurso)
            .await
            .unwrap()
            .unwrap();
        (bronze, Arc::new(snapshot))
    }

    #[tokio::test]
    async fn publica_e_reaproveita_sessao_preservando_votos() {
        let (bronze, snapshot) = fonte(&["S1"]).await;
        let silver = Memoria::default();
        let recurso = "votacoes/ano=2024";
        let tarefa = TarefaDeSilverSenadoVotacoes::nova_sessoes(
            Arc::new(Memoria::default()),
            snapshot.clone(),
            recurso,
        );
        let tarefa_votos = TarefaDeSilverSenadoVotacoes::nova_votos(
            Arc::new(Memoria::default()),
            snapshot.clone(),
            recurso,
        )
        .unwrap();
        assert_ne!(tarefa.nome(), tarefa_votos.nome());
        assert!(tarefa.pendente(&silver).await.unwrap());
        assert!(tarefa_votos.pendente(&silver).await.unwrap());
        let resumo =
            publicar_senado_votacoes(&bronze, &silver, &snapshot, recurso, Utc::now(), "silver-1")
                .await
                .unwrap();
        assert_eq!(resumo.partes, 1);
        assert!(!tarefa.pendente(&silver).await.unwrap());
        assert!(tarefa_votos.pendente(&silver).await.unwrap());
        let votos =
            publicar_senado_votos(&bronze, &silver, &snapshot, recurso, Utc::now(), "silver-1")
                .await
                .unwrap();
        assert_eq!(votos.partes, 1);
        assert!(!tarefa_votos.pendente(&silver).await.unwrap());
        assert!(
            silver
                .ler(&LocalizacaoRaw::ultima_versao("senado", "votos/ano=2024"))
                .await
                .unwrap()
                .is_some()
        );
        assert!(!tarefa.pendente(&silver).await.unwrap());
        assert!(
            publicar_senado_votacoes(&bronze, &silver, &snapshot, recurso, Utc::now(), "silver-2",)
                .await
                .unwrap()
                .reaproveitada
        );
    }

    #[tokio::test]
    async fn construtor_legado_publica_sessoes_e_votos_pelo_executar() {
        let (bronze, snapshot) = fonte(&["S1"]).await;
        let bronze = Arc::new(bronze);
        let silver = Arc::new(Memoria::default());
        let tarefa = TarefaDeSilverSenadoVotacoes::nova(bronze, snapshot, "votacoes/ano=2024");
        let contexto = ContextoDeExecucao {
            id_execucao: "silver-legado".into(),
            iniciada_em: Utc::now(),
            armazenamento: silver.clone(),
            http: Arc::new(HttpInerte),
            progresso: None,
        };
        assert!(tarefa.pendente(silver.as_ref()).await.unwrap());
        assert_eq!(tarefa.executar(&contexto).await.unwrap().partes, 2);
        assert!(!tarefa.pendente(silver.as_ref()).await.unwrap());
        assert!(
            silver
                .ler(&LocalizacaoRaw::ultima_versao("senado", "votos/ano=2024"))
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn sql_preserva_ordem_e_equivale_ao_arrow_em_envelopes_atual_e_legado() {
        let fixtures = [
            Some(
                r#"[{"codigoParlamentar":7,"nomeParlamentar":"Senadora A","siglaPartido":"ABC","siglaUf":"DF","voto":"Sim"},{"IdentificacaoParlamentar":{"CodigoParlamentar":"008","NomeParlamentar":"Senador B"},"Voto":"Não"}]"#,
            ),
            Some(
                r#"{"Votos":{"VotoParlamentar":{"IdentificacaoParlamentar":{"CodigoParlamentar":9,"NomeParlamentar":"Senador C","SiglaPartidoParlamentar":"XYZ","UfParlamentar":"SP"},"DescricaoVoto":"Abstenção"}}}"#,
            ),
            Some("[]"),
            None,
        ];
        let (bronze, snapshot) = fonte_com_votos(&["S1", "S2", "S3", "S4"], &fixtures).await;
        let lote_bronze = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(
            bronze
                .ler(&snapshot.manifesto().partes[0].chave)
                .await
                .unwrap()
                .unwrap(),
        ))
        .unwrap()
        .build()
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
        let esperado = transformar_senado_votos(&lote_bronze).unwrap();
        let silver = Memoria::default();
        publicar_senado_votacoes(
            &bronze,
            &silver,
            &snapshot,
            "votacoes/ano=2024",
            Utc::now(),
            "silver-json-parity",
        )
        .await
        .unwrap();
        publicar_senado_votos(
            &bronze,
            &silver,
            &snapshot,
            "votacoes/ano=2024",
            Utc::now(),
            "silver-json-parity",
        )
        .await
        .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(
            &silver
                .ler(&LocalizacaoRaw::ultima_versao("senado", "votos/ano=2024"))
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let saida = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(
            silver
                .ler(&manifesto.partes[0].chave)
                .await
                .unwrap()
                .unwrap(),
        ))
        .unwrap()
        .build()
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
        assert_eq!(saida, esperado);
        assert_eq!(saida.num_rows(), 3);

        let ponteiro_sessoes: UltimaVersao = serde_json::from_slice(
            &silver
                .ler(&LocalizacaoRaw::ultima_versao(
                    "senado",
                    "votacoes/ano=2024",
                ))
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let manifesto_sessoes: ManifestoRaw = serde_json::from_slice(
            &silver
                .ler(&ponteiro_sessoes.manifesto)
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let sessoes = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(
            silver
                .ler(&manifesto_sessoes.partes[0].chave)
                .await
                .unwrap()
                .unwrap(),
        ))
        .unwrap()
        .build()
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
        assert_eq!(sessoes.num_rows(), 4);
    }

    #[tokio::test]
    async fn nao_publica_recurso_que_nao_seja_votacao_anual() {
        let (bronze, snapshot) = fonte(&["S1"]).await;
        assert!(
            publicar_senado_votacoes(
                &bronze,
                &Memoria::default(),
                &snapshot,
                "votacoes/recente",
                Utc::now(),
                "silver-1",
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn rejeita_duplicata_no_recurso_e_anula_publicacao() {
        let (bronze, snapshot) = fonte(&["S1", "S1"]).await;
        let silver = Memoria::default();
        assert!(
            publicar_senado_votacoes(
                &bronze,
                &silver,
                &snapshot,
                "votacoes/ano=2024",
                Utc::now(),
                "silver-1",
            )
            .await
            .is_err()
        );
        assert!(
            silver
                .ler(&LocalizacaoRaw::ultima_versao(
                    "senado",
                    "votacoes/ano=2024"
                ))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn recusa_publicar_votos_fora_de_um_recurso_anual() {
        let (bronze, snapshot) = fonte(&["S1"]).await;
        let silver = Memoria::default();
        let erro = publicar_senado_votos(
            &bronze,
            &silver,
            &snapshot,
            "votacoes/recente",
            Utc::now(),
            "silver-1",
        )
        .await;
        assert!(erro.is_err());
    }

    #[tokio::test]
    async fn json_de_votos_invalido_impede_publicacao() {
        let (bronze, snapshot) = fonte_com_votos(&["S1"], &[Some("não é JSON")]).await;
        let silver = Memoria::default();
        assert!(
            publicar_senado_votos(
                &bronze,
                &silver,
                &snapshot,
                "votacoes/ano=2024",
                Utc::now(),
                "silver-invalid-json",
            )
            .await
            .is_err()
        );
        assert!(
            silver
                .ler(&LocalizacaoRaw::ultima_versao("senado", "votos/ano=2024"))
                .await
                .unwrap()
                .is_none()
        );
    }
}
