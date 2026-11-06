//! Contrato Silver para o catálogo de legislaturas da Câmara dos Deputados.

use std::collections::HashSet;
use std::sync::Arc;

use arrow::array::{Array, Date32Array, RecordBatch, StringArray, UInt16Array};
use arrow::datatypes::{DataType, Field, Schema};
use chrono::NaiveDate;
use thiserror::Error;

/// Erros encontrados ao validar a tabela Bronze de legislaturas.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroCamaraLegislaturasSilver {
    /// A parte não contém as colunas textuais esperadas.
    #[error("schema Bronze de legislaturas da Câmara incompatível")]
    Schema,
    /// Um ID, uma data ou o ano eleitoral não pode ser interpretado.
    #[error("campo inválido na linha {linha} do lote")]
    Valor {
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
    /// O mesmo ID ocorre mais de uma vez na parte.
    #[error("ID de legislatura duplicado na linha {linha} do lote")]
    IdDuplicado {
        /// Índice da segunda ocorrência dentro do lote, começando em zero.
        linha: usize,
    },
}

/// Schema versionado do contrato `camara.legislatura.periodo.v1`.
pub fn esquema_camara_legislaturas() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::UInt16, false),
        Field::new("data_inicio", DataType::Date32, false),
        Field::new("data_fim", DataType::Date32, false),
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("uri", DataType::Utf8, true),
    ]))
}

/// Interpreta o ID sequencial, o período e o ano eleitoral da API da Câmara.
pub fn transformar_camara_legislaturas(
    lote: &RecordBatch,
) -> Result<RecordBatch, ErroCamaraLegislaturasSilver> {
    let schema = lote.schema();
    let indices: Vec<usize> = ["id", "dataInicio", "dataFim", "dataEleicao", "uri"]
        .iter()
        .map(|nome| {
            schema
                .index_of(nome)
                .map_err(|_| ErroCamaraLegislaturasSilver::Schema)
        })
        .collect::<Result<_, _>>()?;
    if indices
        .iter()
        .any(|indice| schema.field(*indice).data_type() != &DataType::Utf8)
    {
        return Err(ErroCamaraLegislaturasSilver::Schema);
    }
    let arrays: Vec<&StringArray> = indices
        .iter()
        .map(|indice| {
            lote.column(*indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroCamaraLegislaturasSilver::Schema)
        })
        .collect::<Result<_, _>>()?;
    let mut ids = HashSet::with_capacity(lote.num_rows());
    let mut legislaturas = Vec::with_capacity(lote.num_rows());
    let mut inicios = Vec::with_capacity(lote.num_rows());
    let mut fins = Vec::with_capacity(lote.num_rows());
    let mut anos = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        let parse =
            |indice: usize| (!arrays[indice].is_null(linha)).then(|| arrays[indice].value(linha));
        let id = parse(0)
            .and_then(|texto| texto.parse::<u16>().ok())
            .ok_or(ErroCamaraLegislaturasSilver::Valor { linha })?;
        if !ids.insert(id) {
            return Err(ErroCamaraLegislaturasSilver::IdDuplicado { linha });
        }
        let inicio = parse(1)
            .and_then(parse_data)
            .ok_or(ErroCamaraLegislaturasSilver::Valor { linha })?;
        let fim = parse(2)
            .and_then(parse_data)
            .ok_or(ErroCamaraLegislaturasSilver::Valor { linha })?;
        let ano = parse(3)
            .and_then(|texto| texto.parse::<u16>().ok())
            .ok_or(ErroCamaraLegislaturasSilver::Valor { linha })?;
        if fim < inicio {
            return Err(ErroCamaraLegislaturasSilver::Valor { linha });
        }
        legislaturas.push(id);
        inicios.push(inicio);
        fins.push(fim);
        anos.push(ano);
    }
    let uri = arrays[4].clone();
    let para_dias = |data: NaiveDate| {
        data.signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1).expect("epoch válida"))
            .num_days() as i32
    };
    RecordBatch::try_new(
        esquema_camara_legislaturas(),
        vec![
            Arc::new(UInt16Array::from(legislaturas)),
            Arc::new(Date32Array::from(
                inicios.into_iter().map(para_dias).collect::<Vec<_>>(),
            )),
            Arc::new(Date32Array::from(
                fins.into_iter().map(para_dias).collect::<Vec<_>>(),
            )),
            Arc::new(UInt16Array::from(anos)),
            Arc::new(uri.clone()),
        ],
    )
    .map_err(|_| ErroCamaraLegislaturasSilver::Schema)
}

fn parse_data(texto: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(texto, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod testes {
    use super::*;

    fn lote(
        ids: Vec<Option<&str>>,
        inicio: Vec<Option<&str>>,
        fim: Vec<Option<&str>>,
        ano: Vec<Option<&str>>,
    ) -> RecordBatch {
        let quantidade = ids.len();
        let uris =
            vec![Some("https://dadosabertos.camara.leg.br/api/v2/legislaturas/57"); quantidade];
        RecordBatch::try_from_iter(vec![
            ("id", Arc::new(StringArray::from(ids)) as _),
            ("dataInicio", Arc::new(StringArray::from(inicio)) as _),
            ("dataFim", Arc::new(StringArray::from(fim)) as _),
            ("dataEleicao", Arc::new(StringArray::from(ano)) as _),
            ("uri", Arc::new(StringArray::from(uris)) as _),
        ])
        .unwrap()
    }

    #[test]
    fn tipa_id_periodo_e_ano_eleitoral() {
        let bronze = lote(
            vec![Some("57")],
            vec![Some("2023-02-01")],
            vec![Some("2027-01-31")],
            vec![Some("2022")],
        );
        let silver = transformar_camara_legislaturas(&bronze).unwrap();
        assert_eq!(silver.schema(), esquema_camara_legislaturas());
        assert_eq!(
            silver
                .column(0)
                .as_any()
                .downcast_ref::<UInt16Array>()
                .unwrap()
                .value(0),
            57
        );
        assert_eq!(
            silver
                .column(1)
                .as_any()
                .downcast_ref::<Date32Array>()
                .unwrap()
                .value(0),
            19389
        );
    }

    #[test]
    fn rejeita_periodo_invalido_e_id_duplicado() {
        let duplicada = lote(
            vec![Some("57"), Some("57")],
            vec![Some("2023-02-01"); 2],
            vec![Some("2027-01-31"); 2],
            vec![Some("2022"); 2],
        );
        assert_eq!(
            transformar_camara_legislaturas(&duplicada),
            Err(ErroCamaraLegislaturasSilver::IdDuplicado { linha: 1 })
        );
        let invertida = lote(
            vec![Some("57")],
            vec![Some("2027-01-31")],
            vec![Some("2023-02-01")],
            vec![Some("2022")],
        );
        assert_eq!(
            transformar_camara_legislaturas(&invertida),
            Err(ErroCamaraLegislaturasSilver::Valor { linha: 0 })
        );
    }
}
