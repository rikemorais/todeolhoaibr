//! Contrato Silver para totais de folha municipal do TCE-CE/SIM.

use chrono::{DateTime, Utc};
use todeolho_nucleo::{ArmazenamentoRaw, ErroDeIngestao, ResumoDaTarefa};

use super::publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql};
use crate::SnapshotBronze;

const REGRA: &str = "tce-ce-folhas-pagamentos-v1";
const SQL: &str = include_str!("../../sql/silver/tce_ce.sql");

/// Publica a transformação Silver passthrough do TCE-CE.
pub async fn publicar_remuneracao_municipal_tce_ce(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao_sql(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "remuneracao-municipal-tce-ce",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: None,
            iniciada_em,
            id_execucao,
        },
        SQL,
        &Default::default(),
    )
    .await
}
