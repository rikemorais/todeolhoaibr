//! Contrato Silver para despesas contratadas de candidatos do TSE em 2022.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

/// Schema versionado `tse.prestacao_contas.despesa_contratada.2022.v1`.
pub fn esquema_tse_despesas_contratadas_2022() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("codigo_eleicao", DataType::Utf8, false),
        Field::new("sequencia_prestador", DataType::Utf8, false),
        Field::new("sequencia_despesa", DataType::UInt32, true),
        Field::new("data_despesa", DataType::Date32, true),
        Field::new("descricao_despesa", DataType::Utf8, true),
        Field::new("valor_contratado", DataType::Decimal128(18, 2), false),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
