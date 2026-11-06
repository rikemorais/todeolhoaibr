//! Contrato Silver dos votos apurados por seção eleitoral do TSE.

use std::collections::HashSet;
use std::sync::Arc;

use arrow::array::{
    Array, Date32Array, RecordBatch, StringArray, UInt8Array, UInt16Array, UInt32Array,
};
use arrow::datatypes::{DataType, Field, Schema};
use chrono::NaiveDate;
use thiserror::Error;

/// Erros ao transformar os votos do TSE por seção.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroTseVotacaoSecaoSilver {
    /// O Parquet Bronze não contém os campos textuais esperados.
    #[error("schema Bronze de votação por seção do TSE incompatível")]
    Schema,
    /// Um dos campos da chave está ausente, vazio ou fora do domínio.
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
    /// A chave da apuração repete uma linha no mesmo recurso.
    #[error("chave de votação por seção duplicada na linha {linha} do lote")]
    ChaveDuplicada {
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// A data preenchida não pôde ser interpretada.
    #[error("DT_ELEICAO inválida na linha {linha} do lote")]
    Data {
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// A quantidade de votos preenchida não é inteira ou excede UInt32.
    #[error("QT_VOTOS inválida na linha {linha} do lote")]
    Votos {
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// Os campos de origem não puderam ser preservados como JSON.
    #[error("não foi possível preservar os campos originais da votação")]
    Json,
}

/// Schema do contrato `tse.votacao.secao.v1`.
pub fn esquema_tse_votacao_secao() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("codigo_eleicao", DataType::Utf8, false),
        Field::new("turno", DataType::UInt8, false),
        Field::new("uf", DataType::Utf8, false),
        Field::new("unidade_eleitoral", DataType::Utf8, false),
        Field::new("codigo_municipio", DataType::Utf8, false),
        Field::new("zona", DataType::Utf8, false),
        Field::new("secao", DataType::Utf8, false),
        Field::new("codigo_cargo", DataType::Utf8, false),
        Field::new("numero_votavel", DataType::Utf8, false),
        Field::new("quantidade_votos", DataType::UInt32, false),
        Field::new("data_eleicao", DataType::Date32, true),
        Field::new("tipo_eleicao", DataType::Utf8, true),
        Field::new("nome_eleicao", DataType::Utf8, true),
        Field::new("abrangencia", DataType::Utf8, true),
        Field::new("nome_unidade_eleitoral", DataType::Utf8, true),
        Field::new("nome_municipio", DataType::Utf8, true),
        Field::new("descricao_cargo", DataType::Utf8, true),
        Field::new("nome_votavel", DataType::Utf8, true),
        Field::new("numero_local_votacao", DataType::Utf8, true),
        Field::new("sequencia_candidato", DataType::Utf8, true),
        Field::new("nome_local_votacao", DataType::Utf8, true),
        Field::new("endereco_local_votacao", DataType::Utf8, true),
        Field::new("registro_original", DataType::Utf8, false),
    ]))
}

