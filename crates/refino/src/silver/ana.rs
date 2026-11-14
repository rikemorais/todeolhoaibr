//! Contrato Silver para estações hidrológicas ANA/SNIRH.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

/// Schema versionado `ana.estacao.v1`.
pub fn esquema_ana() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("codigo_estacao", DataType::Int64, true),
        Field::new("nome_estacao", DataType::Utf8, true),
        Field::new("tipo_estacao", DataType::Int32, true),
        Field::new("descricao", DataType::Utf8, true),
        Field::new("codigo_bacia", DataType::Int32, true),
        Field::new("nome_bacia", DataType::Utf8, true),
        Field::new("codigo_sub_bacia", DataType::Int32, true),
        Field::new("nome_sub_bacia", DataType::Utf8, true),
        Field::new("codigo_rio", DataType::Int32, true),
        Field::new("nome_rio", DataType::Utf8, true),
        Field::new("codigo_estado", DataType::Int32, true),
        Field::new("nome_estado", DataType::Utf8, true),
        Field::new("codigo_municipio", DataType::Int32, true),
        Field::new("nome_municipio", DataType::Utf8, true),
        Field::new("codigo_responsavel", DataType::Int32, true),
        Field::new("responsavel_sigla", DataType::Utf8, true),
        Field::new("responsavel_unidade", DataType::Utf8, true),
        Field::new("responsavel_jurisdicao", DataType::Utf8, true),
        Field::new("codigo_operadora", DataType::Int32, true),
        Field::new("operadora_sigla", DataType::Utf8, true),
        Field::new("operadora_unidade", DataType::Utf8, true),
        Field::new("operadora_sub_unidade", DataType::Utf8, true),
        Field::new("latitude", DataType::Float64, true),
        Field::new("longitude", DataType::Float64, true),
        Field::new("altitude", DataType::Float64, true),
        Field::new("area_drenagem", DataType::Float64, true),
        Field::new("operando", DataType::Int32, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
