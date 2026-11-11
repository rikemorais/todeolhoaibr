//! Contrato Silver para despesas pagas de candidatos do TSE em 2020.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

/// Schema versionado `tse.prestacao_contas.despesa_paga.2020.v1`.
pub fn esquema_tse_despesas_pagas_2020() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("codigo_eleicao", DataType::Utf8, false),
        Field::new("sequencia_prestador", DataType::Utf8, false),
        Field::new("sequencia_despesa", DataType::UInt32, false),
        Field::new("sequencia_pagamento", DataType::UInt32, false),
        Field::new("data_pagamento", DataType::Date32, false),
        Field::new("valor_pago", DataType::Decimal128(18, 2), false),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