/// Tipa a chave da apuração, turno e quantidade de votos sem alterar códigos textuais.
pub fn transformar_tse_votacao_secao(
    lote: &RecordBatch,
    ano: u16,
    uf_recurso: &str,
    chaves_vistas: &mut HashSet<String>,
) -> Result<RecordBatch, ErroTseVotacaoSecaoSilver> {
    if lote
        .schema()
        .fields()
        .iter()
        .any(|campo| campo.data_type() != &DataType::Utf8)
    {
        return Err(ErroTseVotacaoSecaoSilver::Schema);
    }

    let nomes = [
        "ANO_ELEICAO",
        "CD_ELEICAO",
        "NR_TURNO",
        "SG_UF",
        "SG_UE",
        "CD_MUNICIPIO",
        "NR_ZONA",
        "NR_SECAO",
        "CD_CARGO",
        "NR_VOTAVEL",
        "QT_VOTOS",
    ];
    let colunas = nomes
        .iter()
        .map(|nome| coluna_obrigatoria(lote, nome))
        .collect::<Result<Vec<_>, _>>()?;
    let data_eleicao = coluna_opcional(lote, "DT_ELEICAO")?;
    let tipo_eleicao = coluna_opcional(lote, "NM_TIPO_ELEICAO")?;
    let nome_eleicao = coluna_opcional(lote, "DS_ELEICAO")?;
    let abrangencia = coluna_opcional(lote, "TP_ABRANGENCIA")?;
    let nome_unidade = coluna_opcional(lote, "NM_UE")?;
    let nome_municipio = coluna_opcional(lote, "NM_MUNICIPIO")?;
    let descricao_cargo = coluna_opcional(lote, "DS_CARGO")?;
    let nome_votavel = coluna_opcional(lote, "NM_VOTAVEL")?;
    let numero_local = coluna_opcional(lote, "NR_LOCAL_VOTACAO")?;
    let sequencia_candidato = coluna_opcional(lote, "SQ_CANDIDATO")?;
    let nome_local = coluna_opcional(lote, "NM_LOCAL_VOTACAO")?;
    let endereco_local = coluna_opcional(lote, "DS_LOCAL_VOTACAO_ENDERECO")?;

    let mut turnos = Vec::with_capacity(lote.num_rows());
    let mut votos = Vec::with_capacity(lote.num_rows());
    let mut datas = Vec::with_capacity(lote.num_rows());
    let mut originais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        if texto_obrigatorio(colunas[0], linha, "ANO_ELEICAO")?
            .parse::<u16>()
            .ok()
            != Some(ano)
        {
            return Err(ErroTseVotacaoSecaoSilver::AnoDivergente { linha });
        }
        let uf = texto_obrigatorio(colunas[3], linha, "SG_UF")?;
        if uf_recurso != "BR" && uf != uf_recurso {
            return Err(ErroTseVotacaoSecaoSilver::Obrigatorio {
                campo: "SG_UF",
                linha,
            });
        }
        let turno = texto_obrigatorio(colunas[2], linha, "NR_TURNO")?
            .parse::<u8>()
            .ok()
            .filter(|turno| *turno > 0)
            .ok_or(ErroTseVotacaoSecaoSilver::Obrigatorio {
                campo: "NR_TURNO",
                linha,
            })?;
        let quantidade_votos = texto_obrigatorio(colunas[10], linha, "QT_VOTOS")?
            .parse::<u32>()
            .map_err(|_| ErroTseVotacaoSecaoSilver::Votos { linha })?;
        let chave = [
            ano.to_string(),
            texto_obrigatorio(colunas[1], linha, "CD_ELEICAO")?.to_owned(),
            uf.to_owned(),
            turno.to_string(),
            texto_obrigatorio(colunas[5], linha, "CD_MUNICIPIO")?.to_owned(),
            texto_obrigatorio(colunas[6], linha, "NR_ZONA")?.to_owned(),
            texto_obrigatorio(colunas[7], linha, "NR_SECAO")?.to_owned(),
            texto_obrigatorio(colunas[8], linha, "CD_CARGO")?.to_owned(),
            texto_obrigatorio(colunas[9], linha, "NR_VOTAVEL")?.to_owned(),
        ]
        .join("\0");
        if !chaves_vistas.insert(chave) {
            return Err(ErroTseVotacaoSecaoSilver::ChaveDuplicada { linha });
        }
        datas.push(parse_data(data_eleicao, linha)?);
        turnos.push(turno);
        votos.push(quantidade_votos);
        originais.push(Some(registro_original(lote, linha)?));
    }

    let texto_preenchido = |coluna: Option<&StringArray>| -> Arc<dyn Array> {
        coluna.map_or_else(
            || Arc::new(StringArray::from(vec![None::<&str>; lote.num_rows()])) as Arc<dyn Array>,
            |coluna| Arc::new(coluna.clone()),
        )
    };
    RecordBatch::try_new(
        esquema_tse_votacao_secao(),
        vec![
            Arc::new(UInt16Array::from_value(ano, lote.num_rows())),
            Arc::new(colunas[1].clone()),
            Arc::new(UInt8Array::from(turnos)),
            Arc::new(colunas[3].clone()),
            Arc::new(colunas[4].clone()),
            Arc::new(colunas[5].clone()),
            Arc::new(colunas[6].clone()),
            Arc::new(colunas[7].clone()),
            Arc::new(colunas[8].clone()),
            Arc::new(colunas[9].clone()),
            Arc::new(UInt32Array::from(votos)),
            Arc::new(Date32Array::from(datas)),
            texto_preenchido(tipo_eleicao),
            texto_preenchido(nome_eleicao),
            texto_preenchido(abrangencia),
            texto_preenchido(nome_unidade),
            texto_preenchido(nome_municipio),
            texto_preenchido(descricao_cargo),
            texto_preenchido(nome_votavel),
            texto_preenchido(numero_local),
            texto_preenchido(sequencia_candidato),
            texto_preenchido(nome_local),
            texto_preenchido(endereco_local),
            Arc::new(StringArray::from(originais)),
        ],
    )
    .map_err(|_| ErroTseVotacaoSecaoSilver::Schema)
}

