use std::sync::Arc;

use async_trait::async_trait;
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

use super::ibge_estados::esquema_ibge_estados;
use super::ibge_municipios::esquema_ibge_municipios;
use super::ibge_regioes::esquema_ibge_regioes;

#[derive(Clone, Copy)]
/// Seleciona qual nível de localidade do IBGE será publicado.
pub enum TipoLocalidadeIbge {
    /// Regiões geográficas do Brasil.
    Regioes,
    /// Unidades federativas.
    Estados,
    /// Municípios.
    Municipios,
}

impl TipoLocalidadeIbge {
    fn regra(self) -> &'static str {
        match self {
            Self::Regioes => "ibge-regiao-v1",
            Self::Estados => "ibge-estado-v1",
            Self::Municipios => "ibge-municipio-v1",
        }
    }

    fn sql(self) -> &'static str {
        match self {
            Self::Regioes => include_str!("../../sql/silver/ibge_regioes.sql"),
            Self::Estados => include_str!("../../sql/silver/ibge_estados.sql"),
            Self::Municipios => include_str!("../../sql/silver/ibge_municipios.sql"),
        }
    }

    fn esquema(self) -> arrow::datatypes::SchemaRef {
        match self {
            Self::Regioes => esquema_ibge_regioes(),
            Self::Estados => esquema_ibge_estados(),
            Self::Municipios => esquema_ibge_municipios(),
        }
    }
}

/// Publica um recurso de localidades do IBGE a partir de um snapshot Bronze.
pub struct TarefaDeSilverIbgeLocalidades {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    tipo: TipoLocalidadeIbge,
}

impl TarefaDeSilverIbgeLocalidades {
    /// Cria a tarefa para o recurso e o tipo de localidade indicados.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
        tipo: TipoLocalidadeIbge,
    ) -> Self {
        Self {
            bronze,
            snapshot,
            recurso: recurso.into(),
            tipo,
        }
    }

    /// Informa se o snapshot ainda precisa ser publicado na camada Silver.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!(
            "{}|regra={}",
            self.snapshot.manifesto().id_execucao,
            self.tipo.regra()
        );
        Ok(!matches!(
            estado_na_raw(silver, "ibge", &self.recurso).await?,
            Some(ref estado) if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverIbgeLocalidades {
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
                fonte: "ibge",
                recurso: &self.recurso,
                recurso_destino: None,
                regra: self.tipo.regra(),
                esquema: Some(self.tipo.esquema()),
                iniciada_em: contexto.iniciada_em,
                id_execucao: &contexto.id_execucao,
            },
            self.tipo.sql(),
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

    fn fixture_regioes() -> Vec<u8> {
        let lote = RecordBatch::try_from_iter(vec![
            ("id", Arc::new(StringArray::from(vec!["1", "2"])) as _),
            ("sigla", Arc::new(StringArray::from(vec!["N", "NE"])) as _),
            (
                "nome",
                Arc::new(StringArray::from(vec!["Norte", "Nordeste"])) as _,
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

    async fn snapshot_ibge(
        bronze: &ArmazenamentoEmMemoria,
        recurso: &str,
        parquet: Vec<u8>,
    ) -> SnapshotBronze {
        let iniciada_em = Utc::now();
        let local = LocalizacaoRaw::nova("ibge", recurso, iniciada_em, "bronze-1");
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: format!("https://servicodados.ibge.gov.br/api/v1/localidades/{recurso}"),
            bytes: parquet.len() as u64,
            sha256: hex::encode(Sha256::digest(&parquet)),
            tipo_de_conteudo: TIPO_PARQUET.into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "ibge".into(),
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
                conteudo: parquet,
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
                chave: LocalizacaoRaw::ultima_versao("ibge", recurso),
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
        SnapshotBronze::vigente(bronze, "ibge", recurso)
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn publica_regioes_e_reaproveita() {
        let bronze = ArmazenamentoEmMemoria::default();
        let silver = ArmazenamentoEmMemoria::default();
        let snapshot = Arc::new(snapshot_ibge(&bronze, "regioes", fixture_regioes()).await);

        let tarefa = TarefaDeSilverIbgeLocalidades::nova(
            Arc::new(ArmazenamentoEmMemoria::default()),
            Arc::clone(&snapshot),
            "regioes",
            TipoLocalidadeIbge::Regioes,
        );
        assert!(tarefa.pendente(&silver).await.unwrap());

        let primeiro = publicar_transformacao_sql(
            &bronze,
            &silver,
            &snapshot,
            ConfiguracaoDePublicacao {
                fonte: "ibge",
                recurso: "regioes",
                recurso_destino: None,
                regra: "ibge-regiao-v1",
                esquema: Some(esquema_ibge_regioes()),
                iniciada_em: Utc::now(),
                id_execucao: "silver-1",
            },
            TipoLocalidadeIbge::Regioes.sql(),
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
                fonte: "ibge",
                recurso: "regioes",
                recurso_destino: None,
                regra: "ibge-regiao-v1",
                esquema: Some(esquema_ibge_regioes()),
                iniciada_em: Utc::now(),
                id_execucao: "silver-2",
            },
            TipoLocalidadeIbge::Regioes.sql(),
            &ConfiguracaoExecutorSql::default(),
        )
        .await
        .unwrap();
        assert!(repetido.reaproveitada);
    }
}
