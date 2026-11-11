//! Contrato Silver de receitas de doador originário do TSE em 2022.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

/// Schema versionado `tse.prestacao_contas.receita_doador_originario.2022.v1`.
pub fn esquema_tse_receitas_doador_originario_2022() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("codigo_eleicao", DataType::Utf8, false),
        Field::new("sequencia_prestador", DataType::Utf8, false),
        Field::new("documento_doador", DataType::Utf8, true),
        Field::new("nome_doador", DataType::Utf8, true),
        Field::new("nome_doador_rfb", DataType::Utf8, true),
        Field::new("tipo_doador", DataType::Utf8, true),
        Field::new("codigo_cnae", DataType::Utf8, true),
        Field::new("descricao_cnae", DataType::Utf8, true),
        Field::new("sequencia_receita", DataType::UInt32, true),
        Field::new("data_receita", DataType::Date32, true),
        Field::new("descricao_receita", DataType::Utf8, true),
        Field::new("valor_receita", DataType::Decimal128(18, 2), false),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
