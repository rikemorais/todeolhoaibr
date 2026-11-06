//! Contrato Silver para eventos de tramitação das proposições da Câmara.

use std::sync::Arc;

use arrow::array::{
    Array, RecordBatch, StringArray, TimestampMicrosecondArray, UInt16Array, UInt32Array,
};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use chrono::NaiveDateTime;
use thiserror::Error;

/// Erros ao transformar um lote Bronze de tramitações.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroCamaraTramitacoesSilver {
    /// Faltam colunas da origem ou uma coluna deixou de ser textual.
    #[error("schema Bronze de tramitações da Câmara incompatível")]
    Schema,
    /// Um valor preenchido não corresponde ao formato declarado.
    #[error("campo {campo} inválido na linha {linha} do lote")]
    Valor {
        /// Nome do campo de origem inválido.
        campo: &'static str,
        /// Índice da linha do lote, começando em zero.
        linha: usize,
    },
    /// O JSON com os campos da origem não pôde ser serializado.
    #[error("não foi possível preservar os campos originais da tramitação")]
    Json,
}

/// Schema versionado do contrato `camara.tramitacao.evento.v1`.
pub fn esquema_camara_tramitacoes() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano", DataType::UInt16, false),
        Field::new("uri_proposicao", DataType::Utf8, false),
        Field::new("sequencia", DataType::UInt32, true),
        Field::new(
            "data_hora",
            DataType::Timestamp(TimeUnit::Microsecond, None),
            true,
        ),
        Field::new("id_tramitacao", DataType::Utf8, true),
        Field::new("codigo_situacao", DataType::Utf8, true),
        Field::new("numero", DataType::Utf8, true),
        Field::new("sigla_orgao", DataType::Utf8, true),
        Field::new("uri_orgao", DataType::Utf8, true),
        Field::new("regime", DataType::Utf8, true),
        Field::new("descricao_tramitacao", DataType::Utf8, true),
        Field::new("despacho", DataType::Utf8, true),
        Field::new("registro_original", DataType::Utf8, false),
    ]))
}

/// Tipifica os campos consultados e preserva todos os atributos e linhas da origem.
pub fn transformar_camara_tramitacoes(
    lote: &RecordBatch,
    ano: u16,
) -> Result<RecordBatch, ErroCamaraTramitacoesSilver> {
    let nomes = [
        "uriProposicao",
        "sequencia",
        "dataHora",
        "idTramitacao",
        "codSituacao",
        "numero",
        "siglaOrgao",
        "uriOrgao",
        "regime",
        "descricaoTramitacao",
        "despacho",
    ];
    let indices: Vec<Option<usize>> = nomes
        .iter()
        .map(|nome| lote.schema().index_of(nome).ok())
        .collect();
    if indices[..3].iter().any(Option::is_none)
        || indices
            .iter()
            .flatten()
            .any(|indice| lote.schema().field(*indice).data_type() != &DataType::Utf8)
        || lote
            .schema()
            .fields()
            .iter()
            .any(|campo| campo.data_type() != &DataType::Utf8)
    {
        return Err(ErroCamaraTramitacoesSilver::Schema);
    }
    let mut colunas = Vec::with_capacity(indices.len());
    for indice in indices {
        colunas.push(match indice {
            Some(indice) => Some(
                lote.column(indice)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or(ErroCamaraTramitacoesSilver::Schema)?,
            ),
            None => None,
        });
    }

    let mut uri_proposicao = Vec::with_capacity(lote.num_rows());
    let mut sequencias = Vec::with_capacity(lote.num_rows());
    let mut datas_horas = Vec::with_capacity(lote.num_rows());
    let mut originais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        let uri = valor_obrigatorio(
            colunas[0].expect("required column checked"),
            linha,
            "uriProposicao",
        )?;
        uri_proposicao.push(Some(uri.to_owned()));
        sequencias.push(parse_u32(
            colunas[1].expect("required column checked"),
            linha,
            "sequencia",
        )?);
        datas_horas.push(parse_data_hora(
            colunas[2].expect("required column checked"),
            linha,
            "dataHora",
        )?);

        let mut original = serde_json::Map::new();
        for (indice, campo) in lote.schema().fields().iter().enumerate() {
            let valores = lote
                .column(indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroCamaraTramitacoesSilver::Schema)?;
            original.insert(
                campo.name().clone(),
                if valores.is_null(linha) {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(valores.value(linha).to_owned())
                },
            );
        }
        originais.push(Some(
            serde_json::to_string(&original).map_err(|_| ErroCamaraTramitacoesSilver::Json)?,
        ));
    }

    let texto = |indice: usize| -> Arc<dyn Array> {
        match colunas[indice] {
            Some(valores) => Arc::new(valores.clone()),
            None => Arc::new(StringArray::new_null(lote.num_rows())),
        }
    };
    RecordBatch::try_new(
        esquema_camara_tramitacoes(),
        vec![
            Arc::new(UInt16Array::from_value(ano, lote.num_rows())),
            Arc::new(StringArray::from(uri_proposicao)),
            Arc::new(UInt32Array::from(sequencias)),
            Arc::new(TimestampMicrosecondArray::from(datas_horas)),
            texto(3),
            texto(4),
            texto(5),
            texto(6),
            texto(7),
            texto(8),
            texto(9),
            texto(10),
            Arc::new(StringArray::from(originais)),
        ],
    )
    .map_err(|_| ErroCamaraTramitacoesSilver::Schema)
}

