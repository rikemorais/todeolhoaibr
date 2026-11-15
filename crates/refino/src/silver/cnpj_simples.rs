//! Contrato Silver para opção pelo Simples/MEI da Receita Federal (CNPJ).

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// SQL do contrato `cnpj.simples.v1`.
pub const SQL_CNPJ_SIMPLES: &str = include_str!("../../sql/silver/cnpj_simples.sql");

/// Schema `cnpj.simples.v1`.
pub fn esquema_cnpj_simples() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("cnpj_basico", DataType::Utf8, false),
        Field::new("opcao_simples", DataType::Utf8, true),
        Field::new("data_opcao_simples", DataType::Date32, true),
        Field::new("data_exclusao_simples", DataType::Date32, true),
        Field::new("opcao_mei", DataType::Utf8, true),
        Field::new("data_opcao_mei", DataType::Date32, true),
        Field::new("data_exclusao_mei", DataType::Date32, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
