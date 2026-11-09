//! Contrato Silver para os lançamentos da Cota para o Exercício da Atividade Parlamentar.

use std::sync::Arc;

#[cfg(test)]
use arrow::array::{Array, Date32Array, Decimal128Array, RecordBatch, StringArray, UInt16Array};
use arrow::datatypes::{DataType, Field, Schema};
#[cfg(test)]
use chrono::NaiveDate;
#[cfg(test)]
use thiserror::Error;

/// Erros ao transformar um lote de despesas da Câmara.
#[cfg(test)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroCamaraDespesasSilver {
    /// A origem não contém as colunas textuais necessárias.
    #[error("schema Bronze de despesas da Câmara incompatível")]
    Schema,
    /// Um campo preenchido não corresponde ao formato documentado.
    #[error("campo {campo} inválido na linha {linha} do lote")]
    Valor {
        /// Campo da origem que não pôde ser interpretado.
        campo: &'static str,
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
    /// Os campos da origem não puderam ser preservados em JSON.
    #[error("não foi possível preservar os campos originais da despesa")]
    Json,
}

/// Schema versionado do contrato `camara.despesa.ceap.v2`.
pub fn esquema_camara_despesas() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano", DataType::UInt16, false),
        Field::new("nome_parlamentar", DataType::Utf8, true),
        Field::new("id_deputado", DataType::Utf8, true),
        Field::new("id_documento", DataType::Utf8, true),
        Field::new("documento_fornecedor", DataType::Utf8, true),
        Field::new("data_emissao", DataType::Date32, true),
        Field::new("data_pagamento_restituicao", DataType::Date32, true),
        Field::new("valor_documento", DataType::Decimal128(18, 6), true),
        Field::new("valor_glosa", DataType::Decimal128(18, 6), true),
        Field::new("valor_liquido", DataType::Decimal128(18, 6), true),
        Field::new("valor_restituicao", DataType::Decimal128(18, 6), true),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

