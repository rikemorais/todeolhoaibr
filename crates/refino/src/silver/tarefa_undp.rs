//! Tarefa Silver para índices compostos UNDP/HDR.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa,
};

use super::{
    publicacao::ConfiguracaoDePublicacao, tarefa_sql::TarefaDeSilverSql, undp::esquema_undp,
};
use crate::SnapshotBronze;

const REGRA: &str = "undp-indices-compostos-v1";
const SQL: &str = include_str!("../../sql/silver/undp.sql");

/// Tarefa Silver para o UNDP.
pub struct TarefaDeSilverUndp {
    tarefa_sql: TarefaDeSilverSql,
}

impl TarefaDeSilverUndp {
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
                "undp",
                recurso,
                None,
                REGRA,
                SQL,
                esquema_undp(),
            ),
        }
    }

    /// Indica se a versão Silver precisa acompanhar o snapshot Bronze atual.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        self.tarefa_sql.pendente(silver).await
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverUndp {
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

/// Publica a transformação UNDP usando o fluxo compartilhado de Silver.
pub async fn publicar_undp(
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
            fonte: "undp",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_undp()),
            iniciada_em,
            id_execucao,
        },
        SQL,
        &Default::default(),
    )
    .await
}
