//! Contrato Silver para renda domiciliar per capita (IBGE/Censo via SIDRA).

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// Schema versionado `ibge.renda_per_capita.observacao.v1`.
pub fn esquema_ibge_renda_per_capita() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("municipio_id", DataType::Utf8, false),
        Field::new("municipio_nome", DataType::Utf8, true),
        Field::new("variavel_id", DataType::Utf8, true),
        Field::new("variavel_nome", DataType::Utf8, true),
        Field::new("unidade", DataType::Utf8, true),
        Field::new("ano_censo", DataType::Utf8, false),
        Field::new("valor_original", DataType::Utf8, true),
        Field::new("valor_decimal", DataType::Decimal128(38, 9), true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