fn valor_obrigatorio<'a>(
    valores: &'a StringArray,
    linha: usize,
    campo: &'static str,
) -> Result<&'a str, ErroCamaraTramitacoesSilver> {
    if valores.is_null(linha) || valores.value(linha).trim().is_empty() {
        return Err(ErroCamaraTramitacoesSilver::Valor { campo, linha });
    }
    Ok(valores.value(linha).trim())
}

fn parse_u32(
    valores: &StringArray,
    linha: usize,
    campo: &'static str,
) -> Result<Option<u32>, ErroCamaraTramitacoesSilver> {
    if valores.is_null(linha) || valores.value(linha).trim().is_empty() {
        return Ok(None);
    }
    valores
        .value(linha)
        .trim()
        .parse()
        .map(Some)
        .map_err(|_| ErroCamaraTramitacoesSilver::Valor { campo, linha })
}

fn parse_data_hora(
    valores: &StringArray,
    linha: usize,
    campo: &'static str,
) -> Result<Option<i64>, ErroCamaraTramitacoesSilver> {
    if valores.is_null(linha) || valores.value(linha).trim().is_empty() {
        return Ok(None);
    }
    let texto = valores.value(linha).trim();
    let data_hora = [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
    ]
    .iter()
    .find_map(|formato| NaiveDateTime::parse_from_str(texto, formato).ok())
    .ok_or(ErroCamaraTramitacoesSilver::Valor { campo, linha })?;
    Ok(Some(data_hora.and_utc().timestamp_micros()))
}

