//! Contrato Silver inicial para observações BCB/SGS.

use std::sync::Arc;

#[cfg(test)]
use arrow::array::{Array, Date32Array, Decimal128Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
#[cfg(test)]
use chrono::Datelike;
#[cfg(test)]
use thiserror::Error;

/// Erros de transformação das observações BCB/SGS.
#[cfg(test)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroBcbSilver {
    /// O Bronze não tem as colunas textuais `data` e `valor` esperadas.
    #[error("schema Bronze BCB incompatível: esperadas colunas Utf8 data e valor")]
    Schema,
    /// Uma linha tem data nula ou fora do formato `DD/MM/AAAA`.
    #[error("data inválida na linha {linha} do lote BCB")]
    Data {
        /// Índice da linha dentro do lote, começando em zero.
        linha: usize,
    },
    /// Uma linha tem valor nulo, inválido ou com mais de seis casas decimais.
    #[error("valor inválido na linha {linha} do lote BCB")]
    Valor {
        /// Índice da linha dentro do lote, começando em zero.
        linha: usize,
    },
}

/// Schema versionado do contrato `bcb.sgs.observacao.v1`.
pub fn esquema_bcb_sgs() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("data", DataType::Date32, false),
        Field::new("valor", DataType::Decimal128(18, 6), false),
    ]))
}

/// Transforma um lote Bronze BCB (`data`, `valor`, ambas Utf8) no contrato tipado SGS.
///
/// Datas usam `DD/MM/AAAA`; valores são decimais com ponto e escala máxima de seis
/// casas, sem conversão por ponto flutuante.
#[cfg(test)]
pub fn transformar_bcb_sgs(lote: &RecordBatch) -> Result<RecordBatch, ErroBcbSilver> {
    let schema = lote.schema();
    let (Some(indice_data), Some(indice_valor)) =
        (schema.index_of("data").ok(), schema.index_of("valor").ok())
    else {
        return Err(ErroBcbSilver::Schema);
    };
    if schema.field(indice_data).data_type() != &DataType::Utf8
        || schema.field(indice_valor).data_type() != &DataType::Utf8
    {
        return Err(ErroBcbSilver::Schema);
    }
    let datas = lote
        .column(indice_data)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or(ErroBcbSilver::Schema)?;
    let valores = lote
        .column(indice_valor)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or(ErroBcbSilver::Schema)?;
    let mut dias = Vec::with_capacity(lote.num_rows());
    let mut decimais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        let data = if datas.is_null(linha) {
            return Err(ErroBcbSilver::Data { linha });
        } else {
            chrono::NaiveDate::parse_from_str(datas.value(linha), "%d/%m/%Y")
                .map_err(|_| ErroBcbSilver::Data { linha })?
        };
        dias.push(
            data.num_days_from_ce()
                - chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
                    .unwrap()
                    .num_days_from_ce(),
        );
        if valores.is_null(linha) {
            return Err(ErroBcbSilver::Valor { linha });
        }
        decimais
            .push(decimal_escalado(valores.value(linha)).ok_or(ErroBcbSilver::Valor { linha })?);
    }
    RecordBatch::try_new(
        esquema_bcb_sgs(),
        vec![
            Arc::new(Date32Array::from(dias)),
            Arc::new(
                Decimal128Array::from(decimais)
                    .with_precision_and_scale(18, 6)
                    .map_err(|_| ErroBcbSilver::Schema)?,
            ),
        ],
    )
    .map_err(|_| ErroBcbSilver::Schema)
}

#[cfg(test)]
fn decimal_escalado(valor: &str) -> Option<i128> {
    let (negativo, numero) = valor
        .strip_prefix('-')
        .map_or((false, valor), |v| (true, v));
    let (inteiro, fracao) = numero.split_once('.').unwrap_or((numero, ""));
    if inteiro.is_empty()
        || !inteiro.bytes().all(|b| b.is_ascii_digit())
        || !fracao.bytes().all(|b| b.is_ascii_digit())
        || fracao.len() > 6
        || inteiro.len() > 12
    {
        return None;
    }
    let escala = format!("{fracao:0<6}");
    let magnitude = format!("{inteiro}{escala}").parse::<i128>().ok()?;
    Some(if negativo { -magnitude } else { magnitude })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tipa_data_e_decimal_sem_ponto_flutuante() {
        let bronze = RecordBatch::try_from_iter(vec![
            ("data", Arc::new(StringArray::from(vec!["01/06/1989"])) as _),
            ("valor", Arc::new(StringArray::from(vec!["19.68"])) as _),
        ])
        .unwrap();
        let silver = transformar_bcb_sgs(&bronze).unwrap();
        assert_eq!(silver.schema(), esquema_bcb_sgs());
        assert_eq!(
            silver
                .column(0)
                .as_any()
                .downcast_ref::<Date32Array>()
                .unwrap()
                .value(0),
            chrono::NaiveDate::from_ymd_opt(1989, 6, 1)
                .unwrap()
                .num_days_from_ce()
                - chrono::NaiveDate::from_ymd_opt(1970, 1, 1)
                    .unwrap()
                    .num_days_from_ce()
        );
        assert_eq!(
            silver
                .column(1)
                .as_any()
                .downcast_ref::<Decimal128Array>()
                .unwrap()
                .value(0),
            19_680_000
        );
    }

    #[test]
    fn rejeita_data_invalida_sem_descartar_linha_silenciosamente() {
        let bronze = RecordBatch::try_from_iter(vec![
            ("data", Arc::new(StringArray::from(vec!["31/02/2024"])) as _),
            ("valor", Arc::new(StringArray::from(vec!["1.25"])) as _),
        ])
        .unwrap();
        assert_eq!(
            transformar_bcb_sgs(&bronze),
            Err(ErroBcbSilver::Data { linha: 0 })
        );
    }
}
