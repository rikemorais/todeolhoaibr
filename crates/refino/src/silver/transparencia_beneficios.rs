//! Contrato Silver para benefícios sociais do Portal da Transparência.

use chrono::{DateTime, Utc};
use todeolho_nucleo::{ArmazenamentoRaw, ErroDeIngestao, ResumoDaTarefa};

use super::publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql_com_fonte_bronze};
use crate::SnapshotBronze;

const REGRA: &str = "transparencia-beneficios-v1";
const SQL: &str = include_str!("../../sql/silver/transparencia_beneficios.sql");

/// Publica a transformação Silver passthrough de benefícios sociais.
pub async fn publicar_transparencia_beneficios(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao_sql_com_fonte_bronze(
        bronze,
        silver,
        snapshot,
        "transparencia",
        ConfiguracaoDePublicacao {
            fonte: "transparencia-beneficios",
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
