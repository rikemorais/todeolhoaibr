//! Publicação Silver do consolidado nacional de despesas contratadas do TSE em 2020.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql_com_json_string_original},
    tse_despesas_contratadas_2020::esquema_tse_despesas_contratadas_2020,
};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

const RECURSO: &str = "prestacao-contas/ano=2020";
const DESTINO: &str = "prestacao-contas/ano=2020/despesas-contratadas";
const REGRA: &str = "tse-despesa-contratada-2020-v1";
const SQL: &str = include_str!("../../sql/silver/tse_despesas_contratadas_2020.sql");

/// Tarefa Silver para despesas contratadas do consolidado `BRASIL` de 2020.
pub struct TarefaDeSilverTseDespesasContratadas2020 {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
}

impl TarefaDeSilverTseDespesasContratadas2020 {
    /// Cria a tarefa para o recurso de prestação de contas eleitorais de 2020.
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
impl Tarefa for TarefaDeSilverTseDespesasContratadas2020 {
    fn nome(&self) -> &str {
        DESTINO
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_tse_despesas_contratadas_2020(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Publica somente `despesas_contratadas_candidatos_2020_BRASIL`, preservando todas as linhas.
pub async fn publicar_tse_despesas_contratadas_2020(
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
            let nome = nome.to_ascii_lowercase();
            nome.trim_end_matches(".csv").trim_end_matches(".txt")
                == "despesas_contratadas_candidatos_2020_brasil"
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

    let snapshot_brasil = snapshot.filtrar_partes(parte_brasil);
    let executor = ConfiguracaoExecutorSql {
        limite_memoria_bytes: 256 * 1024 * 1024,
        limite_spill_bytes: 8 * 1024 * 1024 * 1024,
        ..ConfiguracaoExecutorSql::default()
    };
    publicar_transformacao_sql_com_json_string_original(
        bronze,
        silver,
        &snapshot_brasil,
        ConfiguracaoDePublicacao {
            fonte: "tse",
            recurso: RECURSO,
            recurso_destino: Some(DESTINO),
            regra: REGRA,
            esquema: Some(esquema_tse_despesas_contratadas_2020()),
            iniciada_em,
            id_execucao,
        },
        SQL,
        &executor,
    )
    .await
}
