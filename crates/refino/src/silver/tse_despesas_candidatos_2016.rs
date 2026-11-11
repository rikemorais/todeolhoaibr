//! Contrato Silver para despesas de candidatos do TSE em 2016.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

/// Schema versionado `tse.prestacao_contas.despesa_candidato.2016.v1`.
pub fn esquema_tse_despesas_candidatos_2016() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("codigo_eleicao", DataType::Utf8, false),
        Field::new("sequencia_candidato", DataType::Utf8, false),
        Field::new("cpf_candidato", DataType::Utf8, false),
        Field::new("data_despesa", DataType::Date32, true),
        Field::new("valor_despesa", DataType::Decimal128(18, 2), false),
        Field::new("tipo_despesa", DataType::Utf8, true),
        Field::new("descricao_despesa", DataType::Utf8, true),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
