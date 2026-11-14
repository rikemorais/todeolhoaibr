//! Tarefa Silver para estações hidrológicas ANA/SNIRH.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa,
};

use super::{
    ana::esquema_ana, publicacao::ConfiguracaoDePublicacao, tarefa_sql::TarefaDeSilverSql,
};
use crate::SnapshotBronze;

const REGRA: &str = "ana-estacao-v1";
const SQL: &str = include_str!("../../sql/silver/ana.sql");

/// Tarefa Silver de um recurso ANA (fluviométricas ou pluviométricas).
pub struct TarefaDeSilverAna {
    tarefa_sql: TarefaDeSilverSql,
}

impl TarefaDeSilverAna {
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
                "ana",
                recurso,
                None,
                REGRA,
                SQL,
                esquema_ana(),
            ),
        }
    }

    /// Indica se a versão Silver precisa acompanhar o snapshot Bronze atual.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        self.tarefa_sql.pendente(silver).await
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverAna {
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

/// Publica a transformação ANA usando o fluxo compartilhado de Silver.
pub async fn publicar_ana(
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
            fonte: "ana",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_ana()),
            iniciada_em,
            id_execucao,
        },
        SQL,
        &Default::default(),
    )
    .await
}
