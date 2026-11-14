//! Tarefa Silver para cotações históricas B3 COTAHIST.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa,
};

use super::{b3::esquema_b3, publicacao::ConfiguracaoDePublicacao, tarefa_sql::TarefaDeSilverSql};
use crate::SnapshotBronze;

const REGRA: &str = "b3-cotahist-v1";
const SQL: &str = include_str!("../../sql/silver/b3.sql");

/// Tarefa Silver de um ano COTAHIST.
pub struct TarefaDeSilverB3 {
    tarefa_sql: TarefaDeSilverSql,
}

impl TarefaDeSilverB3 {
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
                "b3",
                recurso,
                None,
                REGRA,
                SQL,
                esquema_b3(),
            ),
        }
    }

    /// Indica se a versão Silver precisa acompanhar o snapshot Bronze atual.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        self.tarefa_sql.pendente(silver).await
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverB3 {
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

/// Publica a transformação B3 usando o fluxo compartilhado de Silver.
pub async fn publicar_b3(
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
            fonte: "b3",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_b3()),
            iniciada_em,
            id_execucao,
        },
        SQL,
        &Default::default(),
    )
    .await
}
