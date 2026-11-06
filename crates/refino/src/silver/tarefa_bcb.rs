//! Tarefa e adaptador do contrato BCB/SGS para o publicador Silver compartilhado.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    bcb::esquema_bcb_sgs,
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao},
};
use crate::SnapshotBronze;

const REGRA: &str = "bcb-sgs-v1";

/// Tarefa Silver de uma série BCB, utilizável pelo executor e pelo painel.
pub struct TarefaDeSilverBcb {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
}

impl TarefaDeSilverBcb {
    /// Cria a tarefa para um snapshot já selecionado e verificado.
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

    /// Indica se a versão Silver precisa acompanhar o snapshot Bronze atual.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "bcb", &self.recurso).await?,
            Some(ref estado)
                if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverBcb {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_bcb_sgs(
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

/// Publica a transformação BCB/SGS usando o fluxo compartilhado de Silver.
pub async fn publicar_bcb_sgs(
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
            fonte: "bcb",
            recurso,
            regra: REGRA,
            esquema: esquema_bcb_sgs(),
            iniciada_em,
            id_execucao,
        },
        |lote| super::bcb::transformar_bcb_sgs(lote).map_err(|erro| erro.to_string()),
    )
    .await
}
