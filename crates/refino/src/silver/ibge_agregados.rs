//! Contrato Silver para linhas dos agregados da API do IBGE.

use std::sync::Arc;

use arrow::array::{Array, Decimal128Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use thiserror::Error;

const COLUNAS_TEXTO: [&str; 10] = [
    "variavel_id",
    "variavel",
    "unidade",
    "classificacoes",
    "nivel_id",
    "nivel_nome",
    "localidade_id",
    "localidade_nome",
    "periodo",
    "valor",
];

/// Erros encontrados ao validar o schema Bronze dos agregados IBGE.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroIbgeSilver {
    /// O lote não contém as colunas textuais esperadas pelo contrato.
    #[error(
        "schema Bronze IBGE incompatível: esperadas dez colunas Utf8 do decodificador de agregados"
    )]
    Schema,
}

/// Schema versionado do contrato `ibge.agregado.observacao.v1`.
///
/// Os identificadores, classificações e períodos permanecem texto. O valor
/// original também é preservado; `valor_decimal` é nulo quando o conteúdo não
/// é um decimal simples com ponto e até nove casas.
pub fn esquema_ibge_agregados() -> Arc<Schema> {
    let mut campos: Vec<Field> = COLUNAS_TEXTO[..9]
        .iter()
        .map(|nome| Field::new(*nome, DataType::Utf8, true))
        .collect();
    campos.push(Field::new("valor_original", DataType::Utf8, true));
    campos.push(Field::new(
        "valor_decimal",
        DataType::Decimal128(38, 9),
        true,
    ));
    Arc::new(Schema::new(campos))
}

/// Converte dimensões textuais do Bronze e adiciona a representação decimal opcional.
///
/// O texto original nunca é descartado. Códigos de situação ou valores fora do
/// formato decimal são mantidos em `valor_original` e recebem decimal nulo;
/// interpretar esses códigos depende de metadados específicos de cada tabela.
pub fn transformar_ibge_agregados(lote: &RecordBatch) -> Result<RecordBatch, ErroIbgeSilver> {
    let schema = lote.schema();
    let indices: Vec<usize> = COLUNAS_TEXTO
        .iter()
        .map(|nome| schema.index_of(nome).map_err(|_| ErroIbgeSilver::Schema))
        .collect::<Result<_, _>>()?;
    if indices
        .iter()
        .any(|indice| schema.field(*indice).data_type() != &DataType::Utf8)
    {
        return Err(ErroIbgeSilver::Schema);
    }

    let arrays: Vec<&StringArray> = indices
        .iter()
        .map(|indice| {
            lote.column(*indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroIbgeSilver::Schema)
        })
        .collect::<Result<_, _>>()?;
    let valores = arrays[9];
    let decimais: Vec<Option<i128>> = (0..lote.num_rows())
        .map(|linha| {
            (!valores.is_null(linha))
                .then(|| decimal_escalado(valores.value(linha)))
                .flatten()
        })
        .collect();

    let mut saida: Vec<Arc<dyn Array>> = arrays[..9]
        .iter()
        .map(|array| Arc::new((*array).clone()) as Arc<dyn Array>)
        .collect();
    saida.push(Arc::new(valores.clone()));
    saida.push(Arc::new(
        Decimal128Array::from(decimais)
            .with_precision_and_scale(38, 9)
            .map_err(|_| ErroIbgeSilver::Schema)?,
    ));
    RecordBatch::try_new(esquema_ibge_agregados(), saida).map_err(|_| ErroIbgeSilver::Schema)
}

fn decimal_escalado(texto: &str) -> Option<i128> {
    let (negativo, numero) = texto
        .strip_prefix('-')
        .map_or((false, texto), |resto| (true, resto));
    let (inteiro, fracao) = numero.split_once('.').unwrap_or((numero, ""));
    if inteiro.is_empty()
        || !inteiro.bytes().all(|byte| byte.is_ascii_digit())
        || !fracao.bytes().all(|byte| byte.is_ascii_digit())
        || inteiro.len() > 29
        || fracao.len() > 9
    {
        return None;
    }
    let escala = format!("{fracao:0<9}");
    let magnitude = format!("{inteiro}{escala}").parse::<i128>().ok()?;
    Some(if negativo { -magnitude } else { magnitude })
}
