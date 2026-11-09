//! Contrato Silver dos bens declarados pelos candidatos do TSE.

use std::collections::HashSet;
use std::sync::Arc;

use arrow::array::{
    Array, Date32Array, Decimal128Array, RecordBatch, StringArray, UInt16Array, UInt32Array,
};
use arrow::datatypes::{DataType, Field, Schema};
use chrono::{Datelike, NaiveDate};
use thiserror::Error;

/// Erros ao transformar bens declarados.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroTseBensSilver {
    /// O Parquet Bronze não tem os campos textuais esperados.
    #[error("schema Bronze de bens do TSE incompatível")]
    Schema,
    /// Um campo obrigatório da chave está ausente ou inválido.
    #[error("campo {campo} inválido na linha {linha} do lote")]
    Obrigatorio {
        /// Campo que não pôde ser validado.
        campo: &'static str,
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// O ano registrado na linha diverge do ano do recurso.
    #[error("ANO_ELEICAO diverge do recurso na linha {linha} do lote")]
    AnoDivergente {
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// O valor monetário preenchido não segue o formato decimal brasileiro.
    #[error("VR_BEM_CANDIDATO inválido na linha {linha} do lote")]
    Valor {
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// A chave candidata completa repete um bem no mesmo consolidado.
    #[error("chave de bem duplicada na linha {linha} do lote")]
    ChaveDuplicada {
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// Os campos de origem não puderam ser preservados como JSON.
    #[error("não foi possível preservar os campos originais do bem declarado")]
    Json,
}

/// Schema do contrato `tse.candidatura.bem.v1`.
pub fn esquema_tse_bens() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("codigo_eleicao", DataType::Utf8, false),
        Field::new("uf", DataType::Utf8, false),
        Field::new("unidade_eleitoral", DataType::Utf8, false),
        Field::new("sequencia_candidato", DataType::Utf8, false),
        Field::new("ordem_bem", DataType::UInt32, false),
        Field::new("data_eleicao", DataType::Date32, true),
        Field::new("data_ultima_atualizacao", DataType::Date32, true),
        Field::new("codigo_tipo_bem", DataType::Utf8, true),
        Field::new("tipo_bem", DataType::Utf8, true),
        Field::new("descricao_bem", DataType::Utf8, true),
        Field::new("valor_bem", DataType::Decimal128(18, 2), true),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

/// Normaliza a chave do bem, tipa ordem, datas e valor e preserva os campos de origem.
pub fn transformar_tse_bens(
    lote: &RecordBatch,
    ano: u16,
    chaves_vistas: &mut HashSet<String>,
) -> Result<RecordBatch, ErroTseBensSilver> {
    if lote
        .schema()
        .fields()
        .iter()
        .any(|campo| campo.data_type() != &DataType::Utf8)
    {
        return Err(ErroTseBensSilver::Schema);
    }
    let nomes = [
        "ANO_ELEICAO",
        "CD_ELEICAO",
        "SG_UF",
        "SG_UE",
        "SQ_CANDIDATO",
    ];
    let indices = nomes
        .iter()
        .map(|nome| {
            lote.schema()
                .index_of(nome)
                .map_err(|_| ErroTseBensSilver::Schema)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let colunas = indices
        .iter()
        .map(|indice| {
            lote.column(*indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroTseBensSilver::Schema)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let coluna_ordem = coluna_obrigatoria(lote, &["NR_ORDEM_CANDIDATO", "NR_ORDEM_BEM_CANDIDATO"])?;
    let data_eleicao = coluna_opcional(lote, &["DT_ELEICAO"])?;
    let data_atualizacao = coluna_opcional(
        lote,
        &["DT_ULTIMA_ATUALIZACAO", "DT_ULT_ATUAL_BEM_CANDIDATO"],
    )?;
    let codigo_tipo = coluna_opcional(lote, &["CD_TIPO_BEM_CANDIDATO"])?;
    let tipo = coluna_opcional(lote, &["DS_TIPO_BEM_CANDIDATO"])?;
    let descricao = coluna_opcional(lote, &["DS_BEM_CANDIDATO"])?;
    let valor = coluna_opcional(lote, &["VR_BEM_CANDIDATO"])?;

    let mut ordens = Vec::with_capacity(lote.num_rows());
    let mut datas_eleicao = Vec::with_capacity(lote.num_rows());
    let mut datas_atualizacao = Vec::with_capacity(lote.num_rows());
    let mut valores = Vec::with_capacity(lote.num_rows());
    let mut originais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        let ano_linha = texto_obrigatorio(colunas[0], linha, "ANO_ELEICAO")?;
        if ano_linha.parse::<u16>().ok() != Some(ano) {
            return Err(ErroTseBensSilver::AnoDivergente { linha });
        }
        let mut componentes = vec![ano.to_string()];
        for (coluna, campo) in colunas[1..].iter().zip(&nomes[1..]) {
            componentes.push(texto_obrigatorio(coluna, linha, campo)?.to_owned());
        }
        let texto_ordem = texto_obrigatorio(coluna_ordem, linha, "NR_ORDEM_BEM")?;
        let ordem = texto_ordem
            .parse::<u32>()
            .map_err(|_| ErroTseBensSilver::Obrigatorio {
                campo: "NR_ORDEM_BEM",
                linha,
            })?;
        componentes.push(ordem.to_string());
        if !chaves_vistas.insert(componentes.join("\0")) {
            return Err(ErroTseBensSilver::ChaveDuplicada { linha });
        }
        ordens.push(ordem);
        datas_eleicao.push(parse_data(data_eleicao, linha, ano));
        datas_atualizacao.push(parse_data(data_atualizacao, linha, ano));
        valores.push(parse_decimal(valor, linha)?);
        originais.push(Some(registro_original(lote, linha)?));
    }

    let texto_preenchido = |coluna: Option<&StringArray>| -> Arc<dyn Array> {
        if let Some(coluna) = coluna {
            Arc::new(coluna.clone())
        } else {
            Arc::new(StringArray::from(vec![None::<&str>; lote.num_rows()]))
        }
    };
    let mut cols: Vec<Arc<dyn Array>> = vec![
        Arc::new(UInt16Array::from_value(ano, lote.num_rows())),
        Arc::new(colunas[1].clone()),
        Arc::new(colunas[2].clone()),
        Arc::new(colunas[3].clone()),
        Arc::new(colunas[4].clone()),
        Arc::new(UInt32Array::from(ordens)),
        Arc::new(Date32Array::from(datas_eleicao)),
        Arc::new(Date32Array::from(datas_atualizacao)),
        texto_preenchido(codigo_tipo),
        texto_preenchido(tipo),
        texto_preenchido(descricao),
        Arc::new(decimal_array(valores)?),
        Arc::new(StringArray::from(originais)),
    ];
    cols.extend(crate::executor_sql::colunas_linhagem_teste(lote.num_rows()));
    RecordBatch::try_new(esquema_tse_bens(), cols).map_err(|_| ErroTseBensSilver::Schema)
}

fn coluna_obrigatoria<'a>(
    lote: &'a RecordBatch,
    nomes: &[&str],
) -> Result<&'a StringArray, ErroTseBensSilver> {
    coluna_opcional(lote, nomes)?.ok_or(ErroTseBensSilver::Schema)
}

fn coluna_opcional<'a>(
    lote: &'a RecordBatch,
    nomes: &[&str],
) -> Result<Option<&'a StringArray>, ErroTseBensSilver> {
    let Some(indice) = nomes
        .iter()
        .find_map(|nome| lote.schema().index_of(nome).ok())
    else {
        return Ok(None);
    };
    lote.column(indice)
        .as_any()
        .downcast_ref::<StringArray>()
        .map(Some)
        .ok_or(ErroTseBensSilver::Schema)
}

fn texto_obrigatorio<'a>(
    coluna: &'a StringArray,
    linha: usize,
    campo: &'static str,
) -> Result<&'a str, ErroTseBensSilver> {
    if coluna.is_null(linha) || coluna.value(linha).trim().is_empty() {
        return Err(ErroTseBensSilver::Obrigatorio { campo, linha });
    }
    Ok(coluna.value(linha).trim())
}

