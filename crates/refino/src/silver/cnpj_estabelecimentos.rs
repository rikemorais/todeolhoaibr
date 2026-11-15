//! Contrato Silver para o cadastro de estabelecimentos da Receita Federal (CNPJ).

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// SQL do contrato `cnpj.estabelecimentos.v1`.
pub const SQL_CNPJ_ESTABELECIMENTOS: &str =
    include_str!("../../sql/silver/cnpj_estabelecimentos.sql");

/// Schema `cnpj.estabelecimentos.v1`.
pub fn esquema_cnpj_estabelecimentos() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("cnpj_basico", DataType::Utf8, false),
        Field::new("cnpj_ordem", DataType::Utf8, false),
        Field::new("cnpj_dv", DataType::Utf8, false),
        Field::new("matriz_filial", DataType::UInt8, true),
        Field::new("nome_fantasia", DataType::Utf8, true),
        Field::new("situacao_cadastral", DataType::Utf8, false),
        Field::new("data_situacao", DataType::Date32, true),
        Field::new("motivo_situacao", DataType::Utf8, true),
        Field::new("cidade_exterior", DataType::Utf8, true),
        Field::new("pais", DataType::Utf8, true),
        Field::new("data_inicio", DataType::Date32, true),
        Field::new("cnae_principal", DataType::Utf8, true),
        Field::new("cnae_secundaria", DataType::Utf8, true),
        Field::new("tipo_logradouro", DataType::Utf8, true),
        Field::new("logradouro", DataType::Utf8, true),
        Field::new("numero", DataType::Utf8, true),
        Field::new("complemento", DataType::Utf8, true),
        Field::new("bairro", DataType::Utf8, true),
        Field::new("cep", DataType::Utf8, true),
        Field::new("uf", DataType::Utf8, true),
        Field::new("municipio", DataType::Utf8, true),
        Field::new("ddd1", DataType::Utf8, true),
        Field::new("telefone1", DataType::Utf8, true),
        Field::new("ddd2", DataType::Utf8, true),
        Field::new("telefone2", DataType::Utf8, true),
        Field::new("ddd_fax", DataType::Utf8, true),
        Field::new("fax", DataType::Utf8, true),
        Field::new("email", DataType::Utf8, true),
        Field::new("situacao_especial", DataType::Utf8, true),
        Field::new("data_situacao_especial", DataType::Date32, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
