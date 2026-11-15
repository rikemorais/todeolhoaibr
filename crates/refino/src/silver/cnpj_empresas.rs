//! Contrato Silver para o cadastro de empresas da Receita Federal (CNPJ).

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// SQL do contrato `cnpj.empresas.v1`.
pub const SQL_CNPJ_EMPRESAS: &str = include_str!("../../sql/silver/cnpj_empresas.sql");

/// Schema `cnpj.empresas.v1`.
pub fn esquema_cnpj_empresas() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("cnpj_basico", DataType::Utf8, false),
        Field::new("razao_social", DataType::Utf8, false),
        Field::new("natureza_juridica", DataType::Utf8, true),
        Field::new("qualificacao_responsavel", DataType::Utf8, true),
        Field::new("capital_social", DataType::Decimal128(18, 2), true),
        Field::new("porte_empresa", DataType::Utf8, true),
        Field::new("ente_federativo", DataType::Utf8, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
