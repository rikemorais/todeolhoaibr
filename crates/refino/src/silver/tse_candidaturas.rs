//! Contrato Silver das candidaturas eleitorais do TSE.

use std::collections::HashSet;
use std::sync::Arc;

use arrow::array::{Array, Date32Array, RecordBatch, StringArray, UInt8Array, UInt16Array};
use arrow::datatypes::{DataType, Field, Schema};
use chrono::{Datelike, NaiveDate};
use thiserror::Error;

/// Erros ao transformar candidaturas eleitorais.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroTseCandidaturasSilver {
    /// O Parquet Bronze não tem os campos textuais esperados.
    #[error("schema Bronze de candidaturas do TSE incompatível")]
    Schema,
    /// Um dos campos da chave candidata está ausente ou vazio.
    #[error("campo {campo} inválido na linha {linha} do lote")]
    Chave {
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
    /// A chave candidata completa repete uma candidatura no mesmo consolidado.
    #[error("chave de candidatura duplicada na linha {linha} do lote")]
    ChaveDuplicada {
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// Os campos de origem não puderam ser preservados como JSON.
    #[error("não foi possível preservar os campos originais da candidatura")]
    Json,
}

/// Schema do contrato `tse.candidatura.registro.v1`.
pub fn esquema_tse_candidaturas() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("codigo_eleicao", DataType::Utf8, false),
        Field::new("uf", DataType::Utf8, false),
        Field::new("unidade_eleitoral", DataType::Utf8, false),
        Field::new("codigo_cargo", DataType::Utf8, false),
        Field::new("numero_candidato", DataType::Utf8, false),
        Field::new("sequencia_candidato", DataType::Utf8, false),
        Field::new("data_eleicao", DataType::Date32, true),
        Field::new("data_nascimento", DataType::Date32, true),
        Field::new("idade_data_posse", DataType::UInt8, true),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

/// Normaliza a chave, tipa datas e idade e mantém os atributos publicados na origem.
pub fn transformar_tse_candidaturas(
    lote: &RecordBatch,
    ano: u16,
    chaves_vistas: &mut HashSet<String>,
) -> Result<RecordBatch, ErroTseCandidaturasSilver> {
    if lote
        .schema()
        .fields()
        .iter()
        .any(|campo| campo.data_type() != &DataType::Utf8)
    {
        return Err(ErroTseCandidaturasSilver::Schema);
    }
    let nomes = [
        "ANO_ELEICAO",
        "CD_ELEICAO",
        "SG_UF",
        "SG_UE",
        "CD_CARGO",
        "NR_CANDIDATO",
        "SQ_CANDIDATO",
    ];
    let indices = nomes
        .iter()
        .map(|nome| {
            lote.schema()
                .index_of(nome)
                .map_err(|_| ErroTseCandidaturasSilver::Schema)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let colunas = indices
        .iter()
        .map(|indice| {
            lote.column(*indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroTseCandidaturasSilver::Schema)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let data_eleicao = coluna_opcional(lote, &["DT_ELEICAO"])?;
    let data_nascimento = coluna_opcional(lote, &["DT_NASCIMENTO"])?;
    let idade = coluna_opcional(lote, &["NR_IDADE_DATA_POSSE"])?;

    let mut datas_eleicao = Vec::with_capacity(lote.num_rows());
    let mut datas_nascimento = Vec::with_capacity(lote.num_rows());
    let mut idades = Vec::with_capacity(lote.num_rows());
    let mut originais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        let ano_linha = texto_obrigatorio(colunas[0], linha, "ANO_ELEICAO")?;
        if ano_linha.parse::<u16>().ok() != Some(ano) {
            return Err(ErroTseCandidaturasSilver::AnoDivergente { linha });
        }
        let valores_chave = colunas[1..]
            .iter()
            .zip(&nomes[1..])
            .map(|(coluna, nome)| texto_obrigatorio(coluna, linha, nome))
            .collect::<Result<Vec<_>, _>>()?;
        let chave = format!("{ano}\0{}", valores_chave.join("\0"));
        if !chaves_vistas.insert(chave) {
            return Err(ErroTseCandidaturasSilver::ChaveDuplicada { linha });
        }

        datas_eleicao.push(parse_data(data_eleicao, linha, ano));
        datas_nascimento.push(parse_data(data_nascimento, linha, ano));
        idades.push(parse_idade(idade, linha));
        originais.push(Some(registro_original(lote, linha)?));
    }

    let texto = |indice: usize| Arc::new(colunas[indice].clone()) as Arc<dyn Array>;
    let mut cols: Vec<Arc<dyn Array>> = vec![
        Arc::new(UInt16Array::from_value(ano, lote.num_rows())),
        texto(1),
        texto(2),
        texto(3),
        texto(4),
        texto(5),
        texto(6),
        Arc::new(Date32Array::from(datas_eleicao)),
        Arc::new(Date32Array::from(datas_nascimento)),
        Arc::new(UInt8Array::from(idades)),
        Arc::new(StringArray::from(originais)),
    ];
    cols.extend(crate::executor_sql::colunas_linhagem_teste(lote.num_rows()));
    RecordBatch::try_new(esquema_tse_candidaturas(), cols)
        .map_err(|_| ErroTseCandidaturasSilver::Schema)
}

fn coluna_opcional<'a>(
    lote: &'a RecordBatch,
    nomes: &[&str],
) -> Result<Option<&'a StringArray>, ErroTseCandidaturasSilver> {
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
        .ok_or(ErroTseCandidaturasSilver::Schema)
}

fn texto_obrigatorio<'a>(
    coluna: &'a StringArray,
    linha: usize,
    campo: &'static str,
) -> Result<&'a str, ErroTseCandidaturasSilver> {
    if coluna.is_null(linha) || coluna.value(linha).trim().is_empty() {
        return Err(ErroTseCandidaturasSilver::Chave { campo, linha });
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

fn parse_idade(coluna: Option<&StringArray>, linha: usize) -> Option<u8> {
    let coluna = coluna?;
    if coluna.is_null(linha) {
        return None;
    }
    let texto = coluna.value(linha).trim();
    if texto.is_empty() || matches!(texto, "-1" | "#NULO" | "#NE") {
        return None;
    }
    texto.parse::<u8>().ok()
}

fn registro_original(
    lote: &RecordBatch,
    linha: usize,
) -> Result<String, ErroTseCandidaturasSilver> {
    let mut original = serde_json::Map::new();
    for (indice, campo) in lote.schema().fields().iter().enumerate() {
        let coluna = lote
            .column(indice)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or(ErroTseCandidaturasSilver::Schema)?;
        original.insert(
            campo.name().clone(),
            if coluna.is_null(linha) {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(coluna.value(linha).to_owned())
            },
        );
    }
    serde_json::to_string(&original).map_err(|_| ErroTseCandidaturasSilver::Json)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn lote(anos: Vec<Option<&str>>, sequencias: Vec<Option<&str>>) -> RecordBatch {
        let tamanho = anos.len();
        let coluna =
            |nome, valores: Vec<Option<&str>>| (nome, Arc::new(StringArray::from(valores)) as _);
        RecordBatch::try_from_iter(vec![
            coluna("ANO_ELEICAO", anos),
            coluna("CD_ELEICAO", vec![Some("0042"); tamanho]),
            coluna("SG_UF", vec![Some("AC"); tamanho]),
            coluna("SG_UE", vec![Some("AC"); tamanho]),
            coluna("CD_CARGO", vec![Some("13"); tamanho]),
            coluna("NR_CANDIDATO", vec![Some("00013"); tamanho]),
            coluna("SQ_CANDIDATO", sequencias),
            coluna("DT_ELEICAO", vec![Some("06/10/2024"); tamanho]),
            coluna("DT_NASCIMENTO", vec![Some("01/02/1980"); tamanho]),
            coluna("NR_IDADE_DATA_POSSE", vec![Some("44"); tamanho]),
            coluna("NM_EMAIL", vec![Some("email@example.org"); tamanho]),
        ])
        .unwrap()
    }

    #[test]
    fn tipa_campos_preserva_codigos_e_registro_historico() {
        let mut chaves = HashSet::new();
        let saida = transformar_tse_candidaturas(
            &lote(vec![Some("2024")], vec![Some("000987")]),
            2024,
            &mut chaves,
        )
        .unwrap();
        assert_eq!(saida.schema(), esquema_tse_candidaturas());
        assert_eq!(
            saida
                .column(5)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "00013"
        );
        assert_eq!(
            saida
                .column(7)
                .as_any()
                .downcast_ref::<Date32Array>()
                .unwrap()
                .value(0),
            NaiveDate::from_ymd_opt(2024, 10, 6)
                .unwrap()
                .signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
                .num_days() as i32
        );
        assert!(
            saida
                .column(10)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0)
                .contains("NM_EMAIL")
        );
    }

    #[test]
    fn aceita_alias_historico_por_preservar_campos_de_origem() {
        let bronze = RecordBatch::try_from_iter(vec![
            (
                "ANO_ELEICAO",
                Arc::new(StringArray::from(vec!["2000"])) as _,
            ),
            ("CD_ELEICAO", Arc::new(StringArray::from(vec!["0042"])) as _),
            ("SG_UF", Arc::new(StringArray::from(vec!["AC"])) as _),
            ("SG_UE", Arc::new(StringArray::from(vec!["AC"])) as _),
            ("CD_CARGO", Arc::new(StringArray::from(vec!["13"])) as _),
            (
                "NR_CANDIDATO",
                Arc::new(StringArray::from(vec!["00013"])) as _,
            ),
            (
                "SQ_CANDIDATO",
                Arc::new(StringArray::from(vec!["00001"])) as _,
            ),
            (
                "DT_ELEICAO",
                Arc::new(StringArray::from(vec!["01/10/2000"])) as _,
            ),
            (
                "NM_EMAIL",
                Arc::new(StringArray::from(vec!["legado@example.org"])) as _,
            ),
        ])
        .unwrap();
        let mut chaves = HashSet::new();
        let saida = transformar_tse_candidaturas(&bronze, 2000, &mut chaves).unwrap();
        assert_eq!(saida.num_rows(), 1);
        assert!(saida.column(8).is_null(0));
    }

    #[test]
    fn rejeita_ano_divergente_chave_ausente_e_duplicata() {
        let mut chaves = HashSet::new();
        assert_eq!(
            transformar_tse_candidaturas(
                &lote(vec![Some("2022")], vec![Some("1")]),
                2024,
                &mut chaves
            ),
            Err(ErroTseCandidaturasSilver::AnoDivergente { linha: 0 })
        );
        assert_eq!(
            transformar_tse_candidaturas(&lote(vec![Some("2024")], vec![None]), 2024, &mut chaves),
            Err(ErroTseCandidaturasSilver::Chave {
                campo: "SQ_CANDIDATO",
                linha: 0
            })
        );
        let duplicadas = lote(vec![Some("2024"), Some("2024")], vec![Some("1"), Some("1")]);
        assert_eq!(
            transformar_tse_candidaturas(&duplicadas, 2024, &mut chaves),
            Err(ErroTseCandidaturasSilver::ChaveDuplicada { linha: 1 })
        );
    }

    #[test]
    fn trata_sentinelas_e_datas_fora_da_eleicao_como_nulas() {
        let mut origem = lote(vec![Some("2024")], vec![Some("1")]);
        let campos = origem.schema().fields().clone();
        let arrays: Vec<_> = campos
            .iter()
            .enumerate()
            .map(|(indice, campo)| {
                let valor = match campo.name().as_str() {
                    "DT_ELEICAO" => "06/10/2099",
                    "DT_NASCIMENTO" => "#NULO",
                    "NR_IDADE_DATA_POSSE" => "-1",
                    _ => origem
                        .column(indice)
                        .as_any()
                        .downcast_ref::<StringArray>()
                        .unwrap()
                        .value(0),
                };
                (
                    campo.name().clone(),
                    Arc::new(StringArray::from(vec![valor])) as _,
                )
            })
            .collect();
        origem = RecordBatch::try_from_iter(arrays).unwrap();
        let mut chaves = HashSet::new();
        let saida = transformar_tse_candidaturas(&origem, 2024, &mut chaves).unwrap();
        assert!(saida.column(7).is_null(0));
        assert!(saida.column(8).is_null(0));
        assert!(saida.column(9).is_null(0));
    }
}