fn parse_data(coluna: Option<&StringArray>, linha: usize, ano: u16) -> Option<i32> {
    let coluna = coluna?;
    if coluna.is_null(linha) {
        return None;
    }
    let texto = coluna.value(linha).trim();
    let data = ["%d/%m/%Y", "%Y-%m-%d", "%d-%m-%Y", "%Y%m%d"]
        .iter()
        .find_map(|formato| NaiveDate::parse_from_str(texto, formato).ok())?;
    if data.year() < 1800 || data.year() > i32::from(ano) + 1 {
        return None;
    }
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch válida");
    Some(data.signed_duration_since(epoch).num_days() as i32)
}

fn parse_decimal(
    coluna: Option<&StringArray>,
    linha: usize,
) -> Result<Option<i128>, ErroTseBensSilver> {
    let Some(coluna) = coluna else {
        return Ok(None);
    };
    if coluna.is_null(linha) || coluna.value(linha).trim().is_empty() {
        return Ok(None);
    }
    decimal_escalado(coluna.value(linha).trim())
        .map(Some)
        .ok_or(ErroTseBensSilver::Valor { linha })
}

fn decimal_escalado(texto: &str) -> Option<i128> {
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

fn decimal_array(valores: Vec<Option<i128>>) -> Result<Decimal128Array, ErroTseBensSilver> {
    Decimal128Array::from(valores)
        .with_precision_and_scale(18, 2)
        .map_err(|_| ErroTseBensSilver::Schema)
}

fn registro_original(lote: &RecordBatch, linha: usize) -> Result<String, ErroTseBensSilver> {
    let mut original = serde_json::Map::new();
    for (indice, campo) in lote.schema().fields().iter().enumerate() {
        let coluna = lote
            .column(indice)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or(ErroTseBensSilver::Schema)?;
        original.insert(
            campo.name().clone(),
            if coluna.is_null(linha) {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(coluna.value(linha).to_owned())
            },
        );
    }
    serde_json::to_string(&original).map_err(|_| ErroTseBensSilver::Json)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn lote(
        ordem_nome: &str,
        ordem: &str,
        valor: &str,
        data_atualizacao_nome: &str,
        atualizacao: &str,
    ) -> RecordBatch {
        let mut colunas = vec![
            ("ANO_ELEICAO", vec![Some("2024")]),
            ("CD_ELEICAO", vec![Some("0042")]),
            ("SG_UF", vec![Some("AC")]),
            ("SG_UE", vec![Some("AC")]),
            ("SQ_CANDIDATO", vec![Some("000987")]),
            ("DT_ELEICAO", vec![Some("06/10/2024")]),
            ("CD_TIPO_BEM_CANDIDATO", vec![Some("01")]),
            ("DS_TIPO_BEM_CANDIDATO", vec![Some("Imóvel")]),
            ("DS_BEM_CANDIDATO", vec![Some("Descrição de exemplo")]),
            ("VR_BEM_CANDIDATO", vec![Some(valor)]),
        ];
        colunas.push((ordem_nome, vec![Some(ordem)]));
        colunas.push((data_atualizacao_nome, vec![Some(atualizacao)]));
        RecordBatch::try_from_iter(
            colunas
                .into_iter()
                .map(|(nome, valores)| (nome, Arc::new(StringArray::from(valores)) as _)),
        )
        .unwrap()
    }

    #[test]
    fn tipa_ordem_datas_e_decimal_sem_perder_codigos() {
        let bronze = lote(
            "NR_ORDEM_BEM_CANDIDATO",
            "1",
            "1234,56",
            "DT_ULT_ATUAL_BEM_CANDIDATO",
            "05/11/2024",
        );
        let mut chaves = HashSet::new();
        let saida = transformar_tse_bens(&bronze, 2024, &mut chaves).unwrap();
        assert_eq!(saida.schema(), esquema_tse_bens());
        assert_eq!(
            saida
                .column(5)
                .as_any()
                .downcast_ref::<UInt32Array>()
                .unwrap()
                .value(0),
            1
        );
        assert_eq!(
            saida
                .column(11)
                .as_any()
                .downcast_ref::<Decimal128Array>()
                .unwrap()
                .value(0),
            123_456
        );
        assert_eq!(
            saida
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "0042"
        );
        assert!(
            saida
                .column(12)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0)
                .contains("DT_ULT_ATUAL_BEM_CANDIDATO")
        );
    }

    #[test]
    fn aceita_alias_historico_ordem_candidato_e_data_nao_tipavel() {
        let bronze = lote(
            "NR_ORDEM_CANDIDATO",
            "7",
            ",95",
            "DT_ULTIMA_ATUALIZACAO",
            "000",
        );
        let mut chaves = HashSet::new();
        let saida = transformar_tse_bens(&bronze, 2024, &mut chaves).unwrap();
        assert_eq!(
            saida
                .column(11)
                .as_any()
                .downcast_ref::<Decimal128Array>()
                .unwrap()
                .value(0),
            95
        );
        assert!(saida.column(7).is_null(0));
    }

    #[test]
    fn rejeita_chave_incompleta_duplicada_ou_ano_divergente() {
        let mut chaves = HashSet::new();
        assert_eq!(
            transformar_tse_bens(
                &lote(
                    "NR_ORDEM_BEM_CANDIDATO",
                    "1",
                    "1,00",
                    "DT_ULT_ATUAL_BEM_CANDIDATO",
                    ""
                ),
                2022,
                &mut chaves,
            ),
            Err(ErroTseBensSilver::AnoDivergente { linha: 0 })
        );
        let duplicado = lote(
            "NR_ORDEM_BEM_CANDIDATO",
            "1",
            "1,00",
            "DT_ULT_ATUAL_BEM_CANDIDATO",
            "",
        );
        transformar_tse_bens(&duplicado, 2024, &mut chaves).unwrap();
        assert_eq!(
            transformar_tse_bens(&duplicado, 2024, &mut chaves),
            Err(ErroTseBensSilver::ChaveDuplicada { linha: 0 })
        );
    }

    #[test]
    fn rejeita_escala_decimal_invalida_e_ordem_nao_numerica() {
        let mut chaves = HashSet::new();
        assert_eq!(
            transformar_tse_bens(
                &lote(
                    "NR_ORDEM_BEM_CANDIDATO",
                    "abc",
                    "1,00",
                    "DT_ULT_ATUAL_BEM_CANDIDATO",
                    ""
                ),
                2024,
                &mut chaves,
            ),
            Err(ErroTseBensSilver::Obrigatorio {
                campo: "NR_ORDEM_BEM",
                linha: 0
            })
        );
        assert_eq!(decimal_escalado("1,001"), None);
        assert_eq!(decimal_escalado("-1,25"), Some(-125));
    }
}
