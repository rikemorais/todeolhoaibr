//! Contrato Silver para índices compostos UNDP/HDR.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

/// Schema versionado `undp.indices-compostos.v1`.
pub fn esquema_undp() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("iso3", DataType::Utf8, true),
        Field::new("pais", DataType::Utf8, true),
        Field::new("codigo_hdi", DataType::Utf8, true),
        Field::new("regiao", DataType::Utf8, true),
        Field::new("ranking_hdi_2023", DataType::Int32, true),
        Field::new("hdi_2023", DataType::Decimal128(10, 4), true),
        Field::new("expectativa_vida_2023", DataType::Decimal128(10, 2), true),
        Field::new(
            "anos_escolaridade_esperados_2023",
            DataType::Decimal128(10, 2),
            true,
        ),
        Field::new(
            "anos_escolaridade_media_2023",
            DataType::Decimal128(10, 2),
            true,
        ),
        Field::new("renda_per_capita_2023", DataType::Decimal128(18, 2), true),
        Field::new("gdi_2023", DataType::Decimal128(10, 4), true),
        Field::new("gii_2023", DataType::Decimal128(10, 4), true),
        Field::new("phdi_2023", DataType::Decimal128(10, 4), true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
