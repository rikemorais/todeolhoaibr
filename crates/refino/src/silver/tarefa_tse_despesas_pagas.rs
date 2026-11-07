//! Publicação Silver do consolidado nacional de despesas pagas do TSE em 2024.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql_com_json_string_original},
    tse_despesas_pagas::esquema_tse_despesas_pagas_2024,
};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

const RECURSO: &str = "prestacao-contas/ano=2024";
const DESTINO: &str = "prestacao-contas/ano=2024/despesas-pagas";
const REGRA: &str = "tse-despesa-paga-2024-v2";
const SQL: &str = include_str!("../../sql/silver/tse_despesas_pagas.sql");

/// Tarefa Silver para despesas pagas do consolidado `BRASIL` de 2024.
pub struct TarefaDeSilverTseDespesasPagas {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
}

impl TarefaDeSilverTseDespesasPagas {
    /// Cria a tarefa quando o recurso é a prestação de contas eleitoral de 2024.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: &str,
    ) -> Option<Self> {
        (recurso == RECURSO).then_some(Self { bronze, snapshot })
    }

    /// Indica se a versão Bronze ou a regra ainda precisa ser publicada.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "tse", DESTINO).await?,
            Some(ref estado)
                if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverTseDespesasPagas {
    fn nome(&self) -> &str {
        DESTINO
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_tse_despesas_pagas_2024(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Publica somente `despesas_pagas_candidatos_2024_BRASIL` em recurso Silver próprio.
pub async fn publicar_tse_despesas_pagas_2024(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    if snapshot.manifesto().fonte != "tse" || snapshot.manifesto().recurso != RECURSO {
        return Err(ErroDeIngestao::Manifesto {
            recurso: snapshot.manifesto().recurso.clone(),
            detalhe: format!("esperado snapshot de {RECURSO}"),
        });
    }
    let parte_brasil = |parte: &todeolho_nucleo::raw::ParteRaw| {
        parte.url.rsplit('/').next().is_some_and(|nome| {
            nome.trim_end_matches(".csv") == "despesas_pagas_candidatos_2024_BRASIL"
        })
    };
    let partes = snapshot
        .manifesto()
        .partes
        .iter()
        .filter(|parte| parte_brasil(parte))
        .count();
    if partes != 1 {
        return Err(ErroDeIngestao::Manifesto {
            recurso: RECURSO.into(),
            detalhe: format!("esperada uma parte BRASIL da tabela, encontradas {partes}"),
        });
    }

    let resultado = publicar_transformacao_sql_com_json_string_original(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "tse",
            recurso: RECURSO,
            recurso_destino: Some(DESTINO),
            regra: REGRA,
            esquema: Some(esquema_tse_despesas_pagas_2024()),
            iniciada_em,
            id_execucao,
        },
        SQL,
        &ConfiguracaoExecutorSql::default(),
    )
    .await;
    resultado
}
