use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// Retorna o contrato Arrow dos municípios e das divisões territoriais do IBGE.
pub fn esquema_ibge_municipios() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("nome", DataType::Utf8, true),
        Field::new("microrregiao_id", DataType::Utf8, true),
        Field::new("microrregiao_nome", DataType::Utf8, true),
        Field::new("mesorregiao_id", DataType::Utf8, true),
        Field::new("mesorregiao_nome", DataType::Utf8, true),
        Field::new("uf_id", DataType::Utf8, true),
        Field::new("uf_sigla", DataType::Utf8, true),
        Field::new("regiao_imediata_id", DataType::Utf8, true),
        Field::new("regiao_imediata_nome", DataType::Utf8, true),
        Field::new("regiao_intermediaria_id", DataType::Utf8, true),
        Field::new("regiao_intermediaria_nome", DataType::Utf8, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
