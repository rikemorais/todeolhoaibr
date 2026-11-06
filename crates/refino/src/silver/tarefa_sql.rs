//! Adaptador de tarefa Dagron para contratos Silver SQL/DataFusion.

use std::sync::Arc;

use arrow::datatypes::SchemaRef;
use async_trait::async_trait;
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

use super::publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql};

/// Tarefa reutilizável para executar um contrato SQL Silver sobre um snapshot Bronze.
pub struct TarefaDeSilverSql {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    fonte: String,
    recurso: String,
    recurso_destino: Option<String>,
    regra: String,
    sql: &'static str,
    esquema: SchemaRef,
    executor: ConfiguracaoExecutorSql,
}

impl TarefaDeSilverSql {
    /// Cria uma tarefa SQL com configuração operacional padrão.
    #[allow(clippy::too_many_arguments)]
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        fonte: impl Into<String>,
        recurso: impl Into<String>,
        recurso_destino: Option<String>,
        regra: impl Into<String>,
        sql: &'static str,
        esquema: SchemaRef,
    ) -> Self {
        Self {
            bronze,
            snapshot,
            fonte: fonte.into(),
            recurso: recurso.into(),
            recurso_destino,
            regra: regra.into(),
            sql,
            esquema,
            executor: ConfiguracaoExecutorSql::default(),
        }
    }

    /// Substitui os limites operacionais desta tarefa.
    pub fn com_executor(mut self, executor: ConfiguracaoExecutorSql) -> Self {
        self.executor = executor;
        self
    }

    /// Indica se a tabela publicada ainda precisa acompanhar o snapshot e a regra.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let destino = self.recurso_destino.as_deref().unwrap_or(&self.recurso);
        let versao = format!(
            "{}|regra={}",
            self.snapshot.manifesto().id_execucao,
            self.regra
        );
        Ok(!matches!(
            estado_na_raw(silver, &self.fonte, destino).await?,
            Some(ref estado)
                if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverSql {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_transformacao_sql(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            ConfiguracaoDePublicacao {
                fonte: &self.fonte,
                recurso: &self.recurso,
                recurso_destino: self.recurso_destino.as_deref(),
                regra: &self.regra,
                esquema: Some(self.esquema.clone()),
                iniciada_em: contexto.iniciada_em,
                id_execucao: &contexto.id_execucao,
            },
            self.sql,
            &self.executor,
        )
        .await
    }
}
