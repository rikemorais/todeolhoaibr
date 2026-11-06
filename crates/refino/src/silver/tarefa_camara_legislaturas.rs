//! Tarefa Silver do catálogo de legislaturas da Câmara.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    camara_legislaturas::{esquema_camara_legislaturas, transformar_camara_legislaturas},
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao},
};
use crate::SnapshotBronze;

const REGRA: &str = "camara-legislatura-periodo-v1";

/// Tarefa Silver para o recurso Bronze `legislaturas` da Câmara.
pub struct TarefaDeSilverCamaraLegislaturas {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
}

impl TarefaDeSilverCamaraLegislaturas {
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
impl Tarefa for TarefaDeSilverCamaraLegislaturas {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_camara_legislaturas(
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

/// Publica o cadastro tipado pelo fluxo compartilhado da Silver.
pub async fn publicar_camara_legislaturas(
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
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_camara_legislaturas()),
            iniciada_em,
            id_execucao,
        },
        |lote| transformar_camara_legislaturas(lote).map_err(|erro| erro.to_string()),
    )
    .await
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use chrono::Utc;
    use sha2::{Digest, Sha256};
    use todeolho_nucleo::{
        ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw, UltimaVersao,
    };
    use tokio::io::{AsyncRead, AsyncReadExt};

    use super::*;

    #[derive(Default)]
    struct ArmazenamentoEmMemoria(std::sync::Mutex<HashMap<String, Vec<u8>>>);

    #[async_trait]
    impl ArmazenamentoRaw for ArmazenamentoEmMemoria {
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

    #[tokio::test]
    async fn publica_e_reaproveita_o_snapshot_vigente() {
        let bronze = ArmazenamentoEmMemoria::default();
        let silver = ArmazenamentoEmMemoria::default();
        let inicio = Utc::now();
        let local = LocalizacaoRaw::nova("camara", "legislaturas", inicio, "bronze-1");
        let lote = arrow::record_batch::RecordBatch::try_from_iter(vec![
            (
                "id",
                Arc::new(arrow::array::StringArray::from(vec!["57"])) as _,
            ),
            (
                "dataInicio",
                Arc::new(arrow::array::StringArray::from(vec!["2023-02-01"])) as _,
            ),
            (
                "dataFim",
                Arc::new(arrow::array::StringArray::from(vec!["2027-01-31"])) as _,
            ),
            (
                "dataEleicao",
                Arc::new(arrow::array::StringArray::from(vec!["2022"])) as _,
            ),
            (
                "uri",
                Arc::new(arrow::array::StringArray::from(vec![
                    "https://example.test/57",
                ])) as _,
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
            url: "https://dadosabertos.camara.leg.br/api/v2/legislaturas".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "camara".into(),
            recurso: "legislaturas".into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "https://dadosabertos.camara.leg.br/api/v2/legislaturas".into(),
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
                chave: LocalizacaoRaw::ultima_versao("camara", "legislaturas"),
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
        let snapshot = SnapshotBronze::vigente(&bronze, "camara", "legislaturas")
            .await
            .unwrap()
            .unwrap();
        let tarefa = TarefaDeSilverCamaraLegislaturas::nova(
            Arc::new(ArmazenamentoEmMemoria::default()),
            Arc::new(snapshot),
            "legislaturas",
        );
        assert!(tarefa.pendente(&silver).await.unwrap());
        let publicada = publicar_camara_legislaturas(
            &bronze,
            &silver,
            &tarefa.snapshot,
            "legislaturas",
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap();
        assert_eq!(publicada.partes, 1);
        assert!(!tarefa.pendente(&silver).await.unwrap());
    }
}
