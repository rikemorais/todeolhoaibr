//! Tarefa e adaptador do contrato BCB/SGS para o publicador Silver compartilhado.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa,
};

use super::{
    bcb::esquema_bcb_sgs, publicacao::ConfiguracaoDePublicacao, tarefa_sql::TarefaDeSilverSql,
};
use crate::SnapshotBronze;

const REGRA: &str = "bcb-sgs-v2";
const SQL: &str = include_str!("../../sql/silver/bcb_sgs.sql");

/// Tarefa Silver de uma série BCB, utilizável pelo executor e pelo painel.
pub struct TarefaDeSilverBcb {
    tarefa_sql: TarefaDeSilverSql,
}

impl TarefaDeSilverBcb {
    /// Cria a tarefa para um snapshot já selecionado e verificado.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Self {
        let recurso = recurso.into();
        Self {
            tarefa_sql: TarefaDeSilverSql::nova(
                bronze,
                snapshot,
                "bcb",
                recurso.clone(),
                None,
                REGRA,
                SQL,
                esquema_bcb_sgs(),
            ),
        }
    }

    /// Indica se a versão Silver precisa acompanhar o snapshot Bronze atual.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        self.tarefa_sql.pendente(silver).await
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverBcb {
    fn nome(&self) -> &str {
        self.tarefa_sql.nome()
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        self.tarefa_sql.executar(contexto).await
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
    super::publicacao::publicar_transformacao_sql(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "bcb",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_bcb_sgs()),
            iniciada_em,
            id_execucao,
        },
        SQL,
        &crate::executor_sql::ConfiguracaoExecutorSql::default(),
    )
    .await
}
