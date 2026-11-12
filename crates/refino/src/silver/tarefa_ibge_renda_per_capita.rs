//! Publicação Silver da renda domiciliar per capita (IBGE/Censo via SIDRA).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa,
};

use super::{
    ibge_renda_per_capita::esquema_ibge_renda_per_capita, publicacao::ConfiguracaoDePublicacao,
    tarefa_sql::TarefaDeSilverSql,
};
use crate::SnapshotBronze;

const REGRA: &str = "ibge-renda-per-capita-v1";
const SQL: &str = include_str!("../../sql/silver/ibge_renda_per_capita.sql");

/// Tarefa Silver para um recurso de renda per capita do IBGE/Censo.
pub struct TarefaDeSilverIbgeRendaPerCapita {
    tarefa_sql: TarefaDeSilverSql,
}

impl TarefaDeSilverIbgeRendaPerCapita {
    /// Cria uma tarefa para o recurso e snapshot Bronze selecionados.
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
                "ibge",
                recurso.clone(),
                None,
                REGRA,
                SQL,
                esquema_ibge_renda_per_capita(),
            ),
        }
    }

    /// Indica se a versão Silver precisa acompanhar o snapshot Bronze atual.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        self.tarefa_sql.pendente(silver).await
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverIbgeRendaPerCapita {
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

/// Transforma e publica um recurso IBGE renda per capita a partir do snapshot recebido.
pub async fn publicar_ibge_renda_per_capita(
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
            fonte: "ibge",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_ibge_renda_per_capita()),
            iniciada_em,
            id_execucao,
        },
        SQL,
        &crate::executor_sql::ConfiguracaoExecutorSql::default(),
    )
    .await
}
