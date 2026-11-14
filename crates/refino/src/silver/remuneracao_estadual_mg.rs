//! Contrato Silver para remuneração do Executivo estadual de Minas Gerais.
//!
//! O esquema é inferido do Bronze (passthrough) até que um smoke test com dados
//! reais permita fixar colunas e tipos. Não expõe `TarefaDeSilverSql` por
//! exigir `SchemaRef` obrigatório.

use chrono::{DateTime, Utc};
use todeolho_nucleo::{ArmazenamentoRaw, ErroDeIngestao, ResumoDaTarefa};

use super::publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql_com_fonte_bronze};
use crate::SnapshotBronze;

const REGRA: &str = "remuneracao-estadual-mg-folha-v1";
const SQL: &str = include_str!("../../sql/silver/remuneracao_estadual_mg.sql");

/// Publica a transformação Silver passthrough de remuneração MG.
pub async fn publicar_remuneracao_estadual_mg(
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
        "remuneracao-estadual",
        ConfiguracaoDePublicacao {
            fonte: "remuneracao-estadual-mg",
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
