//! Tarefa Silver do cadastro atual de deputados da Câmara.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    camara_deputados::{esquema_camara_deputados, transformar_camara_deputados},
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao},
};
use crate::SnapshotBronze;

const REGRA: &str = "camara-deputado-exercicio-v1";

/// Tarefa Silver para o recurso Bronze `deputados`.
pub struct TarefaDeSilverCamaraDeputados {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
}

impl TarefaDeSilverCamaraDeputados {
    /// Cria a tarefa para o recurso e snapshot selecionados.
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
        Ok(
            !matches!(estado_na_raw(silver, "camara", &self.recurso).await?,
            Some(ref estado) if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())),
        )
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverCamaraDeputados {
    fn nome(&self) -> &str {
        &self.recurso
    }
    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_camara_deputados(
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

/// Publica a fotografia vigente do cadastro de deputados.
pub async fn publicar_camara_deputados(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "camara",
            recurso,
            regra: REGRA,
            esquema: Some(esquema_camara_deputados()),
            iniciada_em,
            id_execucao,
        },
        |lote| transformar_camara_deputados(lote).map_err(|erro| erro.to_string()),
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
            _tipo: &str,
            fluxo: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            let mut dados = Vec::new();
            fluxo
                .read_to_end(&mut dados)
                .await
                .map_err(|erro| ErroDeIngestao::Armazenamento {
                    chave: chave.into(),
                    detalhe: erro.to_string(),
                })?;
            self.0.lock().unwrap().insert(chave.into(), dados);
            Ok(())
        }
        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.0.lock().unwrap().get(chave).cloned())
        }
    }

    fn parquet() -> Vec<u8> {
        let lote = RecordBatch::try_from_iter(vec![
            ("id", Arc::new(StringArray::from(vec!["12345"])) as _),
            (
                "uri",
                Arc::new(StringArray::from(vec![
                    "https://example.test/deputados/12345",
                ])) as _,
            ),
            (
                "nome",
                Arc::new(StringArray::from(vec!["Deputada Exemplo"])) as _,
            ),
            (
                "siglaPartido",
                Arc::new(StringArray::from(vec!["ABC"])) as _,
            ),
            (
                "uriPartido",
                Arc::new(StringArray::from(vec!["https://example.test/partidos/1"])) as _,
            ),
            ("siglaUf", Arc::new(StringArray::from(vec!["DF"])) as _),
            (
                "idLegislatura",
                Arc::new(StringArray::from(vec!["57"])) as _,
            ),
            (
                "urlFoto",
                Arc::new(StringArray::from(vec![None::<&str>])) as _,
            ),
            (
                "email",
                Arc::new(StringArray::from(vec![Some("deputada@camara.leg.br")])) as _,
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

    #[tokio::test]
    async fn publica_e_reutiliza_a_versao_bronze() {
        let bronze = Memoria::default();
        let silver = Memoria::default();
        let iniciado = Utc::now();
        let local = LocalizacaoRaw::nova("camara", "deputados", iniciado, "bronze-1");
        let bytes = parquet();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "https://dadosabertos.camara.leg.br/api/v2/deputados".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "camara".into(),
            recurso: "deputados".into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "https://dadosabertos.camara.leg.br/api/v2/deputados".into(),
            iniciada_em: iniciado,
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
                chave: LocalizacaoRaw::ultima_versao("camara", "deputados"),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some("bronze-1".into()),
                    manifesto: local.manifesto(),
                    iniciada_em: Some(iniciado),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            },
        ] {
            bronze.gravar(objeto).await.unwrap();
        }
        let snapshot = SnapshotBronze::vigente(&bronze, "camara", "deputados")
            .await
            .unwrap()
            .unwrap();
        let tarefa = TarefaDeSilverCamaraDeputados::nova(
            Arc::new(Memoria::default()),
            Arc::new(snapshot),
            "deputados",
        );
        assert_eq!(tarefa.nome(), "deputados");
        assert!(tarefa.pendente(&silver).await.unwrap());
        let primeiro = publicar_camara_deputados(
            &bronze,
            &silver,
            &tarefa.snapshot,
            "deputados",
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap();
        assert_eq!(primeiro.partes, 1);
        assert!(!tarefa.pendente(&silver).await.unwrap());
    }
}
