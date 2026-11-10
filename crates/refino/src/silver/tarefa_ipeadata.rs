use std::sync::Arc;

use async_trait::async_trait;
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::ipeadata::esquema_ipeadata;
use super::publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

const REGRA: &str = "ipeadata-observacao-v1";
const SQL: &str = include_str!("../../sql/silver/ipeadata.sql");

/// Publica uma série do Ipeadata a partir de um snapshot Bronze.
pub struct TarefaDeSilverIpeadata {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
}

impl TarefaDeSilverIpeadata {
    /// Cria a tarefa para o recurso do Ipeadata indicado.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Self {
        Self {
            bronze,
            snapshot,
            recurso: recurso.into(),
        }
    }

    /// Informa se o snapshot ainda precisa ser publicado na camada Silver.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "ipeadata", &self.recurso).await?,
            Some(ref estado) if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverIpeadata {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_transformacao_sql(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            ConfiguracaoDePublicacao {
                fonte: "ipeadata",
                recurso: &self.recurso,
                recurso_destino: None,
                regra: REGRA,
                esquema: Some(esquema_ipeadata()),
                iniciada_em: contexto.iniciada_em,
                id_execucao: &contexto.id_execucao,
            },
            SQL,
            &ConfiguracaoExecutorSql::default(),
        )
        .await
    }
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;

    use arrow::array::StringArray;
    use arrow::record_batch::RecordBatch;
    use async_trait::async_trait;
    use chrono::Utc;
    use sha2::{Digest, Sha256};
    use todeolho_nucleo::{
        ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw, UltimaVersao,
    };
    use tokio::io::{AsyncRead, AsyncReadExt};

    use super::*;

    const TIPO_PARQUET: &str = "application/vnd.apache.parquet";

    #[derive(Default)]
    struct ArmazenamentoEmMemoria {
        objetos: std::sync::Mutex<HashMap<String, Vec<u8>>>,
    }

    #[async_trait]
    impl ArmazenamentoRaw for ArmazenamentoEmMemoria {
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
            chave: &str,
            _tipo_de_conteudo: &str,
            fluxo: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            let mut conteudo = Vec::new();
            fluxo.read_to_end(&mut conteudo).await.map_err(|erro| {
                ErroDeIngestao::Armazenamento {
                    chave: chave.into(),
                    detalhe: erro.to_string(),
                }
            })?;
            self.objetos.lock().unwrap().insert(chave.into(), conteudo);
            Ok(())
        }

        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.objetos.lock().unwrap().get(chave).cloned())
        }
    }

    fn fixture_ipeadata() -> Vec<u8> {
        let lote = RecordBatch::try_from_iter(vec![
            (
                "SERCODIGO",
                Arc::new(StringArray::from(vec!["ADH_GINI", "ADH_GINI"])) as _,
            ),
            (
                "TERCODIGO",
                Arc::new(StringArray::from(vec!["0", "1100015"])) as _,
            ),
            (
                "VALDATA",
                Arc::new(StringArray::from(vec![
                    "2010-01-01T00:00:00-02:00",
                    "2010-01-01T00:00:00-02:00",
                ])) as _,
            ),
            (
                "NIVNOME",
                Arc::new(StringArray::from(vec!["Brasil", "Municípios"])) as _,
            ),
            (
                "VALVALOR",
                Arc::new(StringArray::from(vec!["0.6086", "0.56"])) as _,
            ),
        ])
        .unwrap();
        let mut bytes = Vec::new();
        let mut writer =
            parquet::arrow::ArrowWriter::try_new(&mut bytes, lote.schema(), None).unwrap();
        writer.write(&lote).unwrap();
        writer.close().unwrap();
        bytes
    }

    async fn snapshot_ipeadata(bronze: &ArmazenamentoEmMemoria) -> SnapshotBronze {
        let recurso = "series/gini";
        let iniciada_em = Utc::now();
        let local = LocalizacaoRaw::nova("ipeadata", recurso, iniciada_em, "bronze-1");
        let bytes = fixture_ipeadata();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "https://www.ipeadata.gov.br/api/odata4/Metadados('ADH_GINI')/Valores".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: TIPO_PARQUET.into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "ipeadata".into(),
            recurso: recurso.into(),
            id_execucao: "bronze-1".into(),
            url_inicial: parte.url.clone(),
            iniciada_em,
            concluida_em: Utc::now(),
            partes: vec![parte.clone()],
        };
        bronze
            .gravar(ObjetoRaw {
                chave: parte.chave,
                conteudo: bytes,
                tipo_de_conteudo: TIPO_PARQUET.into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: local.manifesto(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao("ipeadata", recurso),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some("bronze-1".into()),
                    manifesto: local.manifesto(),
                    iniciada_em: Some(iniciada_em),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        SnapshotBronze::vigente(bronze, "ipeadata", recurso)
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn publica_serie_e_reaproveita() {
        let bronze = ArmazenamentoEmMemoria::default();
        let silver = ArmazenamentoEmMemoria::default();
        let snapshot = Arc::new(snapshot_ipeadata(&bronze).await);

        let tarefa = TarefaDeSilverIpeadata::nova(
            Arc::new(ArmazenamentoEmMemoria::default()),
            Arc::clone(&snapshot),
            "series/gini",
        );
        assert!(tarefa.pendente(&silver).await.unwrap());

        let primeiro = publicar_transformacao_sql(
            &bronze,
            &silver,
            &snapshot,
            ConfiguracaoDePublicacao {
                fonte: "ipeadata",
                recurso: "series/gini",
                recurso_destino: None,
                regra: REGRA,
                esquema: Some(esquema_ipeadata()),
                iniciada_em: Utc::now(),
                id_execucao: "silver-1",
            },
            SQL,
            &ConfiguracaoExecutorSql::default(),
        )
        .await
        .unwrap();
        assert_eq!(primeiro.partes, 1);
        assert!(!tarefa.pendente(&silver).await.unwrap());

        let repetido = publicar_transformacao_sql(
            &bronze,
            &silver,
            &snapshot,
            ConfiguracaoDePublicacao {
                fonte: "ipeadata",
                recurso: "series/gini",
                recurso_destino: None,
                regra: REGRA,
                esquema: Some(esquema_ipeadata()),
                iniciada_em: Utc::now(),
                id_execucao: "silver-2",
            },
            SQL,
            &ConfiguracaoExecutorSql::default(),
        )
        .await
        .unwrap();
        assert!(repetido.reaproveitada);
    }
}
