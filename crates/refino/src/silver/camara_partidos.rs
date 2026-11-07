//! Contrato Silver para o cadastro de partidos da Câmara dos Deputados.

#[cfg(test)]
use std::collections::HashSet;
use std::sync::Arc;

#[cfg(test)]
use arrow::array::{Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
#[cfg(test)]
use thiserror::Error;

/// Erros ao interpretar o Bronze de partidos da Câmara.
#[cfg(test)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroCamaraPartidosSilver {
    /// O Bronze não contém as colunas esperadas como texto.
    #[error("schema Bronze de partidos da Câmara incompatível")]
    Schema,
    /// Uma linha não tem um identificador de partido não vazio.
    #[error("identificador de partido inválido na linha {linha} do lote")]
    Id {
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
    /// O cadastro contém mais de uma linha para o mesmo ID.
    #[error("identificador de partido duplicado na linha {linha} do lote")]
    IdDuplicado {
        /// Índice da segunda ocorrência dentro do lote, começando em zero.
        linha: usize,
    },
}

/// Schema versionado do contrato `camara.partido.v1`.
pub fn esquema_camara_partidos() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("sigla", DataType::Utf8, true),
        Field::new("nome", DataType::Utf8, true),
        Field::new("uri", DataType::Utf8, true),
    ]))
}

/// Valida e projeta o cadastro, preservando os identificadores como texto.
#[cfg(test)]
pub fn transformar_camara_partidos(
    lote: &RecordBatch,
) -> Result<RecordBatch, ErroCamaraPartidosSilver> {
    let schema = lote.schema();
    let indices: Vec<usize> = ["id", "sigla", "nome", "uri"]
        .iter()
        .map(|nome| {
            schema
                .index_of(nome)
                .map_err(|_| ErroCamaraPartidosSilver::Schema)
        })
        .collect::<Result<_, _>>()?;
    if indices
        .iter()
        .any(|indice| schema.field(*indice).data_type() != &DataType::Utf8)
    {
        return Err(ErroCamaraPartidosSilver::Schema);
    }
    let arrays: Vec<&StringArray> = indices
        .iter()
        .map(|indice| {
            lote.column(*indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroCamaraPartidosSilver::Schema)
        })
        .collect::<Result<_, _>>()?;
    let id = arrays[0];
    let mut ids = HashSet::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        if id.is_null(linha) || id.value(linha).trim().is_empty() {
            return Err(ErroCamaraPartidosSilver::Id { linha });
        }
        if !ids.insert(id.value(linha)) {
            return Err(ErroCamaraPartidosSilver::IdDuplicado { linha });
        }
    }
    let saida: Vec<Arc<dyn Array>> = arrays
        .iter()
        .map(|array| Arc::new((*array).clone()) as Arc<dyn Array>)
        .collect();
    RecordBatch::try_new(esquema_camara_partidos(), saida)
        .map_err(|_| ErroCamaraPartidosSilver::Schema)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn projeta_partido_preservando_id_textual_e_campos_de_origem() {
        let bronze = RecordBatch::try_from_iter(vec![
            ("id", Arc::new(StringArray::from(vec!["1"])) as _),
            ("sigla", Arc::new(StringArray::from(vec!["XYZ"])) as _),
            ("nome", Arc::new(StringArray::from(vec!["Partido X"])) as _),
            (
                "uri",
                Arc::new(StringArray::from(vec!["https://example.test/partidos/1"])) as _,
            ),
        ])
        .unwrap();
        let silver = transformar_camara_partidos(&bronze).unwrap();
        assert_eq!(silver.schema(), esquema_camara_partidos());
        assert_eq!(
            silver
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "1"
        );
        assert_eq!(silver.num_rows(), 1);
    }

    #[test]
    fn rejeita_id_ausente_ou_vazio() {
        for id in [None, Some("")] {
            let bronze = RecordBatch::try_from_iter(vec![
                ("id", Arc::new(StringArray::from(vec![id])) as _),
                ("sigla", Arc::new(StringArray::from(vec![Some("X")])) as _),
                (
                    "nome",
                    Arc::new(StringArray::from(vec![Some("Partido X")])) as _,
                ),
                ("uri", Arc::new(StringArray::from(vec![None::<&str>])) as _),
            ])
            .unwrap();
            assert_eq!(
                transformar_camara_partidos(&bronze),
                Err(ErroCamaraPartidosSilver::Id { linha: 0 })
            );
        }
    }

    #[test]
    fn rejeita_ids_duplicados_no_mesmo_recurso() {
        let bronze = RecordBatch::try_from_iter(vec![
            ("id", Arc::new(StringArray::from(vec!["1", "1"])) as _),
            ("sigla", Arc::new(StringArray::from(vec!["X", "X"])) as _),
            (
                "nome",
                Arc::new(StringArray::from(vec!["Partido X", "Partido X"])) as _,
            ),
            (
                "uri",
                Arc::new(StringArray::from(vec![None::<&str>, None])) as _,
            ),
        ])
        .unwrap();
        assert_eq!(
            transformar_camara_partidos(&bronze),
            Err(ErroCamaraPartidosSilver::IdDuplicado { linha: 1 })
        );
    }
}