/// Extrai o ano da partição declarada pelo catálogo Bronze.
pub fn ano_do_recurso(recurso: &str) -> Option<u16> {
    let ano = recurso.strip_prefix("tramitacoes/ano=")?;
    if ano.contains('/') {
        return None;
    }
    ano.parse().ok()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn lote(
        uri: Vec<Option<&str>>,
        sequencia: Vec<Option<&str>>,
        data_hora: Vec<Option<&str>>,
    ) -> RecordBatch {
        let tamanho = uri.len();
        let c =
            |nome, valores: Vec<Option<&str>>| (nome, Arc::new(StringArray::from(valores)) as _);
        RecordBatch::try_from_iter(vec![
            c("uriProposicao", uri),
            c("sequencia", sequencia),
            c("dataHora", data_hora),
            c("idTramitacao", vec![Some("9"); tamanho]),
            c("codSituacao", vec![Some("1220"); tamanho]),
            c("numero", vec![Some("078"); tamanho]),
            c("siglaOrgao", vec![Some("PLEN"); tamanho]),
            c("uriOrgao", vec![None; tamanho]),
            c("regime", vec![Some("Urgência"); tamanho]),
            c("descricaoTramitacao", vec![Some("Aprovado"); tamanho]),
            c("despacho", vec![Some("Encaminhado"); tamanho]),
        ])
        .unwrap()
    }

    #[test]
    fn declara_contrato_tipado_e_preserva_ids_como_texto() {
        let saida = transformar_camara_tramitacoes(
            &lote(
                vec![Some(
                    "https://dadosabertos.camara.leg.br/api/v2/proposicoes/2431810",
                )],
                vec![Some("8")],
                vec![Some("2024-07-11T09:00:00")],
            ),
            2024,
        )
        .unwrap();

        assert_eq!(saida.schema(), esquema_camara_tramitacoes());
        assert_eq!(saida.num_rows(), 1);
        assert_eq!(
            saida
                .column(2)
                .as_any()
                .downcast_ref::<UInt32Array>()
                .unwrap()
                .value(0),
            8
        );
        assert_eq!(
            saida
                .column(4)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "9"
        );
        assert_eq!(
            saida
                .column(3)
                .as_any()
                .downcast_ref::<TimestampMicrosecondArray>()
                .unwrap()
                .value(0),
            NaiveDateTime::parse_from_str("2024-07-11T09:00:00", "%Y-%m-%dT%H:%M:%S")
                .unwrap()
                .and_utc()
                .timestamp_micros()
        );
        let original: serde_json::Value = serde_json::from_str(
            saida
                .column(12)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
        )
        .unwrap();
        assert_eq!(original["numero"], "078");
    }

    #[test]
    fn preserva_registros_repetidos_e_sequencia_ausente() {
        let mesma = "https://dadosabertos.camara.leg.br/api/v2/proposicoes/2431810";
        let lote_repetido = lote(
            vec![Some(mesma), Some(mesma)],
            vec![Some("8"), Some("8")],
            vec![Some("2024-07-11T09:00:00"), Some("2024-07-11T09:00:00")],
        );
        let saida = transformar_camara_tramitacoes(&lote_repetido, 2024).unwrap();
        assert_eq!(saida.num_rows(), 2);
        assert_eq!(
            saida
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            saida
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(1)
        );
        assert!(
            transformar_camara_tramitacoes(
                &lote(
                    vec![Some(mesma)],
                    vec![None],
                    vec![Some("2024-07-11T09:00:00")]
                ),
                2024,
            )
            .unwrap()
            .column(2)
            .is_null(0)
        );
    }

    #[test]
    fn rejeita_uri_obrigatoria_e_sequencia_invalida() {
        assert!(matches!(
            transformar_camara_tramitacoes(
                &lote(
                    vec![None],
                    vec![Some("1")],
                    vec![Some("2024-01-01T00:00:00")]
                ),
                2024
            ),
            Err(ErroCamaraTramitacoesSilver::Valor {
                campo: "uriProposicao",
                ..
            })
        ));
        assert!(matches!(
            transformar_camara_tramitacoes(
                &lote(
                    vec![Some("uri")],
                    vec![Some("1.5")],
                    vec![Some("2024-01-01T00:00:00")]
                ),
                2024
            ),
            Err(ErroCamaraTramitacoesSilver::Valor {
                campo: "sequencia",
                ..
            })
        ));
    }

    #[test]
    fn valida_recurso_anual() {
        assert_eq!(ano_do_recurso("tramitacoes/ano=2024"), Some(2024));
        assert_eq!(ano_do_recurso("tramitacoes/ano=x"), None);
        assert_eq!(ano_do_recurso("tramitacoes/ano=2024/sub"), None);
    }
}
