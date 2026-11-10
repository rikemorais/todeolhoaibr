use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

pub fn esquema_ipeadata() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("serie", DataType::Utf8, false),
        Field::new("territorio", DataType::Utf8, false),
        Field::new("data", DataType::Date32, true),
        Field::new("nivel", DataType::Utf8, true),
        Field::new("valor", DataType::Decimal128(38, 9), true),
        Field::new("valor_original", DataType::Utf8, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
