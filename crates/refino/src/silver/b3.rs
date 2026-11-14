//! Contrato Silver para cotações históricas B3 COTAHIST.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};

/// Schema versionado `b3.cotahist.v1`.
pub fn esquema_b3() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("data_pregao", DataType::Date32, true),
        Field::new("codigo_negociacao", DataType::Utf8, true),
        Field::new("codigo_bdi", DataType::Int32, true),
        Field::new("tipo_mercado", DataType::Int32, true),
        Field::new("nome_resumido", DataType::Utf8, true),
        Field::new("especificacao", DataType::Utf8, true),
        Field::new("moeda_referencia", DataType::Utf8, true),
        Field::new("preco_abertura", DataType::Decimal128(18, 2), true),
        Field::new("preco_maximo", DataType::Decimal128(18, 2), true),
        Field::new("preco_minimo", DataType::Decimal128(18, 2), true),
        Field::new("preco_medio", DataType::Decimal128(18, 2), true),
        Field::new("preco_ultimo", DataType::Decimal128(18, 2), true),
        Field::new(
            "preco_melhor_oferta_compra",
            DataType::Decimal128(18, 2),
            true,
        ),
        Field::new(
            "preco_melhor_oferta_venda",
            DataType::Decimal128(18, 2),
            true,
        ),
        Field::new("total_negocios", DataType::Int64, true),
        Field::new("quantidade_titulos", DataType::Int64, true),
        Field::new("volume_total", DataType::Decimal128(18, 2), true),
        Field::new("preco_exercicio", DataType::Decimal128(18, 2), true),
        Field::new("indicador_opcao", DataType::Utf8, true),
        Field::new("data_vencimento", DataType::Utf8, true),
        Field::new("fator_cotacao", DataType::Int32, true),
        Field::new("preco_exercicio_pontos", DataType::Decimal128(18, 6), true),
        Field::new("codigo_isin", DataType::Utf8, true),
        Field::new("distribuicao", DataType::Int32, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}
