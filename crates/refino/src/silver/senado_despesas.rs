//! Contrato Silver das despesas da Cota para o Exercício da Atividade Parlamentar dos Senadores.

use std::collections::HashSet;
use std::sync::Arc;

use arrow::array::{
    Array, Date32Array, Decimal128Array, RecordBatch, StringArray, UInt8Array, UInt16Array,
};
use arrow::datatypes::{DataType, Field, Schema};
use chrono::{Datelike, NaiveDate};
use thiserror::Error;

/// Erros ao transformar um lote Bronze das despesas CEAPS.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroSenadoDespesasSilver {
    /// Faltam colunas esperadas ou há coluna de origem que não é textual.
    #[error("schema Bronze de despesas do Senado incompatível")]
    Schema,
    /// Ano, mês ou senador obrigatórios estão ausentes ou inválidos.
    #[error("campo {campo} inválido na linha {linha} do lote")]
    Obrigatorio {
        /// Campo que não pôde ser validado.
        campo: &'static str,
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// Valor monetário preenchido não segue o formato decimal brasileiro.
    #[error("valor monetário inválido na linha {linha} do lote")]
    Valor {
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// O mesmo código de documento aparece mais de uma vez no recurso anual.
    #[error("COD_DOCUMENTO duplicado na linha {linha} do lote")]
    CodigoDuplicado {
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// Os campos de origem não puderam ser serializados em JSON.
    #[error("não foi possível preservar os campos originais da despesa do Senado")]
    Json,
}

/// Schema do contrato `senado.despesa.ceaps.v1`.
pub fn esquema_senado_despesas() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano", DataType::UInt16, false),
        Field::new("mes", DataType::UInt8, true),
        Field::new("senador", DataType::Utf8, false),
        Field::new("tipo_despesa", DataType::Utf8, true),
        Field::new("cnpj_cpf", DataType::Utf8, true),
        Field::new("fornecedor", DataType::Utf8, true),
        Field::new("documento", DataType::Utf8, true),
        Field::new("data", DataType::Date32, true),
        Field::new("detalhamento", DataType::Utf8, true),
        Field::new("valor_reembolsado", DataType::Decimal128(18, 2), true),
        Field::new("cod_documento", DataType::Utf8, true),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

/// Tipifica mês, data e valor; preserva linhas sem código de documento e os campos de origem.
pub fn transformar_senado_despesas(
    lote: &RecordBatch,
    ano: u16,
    codigos_vistos: &mut HashSet<String>,
) -> Result<RecordBatch, ErroSenadoDespesasSilver> {
    if lote
        .schema()
        .fields()
        .iter()
        .any(|campo| campo.data_type() != &DataType::Utf8)
    {
        return Err(ErroSenadoDespesasSilver::Schema);
    }
    let nomes = [
        "ANO",
        "MES",
        "SENADOR",
        "TIPO_DESPESA",
        "CNPJ_CPF",
        "FORNECEDOR",
        "DOCUMENTO",
        "DATA",
        "DETALHAMENTO",
        "VALOR_REEMBOLSADO",
        "COD_DOCUMENTO",
    ];
    let indices = nomes
        .iter()
        .map(|nome| {
            lote.schema()
                .index_of(nome)
                .map_err(|_| ErroSenadoDespesasSilver::Schema)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let colunas = indices
        .iter()
        .map(|indice| {
            lote.column(*indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroSenadoDespesasSilver::Schema)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut meses = Vec::with_capacity(lote.num_rows());
    let mut datas = Vec::with_capacity(lote.num_rows());
    let mut valores = Vec::with_capacity(lote.num_rows());
    let mut originais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        let ano_origem = parse_u16(colunas[0], linha, "ANO")?;
        if ano_origem != ano {
            return Err(ErroSenadoDespesasSilver::Obrigatorio {
                campo: "ANO",
                linha,
            });
        }
        meses.push(parse_mes(colunas[1], linha)?);
        obrigatorio(colunas[2], linha, "SENADOR")?;

        let codigo = texto_opcional(colunas[10], linha);
        if let Some(codigo) = codigo
            && !codigos_vistos.insert(codigo.to_owned())
        {
            return Err(ErroSenadoDespesasSilver::CodigoDuplicado { linha });
        }
        let data_texto = texto_opcional(colunas[7], linha);
        let data = parse_data(data_texto, ano);
        let linha_csv_desalinhada = data_texto.is_some() && data.is_none() && codigo.is_none();
        datas.push(data);
        valores.push(if linha_csv_desalinhada {
            None
        } else {
            parse_decimal(colunas[9], linha)?
        });

        let mut original = serde_json::Map::new();
        for (indice, campo) in lote.schema().fields().iter().enumerate() {
            let coluna = lote
                .column(indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroSenadoDespesasSilver::Schema)?;
            original.insert(
                campo.name().clone(),
                if coluna.is_null(linha) {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(coluna.value(linha).to_owned())
                },
            );
        }
        originais.push(Some(
            serde_json::to_string(&original).map_err(|_| ErroSenadoDespesasSilver::Json)?,
        ));
    }

    let texto = |indice: usize| Arc::new(colunas[indice].clone()) as Arc<dyn Array>;
    let mut saida: Vec<Arc<dyn Array>> = vec![
        Arc::new(UInt16Array::from_value(ano, lote.num_rows())),
        Arc::new(UInt8Array::from(meses)),
        texto(2),
        texto(3),
        texto(4),
        texto(5),
        texto(6),
        Arc::new(Date32Array::from(datas)),
        texto(8),
        Arc::new(decimal_array(valores)?),
        texto(10),
        Arc::new(StringArray::from(originais)),
    ];
    saida.extend(crate::executor_sql::colunas_linhagem_teste(lote.num_rows()));
    RecordBatch::try_new(esquema_senado_despesas(), saida)
        .map_err(|_| ErroSenadoDespesasSilver::Schema)
}

fn texto_opcional(coluna: &StringArray, linha: usize) -> Option<&str> {
    if coluna.is_null(linha) || coluna.value(linha).trim().is_empty() {
        None
    } else {
        Some(coluna.value(linha).trim())
    }
}

fn obrigatorio(
    coluna: &StringArray,
    linha: usize,
    campo: &'static str,
) -> Result<(), ErroSenadoDespesasSilver> {
    texto_opcional(coluna, linha)
        .map(|_| ())
        .ok_or(ErroSenadoDespesasSilver::Obrigatorio { campo, linha })
}

fn parse_u16(
    coluna: &StringArray,
    linha: usize,
    campo: &'static str,
) -> Result<u16, ErroSenadoDespesasSilver> {
    texto_opcional(coluna, linha)
        .and_then(|texto| texto.parse().ok())
        .ok_or(ErroSenadoDespesasSilver::Obrigatorio { campo, linha })
}

fn parse_mes(coluna: &StringArray, linha: usize) -> Result<Option<u8>, ErroSenadoDespesasSilver> {
    let Some(texto) = texto_opcional(coluna, linha) else {
        return Ok(None);
    };
    let mes = texto
        .parse::<u8>()
        .ok()
        .filter(|mes| (1..=12).contains(mes))
        .ok_or(ErroSenadoDespesasSilver::Obrigatorio {
            campo: "MES",
            linha,
        })?;
    Ok(Some(mes))
}

fn parse_data(texto: Option<&str>, ano: u16) -> Option<i32> {
    let texto = texto?;
    let data = NaiveDate::parse_from_str(texto, "%d/%m/%Y").ok()?;
    if data.year() < 1900 || data.year() > i32::from(ano) + 1 {
        return None;
    }
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch válida");
    Some(data.signed_duration_since(epoch).num_days() as i32)
}

fn parse_decimal(
    coluna: &StringArray,
    linha: usize,
) -> Result<Option<i128>, ErroSenadoDespesasSilver> {
    let Some(texto) = texto_opcional(coluna, linha) else {
        return Ok(None);
    };
    decimal_escalado(texto)
        .map(Some)
        .ok_or(ErroSenadoDespesasSilver::Valor { linha })
}

fn decimal_escalado(texto: &str) -> Option<i128> {
    let sem_quebra = texto.replace(['\r', '\n'], "");
    let texto = sem_quebra.trim();
    let (negativo, numero) = texto
        .strip_prefix('-')
        .map_or((false, texto), |resto| (true, resto));
    let (inteiro, fracao) = numero.split_once(',').unwrap_or((numero, ""));
    if fracao.contains(',')
        || fracao.len() > 2
        || (!inteiro.is_empty() && !inteiro.bytes().all(|byte| byte.is_ascii_digit()))
        || !fracao.bytes().all(|byte| byte.is_ascii_digit())
        || (inteiro.is_empty() && fracao.is_empty())
    {
        return None;
    }
    let parte_inteira = if inteiro.is_empty() {
        0
    } else {
        inteiro.parse::<i128>().ok()?
    };
    let centavos = match fracao.len() {
        0 => 0,
        1 => fracao.parse::<i128>().ok()? * 10,
        2 => fracao.parse::<i128>().ok()?,
        _ => return None,
    };
    let escalado = parte_inteira.checked_mul(100)?.checked_add(centavos)?;
    if negativo {
        escalado.checked_neg()
    } else {
        Some(escalado)
    }
}

fn decimal_array(valores: Vec<Option<i128>>) -> Result<Decimal128Array, ErroSenadoDespesasSilver> {
    Decimal128Array::from(valores)
        .with_precision_and_scale(18, 2)
        .map_err(|_| ErroSenadoDespesasSilver::Schema)
}

#[cfg(test)]
mod testes {
    use super::*;
    use arrow::array::Array;

    fn lote(
        codigos: Vec<Option<&str>>,
        datas: Vec<Option<&str>>,
        valores: Vec<Option<&str>>,
    ) -> RecordBatch {
        let tamanho = codigos.len();
        let c =
            |nome, valores: Vec<Option<&str>>| (nome, Arc::new(StringArray::from(valores)) as _);
        RecordBatch::try_from_iter(vec![
            (
                "ANO",
                Arc::new(StringArray::from(vec![Some("2024"); tamanho])) as _,
            ),
            (
                "MES",
                Arc::new(StringArray::from(vec![Some("02"); tamanho])) as _,
            ),
            (
                "SENADOR",
                Arc::new(StringArray::from(vec![Some("SENADORA EXEMPLO"); tamanho])) as _,
            ),
            (
                "TIPO_DESPESA",
                Arc::new(StringArray::from(vec![Some("Passagens"); tamanho])) as _,
            ),
            (
                "CNPJ_CPF",
                Arc::new(StringArray::from(vec![Some("00123456000199"); tamanho])) as _,
            ),
            (
                "FORNECEDOR",
                Arc::new(StringArray::from(vec![Some("Fornecedor"); tamanho])) as _,
            ),
            (
                "DOCUMENTO",
                Arc::new(StringArray::from(vec![Some("000123"); tamanho])) as _,
            ),
            c("DATA", datas),
            (
                "DETALHAMENTO",
                Arc::new(StringArray::from(vec![Some("Trecho"); tamanho])) as _,
            ),
            c("VALOR_REEMBOLSADO", valores),
            c("COD_DOCUMENTO", codigos),
        ])
        .unwrap()
    }

    #[test]
    fn tipa_mes_data_valor_e_preserva_identificadores_e_origem() {
        let saida = transformar_senado_despesas(
            &lote(
                vec![Some("00045")],
                vec![Some("04/02/2024")],
                vec![Some("1\r\n675,5")],
            ),
            2024,
            &mut HashSet::new(),
        )
        .unwrap();
        assert_eq!(saida.schema(), esquema_senado_despesas());
        assert_eq!(
            saida
                .column(1)
                .as_any()
                .downcast_ref::<UInt8Array>()
                .unwrap()
                .value(0),
            2
        );
        assert_eq!(
            saida
                .column(9)
                .as_any()
                .downcast_ref::<Decimal128Array>()
                .unwrap()
                .value(0),
            167_550
        );
        assert_eq!(
            saida
                .column(10)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "00045"
        );
        let original: serde_json::Value = serde_json::from_str(
            saida
                .column(11)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
        )
        .unwrap();
        assert_eq!(original["CNPJ_CPF"], "00123456000199");
        assert_eq!(original["DOCUMENTO"], "000123");
    }

    #[test]
    fn preserva_codigo_ausente_e_linha_com_csv_desalinhado_sem_inferir_valor() {
        let saida = transformar_senado_despesas(
            &lote(
                vec![None],
                vec![Some("descrição deslocada")],
                vec![Some("2014675")],
            ),
            2024,
            &mut HashSet::new(),
        )
        .unwrap();
        assert_eq!(saida.num_rows(), 1);
        assert!(saida.column(7).is_null(0));
        assert!(saida.column(9).is_null(0));
        assert!(saida.column(10).is_null(0));
    }

    #[test]
    fn valida_cod_documento_sem_deduplicar_registro_sem_codigo() {
        let mut vistos = HashSet::new();
        let erro = transformar_senado_despesas(
            &lote(
                vec![Some("001"), Some("001")],
                vec![None, None],
                vec![Some("1,00"), Some("1,00")],
            ),
            2024,
            &mut vistos,
        )
        .unwrap_err();
        assert_eq!(erro, ErroSenadoDespesasSilver::CodigoDuplicado { linha: 1 });

        let saida = transformar_senado_despesas(
            &lote(
                vec![None, None],
                vec![None, None],
                vec![Some("0,95"), Some(",5")],
            ),
            2024,
            &mut HashSet::new(),
        )
        .unwrap();
        assert_eq!(saida.num_rows(), 2);
    }

    #[test]
    fn rejeita_valor_fora_de_escala_e_ano_divergente() {
        assert_eq!(decimal_escalado(",95"), Some(95));
        assert_eq!(decimal_escalado("13800"), Some(1_380_000));
        assert_eq!(decimal_escalado("1,234"), None);
        assert!(matches!(
            transformar_senado_despesas(
                &lote(vec![Some("1")], vec![None], vec![Some("1,00")]),
                2023,
                &mut HashSet::new(),
            ),
            Err(ErroSenadoDespesasSilver::Obrigatorio { campo: "ANO", .. })
        ));
    }

    #[test]
    fn nao_tipifica_data_futura_ao_ano_de_competencia() {
        assert!(parse_data(Some("02/01/2025"), 2024).is_some());
        assert!(parse_data(Some("02/01/2026"), 2024).is_none());
        assert!(parse_data(Some("02/01/1899"), 2024).is_none());
    }
}