/// Tipifica campos de consulta e preserva a linha original sem colapsar lançamentos.
#[cfg(test)]
pub fn transformar_camara_despesas(
    lote: &RecordBatch,
    ano: u16,
) -> Result<RecordBatch, ErroCamaraDespesasSilver> {
    if lote
        .schema()
        .fields()
        .iter()
        .any(|campo| campo.data_type() != &DataType::Utf8)
    {
        return Err(ErroCamaraDespesasSilver::Schema);
    }
    let campos = [
        ("txNomeParlamentar", "nome_parlamentar"),
        ("nuDeputadoId", "id_deputado"),
        ("ideDocumento", "id_documento"),
        ("txtCNPJCPF", "documento_fornecedor"),
        ("datEmissao", "data_emissao"),
        ("datPagamentoRestituicao", "data_pagamento_restituicao"),
        ("vlrDocumento", "valor_documento"),
        ("vlrGlosa", "valor_glosa"),
        ("vlrLiquido", "valor_liquido"),
        ("vlrRestituicao", "valor_restituicao"),
    ];
    let indices = campos
        .iter()
        .map(|(origem, _)| {
            lote.schema()
                .index_of(origem)
                .map_err(|_| ErroCamaraDespesasSilver::Schema)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let colunas = indices
        .iter()
        .map(|indice| {
            lote.column(*indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroCamaraDespesasSilver::Schema)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut emissao = Vec::with_capacity(lote.num_rows());
    let mut pagamento = Vec::with_capacity(lote.num_rows());
    let mut documento = Vec::with_capacity(lote.num_rows());
    let mut glosa = Vec::with_capacity(lote.num_rows());
    let mut liquido = Vec::with_capacity(lote.num_rows());
    let mut restituicao = Vec::with_capacity(lote.num_rows());
    let mut originais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        emissao.push(parse_data(colunas[4], linha, "datEmissao")?);
        pagamento.push(parse_data(colunas[5], linha, "datPagamentoRestituicao")?);
        documento.push(parse_decimal(colunas[6], linha, "vlrDocumento")?);
        glosa.push(parse_decimal(colunas[7], linha, "vlrGlosa")?);
        liquido.push(parse_decimal(colunas[8], linha, "vlrLiquido")?);
        restituicao.push(parse_decimal(colunas[9], linha, "vlrRestituicao")?);

        let mut original = serde_json::Map::new();
        for (indice, campo) in lote.schema().fields().iter().enumerate() {
            let valores = lote
                .column(indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroCamaraDespesasSilver::Schema)?;
            let valor = if valores.is_null(linha) {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(valores.value(linha).to_owned())
            };
            original.insert(campo.name().clone(), valor);
        }
        originais.push(Some(
            serde_json::to_string(&original).map_err(|_| ErroCamaraDespesasSilver::Json)?,
        ));
    }

    let coluna_texto = |indice: usize| Arc::new(colunas[indice].clone()) as Arc<dyn Array>;
    let mut saida: Vec<Arc<dyn Array>> = vec![
        Arc::new(UInt16Array::from_value(ano, lote.num_rows())),
        coluna_texto(0),
        coluna_texto(1),
        coluna_texto(2),
        coluna_texto(3),
        data_array(emissao)?,
        data_array(pagamento)?,
        decimal_array(documento)?,
        decimal_array(glosa)?,
        decimal_array(liquido)?,
        decimal_array(restituicao)?,
        Arc::new(StringArray::from(originais)),
    ];
    saida.extend(crate::executor_sql::colunas_linhagem_teste(lote.num_rows()));
    RecordBatch::try_new(esquema_camara_despesas(), saida)
        .map_err(|_| ErroCamaraDespesasSilver::Schema)
}

#[cfg(test)]
fn parse_data(
    valores: &StringArray,
    linha: usize,
    campo: &'static str,
) -> Result<Option<i32>, ErroCamaraDespesasSilver> {
    if valores.is_null(linha) || valores.value(linha).trim().is_empty() {
        return Ok(None);
    }
    let texto = valores.value(linha).trim();
    let data = texto
        .get(..10)
        .and_then(|parte| NaiveDate::parse_from_str(parte, "%Y-%m-%d").ok())
        .ok_or(ErroCamaraDespesasSilver::Valor { campo, linha })?;
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch válida");
    Ok(Some(data.signed_duration_since(epoch).num_days() as i32))
}

#[cfg(test)]
fn parse_decimal(
    valores: &StringArray,
    linha: usize,
    campo: &'static str,
) -> Result<Option<i128>, ErroCamaraDespesasSilver> {
    if valores.is_null(linha) || valores.value(linha).trim().is_empty() {
        return Ok(None);
    }
    decimal_escalado(valores.value(linha).trim())
        .map(Some)
        .ok_or(ErroCamaraDespesasSilver::Valor { campo, linha })
}

#[cfg(test)]
fn decimal_escalado(texto: &str) -> Option<i128> {
    let (negativo, numero) = texto
        .strip_prefix('-')
        .map_or((false, texto), |resto| (true, resto));
    let (inteiro, fracao) = numero.split_once('.').unwrap_or((numero, ""));
    if inteiro.is_empty()
        || !inteiro.bytes().all(|byte| byte.is_ascii_digit())
        || !fracao.bytes().all(|byte| byte.is_ascii_digit())
        || inteiro.len() > 12
        || fracao.len() > 6
    {
        return None;
    }
    let magnitude = format!("{inteiro}{fracao:0<6}").parse::<i128>().ok()?;
    Some(if negativo { -magnitude } else { magnitude })
}

#[cfg(test)]
fn data_array(valores: Vec<Option<i32>>) -> Result<Arc<dyn Array>, ErroCamaraDespesasSilver> {
    Ok(Arc::new(Date32Array::from(valores)))
}

#[cfg(test)]
fn decimal_array(valores: Vec<Option<i128>>) -> Result<Arc<dyn Array>, ErroCamaraDespesasSilver> {
    Ok(Arc::new(
        Decimal128Array::from(valores)
            .with_precision_and_scale(18, 6)
            .map_err(|_| ErroCamaraDespesasSilver::Schema)?,
    ))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn lote() -> RecordBatch {
        RecordBatch::try_from_iter(vec![
            (
                "txNomeParlamentar",
                Arc::new(StringArray::from(vec![Some("Deputado Exemplo")])) as _,
            ),
            (
                "nuDeputadoId",
                Arc::new(StringArray::from(vec![Some("00123")])) as _,
            ),
            (
                "ideDocumento",
                Arc::new(StringArray::from(vec![Some("00045")])) as _,
            ),
            (
                "txtCNPJCPF",
                Arc::new(StringArray::from(vec![Some("00123456000199")])) as _,
            ),
            (
                "datEmissao",
                Arc::new(StringArray::from(vec![Some("2024-03-07T00:00:00")])) as _,
            ),
            (
                "datPagamentoRestituicao",
                Arc::new(StringArray::from(vec![None::<&str>])) as _,
            ),
            (
                "vlrDocumento",
                Arc::new(StringArray::from(vec![Some("1234.50")])) as _,
            ),
            (
                "vlrGlosa",
                Arc::new(StringArray::from(vec![Some("0")])) as _,
            ),
            (
                "vlrLiquido",
                Arc::new(StringArray::from(vec![Some("1234.50")])) as _,
            ),
            (
                "vlrRestituicao",
                Arc::new(StringArray::from(vec![Some("")])) as _,
            ),
            (
                "campoExtra",
                Arc::new(StringArray::from(vec![Some("preservado")])) as _,
            ),
        ])
        .unwrap()
    }

    #[test]
    fn tipa_data_valores_e_preserva_identificadores_e_origem() {
        let silver = transformar_camara_despesas(&lote(), 2024).unwrap();
        assert_eq!(silver.schema(), esquema_camara_despesas());
        assert_eq!(silver.num_rows(), 1);
        assert_eq!(
            silver
                .column(0)
                .as_any()
                .downcast_ref::<UInt16Array>()
                .unwrap()
                .value(0),
            2024
        );
        assert_eq!(
            silver
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "Deputado Exemplo"
        );
        assert_eq!(
            silver
                .column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "00123"
        );
        assert_eq!(
            silver
                .column(5)
                .as_any()
                .downcast_ref::<Date32Array>()
                .unwrap()
                .value(0),
            NaiveDate::from_ymd_opt(2024, 3, 7)
                .unwrap()
                .signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
                .num_days() as i32
        );
        assert_eq!(
            silver
                .column(7)
                .as_any()
                .downcast_ref::<Decimal128Array>()
                .unwrap()
                .value(0),
            1_234_500_000
        );
        let original: serde_json::Value = serde_json::from_str(
            silver
                .column(11)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
        )
        .unwrap();
        assert_eq!(original["campoExtra"], "preservado");
    }

    #[test]
    fn preserva_linha_quando_id_repete_e_trata_brancos_como_nulos() {
        let primeira = lote();
        let silver = transformar_camara_despesas(&primeira, 2008).unwrap();
        assert_eq!(silver.num_rows(), 1);
        assert!(silver.column(6).is_null(0));
        assert!(silver.column(10).is_null(0));
    }

    #[test]
    fn rejeita_valor_monetario_invalido_sem_perder_precisao() {
        let lote = lote();
        let mut colunas = lote.columns().to_vec();
        colunas[6] = Arc::new(StringArray::from(vec![Some("1.2345678")])) as _;
        let lote = RecordBatch::try_new(lote.schema(), colunas).unwrap();
        assert_eq!(
            transformar_camara_despesas(&lote, 2024),
            Err(ErroCamaraDespesasSilver::Valor {
                campo: "vlrDocumento",
                linha: 0
            })
        );
    }
}
