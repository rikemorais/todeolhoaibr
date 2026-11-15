//! Contrato Silver para o quadro societário da Receita Federal (CNPJ).

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// SQL do contrato `cnpj.socios.v1`.
pub const SQL_CNPJ_SOCIOS: &str = include_str!("../../sql/silver/cnpj_socios.sql");

/// Schema `cnpj.socios.v1`.
pub fn esquema_cnpj_socios() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("cnpj_basico", DataType::Utf8, false),
        Field::new("identificador_socio", DataType::UInt8, true),
        Field::new("nome_socio", DataType::Utf8, true),
        Field::new("cnpj_cpf_socio", DataType::Utf8, true),
        Field::new("qualificacao", DataType::Utf8, true),
        Field::new("data_entrada", DataType::Date32, true),
        Field::new("pais", DataType::Utf8, true),
        Field::new("representante_legal", DataType::Utf8, true),
        Field::new("nome_representante", DataType::Utf8, true),
        Field::new("qualificacao_representante", DataType::Utf8, true),
        Field::new("faixa_etaria", DataType::UInt8, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
