//! Tarefa Silver do cadastro de partidos da Câmara.

use std::collections::HashSet;
use std::sync::Arc;

use arrow::array::{Array, StringArray};
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    camara_partidos::{esquema_camara_partidos, transformar_camara_partidos},
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao},
};
use crate::SnapshotBronze;

const REGRA: &str = "camara-partido-v1";

#[derive(Default)]
struct ValidadorDeIdsDePartidos {
    ids_vistos: HashSet<String>,
}

impl ValidadorDeIdsDePartidos {
    fn transformar(&mut self, lote: &RecordBatch) -> Result<RecordBatch, String> {
        let transformado = transformar_camara_partidos(lote).map_err(|erro| erro.to_string())?;
        let ids = transformado
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| "schema Silver de partidos incompatível".to_owned())?;
        for linha in 0..ids.len() {
            if !self.ids_vistos.insert(ids.value(linha).to_owned()) {
                return Err(format!(
                    "identificador de partido duplicado na linha {linha} do lote"
                ));
            }
        }
        Ok(transformado)
    }
}

/// Tarefa Silver para o cadastro de partidos da Câmara.
pub struct TarefaDeSilverCamaraPartidos {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
}

impl TarefaDeSilverCamaraPartidos {
    /// Cria a tarefa para o recurso e snapshot Bronze selecionados.
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

    /// Indica se a Silver ainda precisa acompanhar o snapshot Bronze.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "camara", &self.recurso).await?,
            Some(ref estado) if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverCamaraPartidos {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_camara_partidos(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            &self.recurso,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Transforma e publica o cadastro usando o publicador Silver compartilhado.
pub async fn publicar_camara_partidos(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    let mut validador = ValidadorDeIdsDePartidos::default();
    publicar_transformacao(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "camara",
            recurso,
            regra: REGRA,
            esquema: esquema_camara_partidos(),
            iniciada_em,
            id_execucao,
        },
        |lote| validador.transformar(lote),
    )
    .await
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

    fn fixture_parquet() -> Vec<u8> {
        let lote = RecordBatch::try_from_iter(vec![
            ("id", Arc::new(StringArray::from(vec!["42"])) as _),
            ("sigla", Arc::new(StringArray::from(vec!["ABC"])) as _),
            (
                "nome",
                Arc::new(StringArray::from(vec!["Partido ABC"])) as _,
            ),
            (
                "uri",
                Arc::new(StringArray::from(vec!["https://example.test/42"])) as _,
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

    async fn snapshot_camara(bronze: &ArmazenamentoEmMemoria) -> SnapshotBronze {
        let recurso = "partidos";
        let iniciada_em = Utc::now();
        let local = LocalizacaoRaw::nova("camara", recurso, iniciada_em, "bronze-1");
        let bytes = fixture_parquet();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "https://dadosabertos.camara.leg.br/api/v2/partidos".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: TIPO_PARQUET.into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "camara".into(),
            recurso: recurso.into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "https://dadosabertos.camara.leg.br/api/v2/partidos".into(),
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
                chave: LocalizacaoRaw::ultima_versao("camara", recurso),
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
        SnapshotBronze::vigente(bronze, "camara", recurso)
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn publica_cadastro_e_reaproveita_a_mesma_versao() {
        let bronze = ArmazenamentoEmMemoria::default();
        let silver = ArmazenamentoEmMemoria::default();
        let snapshot = Arc::new(snapshot_camara(&bronze).await);
        let tarefa = TarefaDeSilverCamaraPartidos::nova(
            Arc::new(ArmazenamentoEmMemoria::default()),
            Arc::clone(&snapshot),
            "partidos",
        );
        assert_eq!(tarefa.nome(), "partidos");
        assert!(tarefa.pendente(&silver).await.unwrap());
        let primeiro = publicar_camara_partidos(
            &bronze,
            &silver,
            &snapshot,
            "partidos",
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap();
        assert_eq!(primeiro.partes, 1);
        assert!(!tarefa.pendente(&silver).await.unwrap());
        let repetido = publicar_camara_partidos(
            &bronze,
            &silver,
            &snapshot,
            "partidos",
            Utc::now(),
            "silver-2",
        )
        .await
        .unwrap();
        assert!(repetido.reaproveitada);
    }

    #[test]
    fn rejeita_id_repetido_entre_lotes_do_mesmo_recurso() {
        let lote = || {
            RecordBatch::try_from_iter(vec![
                ("id", Arc::new(StringArray::from(vec!["42"])) as _),
                ("sigla", Arc::new(StringArray::from(vec!["ABC"])) as _),
                (
                    "nome",
                    Arc::new(StringArray::from(vec!["Partido ABC"])) as _,
                ),
                (
                    "uri",
                    Arc::new(StringArray::from(vec!["https://example.test/42"])) as _,
                ),
            ])
            .unwrap()
        };
        let mut validador = ValidadorDeIdsDePartidos::default();
        assert!(validador.transformar(&lote()).is_ok());
        assert!(
            validador
                .transformar(&lote())
                .unwrap_err()
                .contains("duplicado")
        );
    }
}