fn coluna_obrigatoria<'a>(
    lote: &'a RecordBatch,
    nome: &'static str,
) -> Result<&'a StringArray, ErroTseVotacaoSecaoSilver> {
    coluna_opcional(lote, nome)?.ok_or(ErroTseVotacaoSecaoSilver::Schema)
}

fn coluna_opcional<'a>(
    lote: &'a RecordBatch,
    nome: &str,
) -> Result<Option<&'a StringArray>, ErroTseVotacaoSecaoSilver> {
    let Ok(indice) = lote.schema().index_of(nome) else {
        return Ok(None);
    };
    lote.column(indice)
        .as_any()
        .downcast_ref::<StringArray>()
        .map(Some)
        .ok_or(ErroTseVotacaoSecaoSilver::Schema)
}

fn texto_obrigatorio<'a>(
    coluna: &'a StringArray,
    linha: usize,
    campo: &'static str,
) -> Result<&'a str, ErroTseVotacaoSecaoSilver> {
    if coluna.is_null(linha) || coluna.value(linha).trim().is_empty() {
        return Err(ErroTseVotacaoSecaoSilver::Obrigatorio { campo, linha });
    }
    Ok(coluna.value(linha).trim())
}

fn parse_data(
    coluna: Option<&StringArray>,
    linha: usize,
) -> Result<Option<i32>, ErroTseVotacaoSecaoSilver> {
    let Some(coluna) = coluna else {
        return Ok(None);
    };
    if coluna.is_null(linha) || coluna.value(linha).trim().is_empty() {
        return Ok(None);
    }
    let texto = coluna.value(linha).trim();
    let data = ["%d/%m/%Y", "%Y-%m-%d", "%Y%m%d"]
        .iter()
        .find_map(|formato| NaiveDate::parse_from_str(texto, formato).ok())
        .ok_or(ErroTseVotacaoSecaoSilver::Data { linha })?;
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch válida");
    Ok(Some(data.signed_duration_since(epoch).num_days() as i32))
}

