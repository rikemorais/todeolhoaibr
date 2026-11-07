//! Tarefa de publicação Silver dos eventos de tramitação da Câmara.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    camara_tramitacoes::{ano_do_recurso, esquema_camara_tramitacoes},
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql_com_json_string_original},
};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

const REGRA: &str = "camara-tramitacao-evento-v2";
const SQL: &str = include_str!("../../sql/silver/camara_tramitacoes.sql");

/// Tarefa Silver para um recurso anual de tramitações Bronze.
pub struct TarefaDeSilverCamaraTramitacoes {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    ano: u16,
}

impl TarefaDeSilverCamaraTramitacoes {
    /// Cria a tarefa apenas para recursos que declaram o ano no caminho.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Option<Self> {
        let recurso = recurso.into();
        let ano = ano_do_recurso(&recurso)?;
        Some(Self {
            bronze,
            snapshot,
            recurso,
            ano,
        })
    }

    /// Indica se a Silver precisa acompanhar o snapshot Bronze e a regra atuais.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "camara", &self.recurso).await?,
            Some(ref estado) if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverCamaraTramitacoes {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_camara_tramitacoes(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            &self.recurso,
            self.ano,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Publica o recurso anual sem remover registros repetidos da origem.
pub async fn publicar_camara_tramitacoes(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    ano: u16,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    if ano_do_recurso(recurso) != Some(ano) {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: "ano declarado não corresponde ao recurso Bronze".into(),
        });
    }
    let sql = SQL.replace("{{ANO_RECURSO}}", &ano.to_string());
    publicar_transformacao_sql_com_json_string_original(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "camara",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_camara_tramitacoes()),
            iniciada_em,
            id_execucao,
        },
        &sql,
        &ConfiguracaoExecutorSql::default(),
    )
    .await
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;

    use arrow::array::StringArray;
    use arrow::record_batch::RecordBatch;
    use async_trait::async_trait;
    use bytes::Bytes;
    use chrono::Utc;
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use sha2::{Digest, Sha256};
    use todeolho_nucleo::{LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw, UltimaVersao};
    use tokio::io::{AsyncRead, AsyncReadExt};

    use super::*;

    #[derive(Default)]
    struct Memoria(std::sync::Mutex<HashMap<String, Vec<u8>>>);

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

    async fn snapshot() -> (Memoria, Arc<SnapshotBronze>) {
        let bronze = Memoria::default();
        let inicio = Utc::now();
        let recurso = "tramitacoes/ano=2024";
        let local = todeolho_nucleo::LocalizacaoRaw::nova("camara", recurso, inicio, "bronze-1");
        let lote = RecordBatch::try_from_iter(vec![
            (
                "uriProposicao",
                Arc::new(StringArray::from(vec![
                    "https://dadosabertos.camara.leg.br/api/v2/proposicoes/2431810",
                ])) as _,
            ),
            ("sequencia", Arc::new(StringArray::from(vec!["8"])) as _),
            (
                "dataHora",
                Arc::new(StringArray::from(vec!["2024-07-11T09:00:00"])) as _,
            ),
            ("idTramitacao", Arc::new(StringArray::from(vec!["9"])) as _),
        ])
        .unwrap();
        let mut bytes = Vec::new();
        let mut writer =
            parquet::arrow::ArrowWriter::try_new(&mut bytes, lote.schema(), None).unwrap();
        writer.write(&lote).unwrap();
        writer.close().unwrap();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "proposicoesTramitacoes-2024.json".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "camara".into(), recurso: recurso.into(), id_execucao: "bronze-1".into(),
            url_inicial: "https://dadosabertos.camara.leg.br/arquivos/proposicoesTramitacoes/json/proposicoesTramitacoes-2024.json".into(),
            iniciada_em: inicio, concluida_em: Utc::now(), partes: vec![parte.clone()],
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
                chave: LocalizacaoRaw::ultima_versao("camara", recurso),
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
        let snapshot = SnapshotBronze::vigente(&bronze, "camara", recurso)
            .await
            .unwrap()
            .unwrap();
        (bronze, Arc::new(snapshot))
    }

    fn lote_silver(silver: &Memoria, recurso: &str) -> RecordBatch {
        let objetos = silver.0.lock().unwrap();
        let ponteiro: UltimaVersao =
            serde_json::from_slice(&objetos[&LocalizacaoRaw::ultima_versao("camara", recurso)])
                .unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&objetos[&ponteiro.manifesto]).unwrap();
        ParquetRecordBatchReaderBuilder::try_new(Bytes::copy_from_slice(
            &objetos[&manifesto.partes[0].chave],
        ))
        .unwrap()
        .build()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
    }

    #[tokio::test]
    async fn publica_recurso_e_reconhece_snapshot_vigente() {
        let (bronze, snapshot) = snapshot().await;
        let silver = Memoria::default();
        let tarefa = TarefaDeSilverCamaraTramitacoes::nova(
            Arc::new(Memoria::default()),
            Arc::clone(&snapshot),
            "tramitacoes/ano=2024",
        )
        .unwrap();
        assert!(tarefa.pendente(&silver).await.unwrap());
        let resumo = publicar_camara_tramitacoes(
            &bronze,
            &silver,
            &snapshot,
            "tramitacoes/ano=2024",
            2024,
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);
        let bytes_bronze = bronze
            .ler(&snapshot.manifesto().partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let lote_bronze = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(bytes_bronze))
            .unwrap()
            .build()
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let esperado =
            super::super::camara_tramitacoes::transformar_camara_tramitacoes(&lote_bronze, 2024)
                .unwrap();
        assert_eq!(lote_silver(&silver, "tramitacoes/ano=2024"), esperado);
        assert!(!tarefa.pendente(&silver).await.unwrap());
        assert!(
            publicar_camara_tramitacoes(
                &bronze,
                &silver,
                &snapshot,
                "tramitacoes/ano=2024",
                2024,
                Utc::now(),
                "silver-2",
            )
            .await
            .unwrap()
            .reaproveitada
        );
    }

    #[tokio::test]
    async fn rejeita_ano_divergente_do_recurso() {
        let (bronze, snapshot) = snapshot().await;
        let erro = publicar_camara_tramitacoes(
            &bronze,
            &Memoria::default(),
            &snapshot,
            "tramitacoes/ano=2024",
            2023,
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap_err();
        assert!(matches!(erro, ErroDeIngestao::Manifesto { .. }));
    }

    #[tokio::test]
    async fn tarefa_nao_aceita_recurso_sem_ano() {
        let (_, snapshot) = snapshot().await;
        assert!(
            TarefaDeSilverCamaraTramitacoes::nova(
                Arc::new(Memoria::default()),
                snapshot,
                "tramitacoes/sem-particao",
            )
            .is_none()
        );
    }
}
