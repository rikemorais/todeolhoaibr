use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// Retorna o contrato Arrow das unidades federativas e suas regiões.
pub fn esquema_ibge_estados() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("sigla", DataType::Utf8, true),
        Field::new("nome", DataType::Utf8, true),
        Field::new("regiao_id", DataType::Utf8, true),
        Field::new("regiao_sigla", DataType::Utf8, true),
        Field::new("regiao_nome", DataType::Utf8, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
