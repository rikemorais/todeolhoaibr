//! Tarefa Silver de parlamentares do Senado por legislatura.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao},
    senado_parlamentares::{esquema_senado_parlamentares, transformar_senado_parlamentares},
};
use crate::SnapshotBronze;

const REGRA: &str = "senado-parlamentar-v1";

/// Tarefa Silver para a listagem de uma legislatura do Senado.
pub struct TarefaDeSilverSenadoParlamentares {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    legislatura: u16,
}

impl TarefaDeSilverSenadoParlamentares {
    /// Cria a tarefa a partir do snapshot Bronze e da legislatura do recurso.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
        legislatura: u16,
    ) -> Self {
        Self {
            bronze,
            snapshot,
            recurso: recurso.into(),
            legislatura,
        }
    }

    /// Indica se a versão Silver está pendente para o snapshot Bronze recebido.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "senado", &self.recurso).await?,
            Some(ref estado)
                if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverSenadoParlamentares {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_senado_parlamentares(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            &self.recurso,
            self.legislatura,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Publica a transformação de parlamentares usando o fluxo Silver compartilhado.
pub async fn publicar_senado_parlamentares(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    legislatura: u16,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "senado",
            recurso,
            regra: REGRA,
            esquema: esquema_senado_parlamentares(),
            iniciada_em,
            id_execucao,
        },
        |lote| transformar_senado_parlamentares(lote, legislatura).map_err(|erro| erro.to_string()),
    )
    .await
}
