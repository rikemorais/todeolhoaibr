//! Contrato Silver para receitas de partidos do TSE em 2016.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

/// Schema versionado `tse.prestacao_contas.receita_partido.2016.v1`.
pub fn esquema_tse_receitas_partidos_2016() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("codigo_eleicao", DataType::Utf8, false),
        Field::new("sequencia_prestador", DataType::Utf8, false),
        Field::new("cnpj_prestador", DataType::Utf8, false),
        Field::new("data_receita", DataType::Date32, true),
        Field::new("valor_receita", DataType::Decimal128(18, 2), false),
        Field::new("tipo_receita", DataType::Utf8, true),
        Field::new("descricao_receita", DataType::Utf8, true),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
