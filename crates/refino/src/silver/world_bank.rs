//! Contrato Silver para indicadores World Bank Open Data.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

/// Schema versionado `world-bank.indicador.v1`.
pub fn esquema_world_bank() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("codigo_indicador", DataType::Utf8, true),
        Field::new("nome_indicador", DataType::Utf8, true),
        Field::new("codigo_pais", DataType::Utf8, true),
        Field::new("nome_pais", DataType::Utf8, true),
        Field::new("iso3", DataType::Utf8, true),
        Field::new("ano", DataType::Int32, true),
        Field::new("valor", DataType::Decimal128(18, 6), true),
        Field::new("unidade", DataType::Utf8, true),
        Field::new("status_observacao", DataType::Utf8, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