fn registro_original(
    lote: &RecordBatch,
    linha: usize,
) -> Result<String, ErroTseVotacaoSecaoSilver> {
    let mut original = serde_json::Map::new();
    for (indice, campo) in lote.schema().fields().iter().enumerate() {
        let coluna = lote
            .column(indice)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or(ErroTseVotacaoSecaoSilver::Schema)?;
        original.insert(
            campo.name().clone(),
            if coluna.is_null(linha) {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(coluna.value(linha).to_owned())
            },
        );
    }
    serde_json::to_string(&original).map_err(|_| ErroTseVotacaoSecaoSilver::Json)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn lote(votavel: &str, votos: &str, uf: &str) -> RecordBatch {
        RecordBatch::try_from_iter(vec![
            (
                "ANO_ELEICAO",
                Arc::new(StringArray::from(vec!["2024"])) as _,
            ),
            ("CD_ELEICAO", Arc::new(StringArray::from(vec!["0097"])) as _),
            ("NR_TURNO", Arc::new(StringArray::from(vec!["1"])) as _),
            ("SG_UF", Arc::new(StringArray::from(vec![uf])) as _),
            ("SG_UE", Arc::new(StringArray::from(vec!["AC"])) as _),
            (
                "CD_MUNICIPIO",
                Arc::new(StringArray::from(vec!["00123"])) as _,
            ),
            ("NR_ZONA", Arc::new(StringArray::from(vec!["01"])) as _),
            ("NR_SECAO", Arc::new(StringArray::from(vec!["0007"])) as _),
            ("CD_CARGO", Arc::new(StringArray::from(vec!["001"])) as _),
            (
                "NR_VOTAVEL",
                Arc::new(StringArray::from(vec![votavel])) as _,
            ),
            ("QT_VOTOS", Arc::new(StringArray::from(vec![votos])) as _),
            (
                "DT_ELEICAO",
                Arc::new(StringArray::from(vec!["06/10/2024"])) as _,
            ),
            (
                "DS_CARGO",
                Arc::new(StringArray::from(vec!["Presidente"])) as _,
            ),
        ])
        .unwrap()
    }

    #[test]
    fn tipa_votos_e_data_e_preserva_codigos_com_zeros() {
        let mut chaves = HashSet::new();
        let saida =
            transformar_tse_votacao_secao(&lote("00013", "245", "AC"), 2024, "AC", &mut chaves)
                .unwrap();
        assert_eq!(saida.schema(), esquema_tse_votacao_secao());
        assert_eq!(
            saida
                .column(5)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "00123"
        );
        assert_eq!(
            saida
                .column(10)
                .as_any()
                .downcast_ref::<UInt32Array>()
                .unwrap()
                .value(0),
            245
        );
        assert_eq!(
            saida
                .column(2)
                .as_any()
                .downcast_ref::<UInt8Array>()
                .unwrap()
                .value(0),
            1
        );
        assert!(
            saida
                .column(11)
                .as_any()
                .downcast_ref::<Date32Array>()
                .unwrap()
                .is_valid(0)
        );
    }

    #[test]
    fn rejeita_ano_uf_chave_e_votos_invalidos() {
        let mut chaves = HashSet::new();
        assert!(matches!(
            transformar_tse_votacao_secao(&lote("13", "245", "AC"), 2022, "AC", &mut chaves),
            Err(ErroTseVotacaoSecaoSilver::AnoDivergente { linha: 0 })
        ));
        assert!(matches!(
            transformar_tse_votacao_secao(&lote("13", "245", "RO"), 2024, "AC", &mut chaves),
            Err(ErroTseVotacaoSecaoSilver::Obrigatorio {
                campo: "SG_UF",
                linha: 0
            })
        ));
        assert!(matches!(
            transformar_tse_votacao_secao(&lote("13", "x", "AC"), 2024, "AC", &mut chaves),
            Err(ErroTseVotacaoSecaoSilver::Votos { linha: 0 })
        ));
    }

    #[test]
    fn valida_chaves_repetidas_entre_lotes() {
        let mut chaves = HashSet::new();
        let lote = lote("13", "245", "AC");
        transformar_tse_votacao_secao(&lote, 2024, "AC", &mut chaves).unwrap();
        assert_eq!(
            transformar_tse_votacao_secao(&lote, 2024, "AC", &mut chaves),
            Err(ErroTseVotacaoSecaoSilver::ChaveDuplicada { linha: 0 })
        );
    }
}
