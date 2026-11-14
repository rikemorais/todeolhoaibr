//! Tarefa Silver para indicadores World Bank Open Data.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa,
};

use super::{
    publicacao::ConfiguracaoDePublicacao, tarefa_sql::TarefaDeSilverSql,
    world_bank::esquema_world_bank,
};
use crate::SnapshotBronze;

const REGRA: &str = "world-bank-indicador-v1";
const SQL: &str = include_str!("../../sql/silver/world_bank.sql");

/// Tarefa Silver de um indicador World Bank.
pub struct TarefaDeSilverWorldBank {
    tarefa_sql: TarefaDeSilverSql,
}

impl TarefaDeSilverWorldBank {
    /// Cria a tarefa para o recurso e snapshot Bronze selecionados.
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
                "world-bank",
                recurso,
                None,
                REGRA,
                SQL,
                esquema_world_bank(),
            ),
        }
    }

    /// Indica se a versão Silver precisa acompanhar o snapshot Bronze atual.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        self.tarefa_sql.pendente(silver).await
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverWorldBank {
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

/// Publica a transformação World Bank usando o fluxo compartilhado de Silver.
pub async fn publicar_world_bank(
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
            fonte: "world-bank",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_world_bank()),
            iniciada_em,
            id_execucao,
        },
        SQL,
        &Default::default(),
    )
    .await
}
