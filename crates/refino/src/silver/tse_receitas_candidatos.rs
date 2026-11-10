//! Contrato Silver para receitas de candidatos do TSE em 2024.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

/// Schema versionado `tse.prestacao_contas.receita_candidato.2024.v1`.
pub fn esquema_tse_receitas_candidatos_2024() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("codigo_eleicao", DataType::Utf8, false),
        Field::new("sequencia_prestador", DataType::Utf8, false),
        Field::new("sequencia_receita", DataType::UInt32, true),
        Field::new("data_receita", DataType::Date32, false),
        Field::new("valor_receita", DataType::Decimal128(18, 2), false),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
