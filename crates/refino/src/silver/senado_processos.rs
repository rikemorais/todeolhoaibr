//! Contrato Silver para o catálogo anual de processos legislativos do Senado.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema};
#[cfg(test)]
use arrow::{
    array::{Array, StringArray, UInt16Array},
    record_batch::RecordBatch,
};
#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use thiserror::Error;

/// Erros ao validar o catálogo anual de processos do Senado.
#[cfg(test)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroSenadoProcessosSilver {
    /// O Bronze não tem schema textual com `id`.
    #[error("schema Bronze de processos incompatível")]
    Schema,
    /// A linha não tem ID de processo.
    #[error("ID de processo inválido na linha {linha} do lote")]
    Identidade {
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
    /// O ID já apareceu em uma parte anterior do mesmo recurso anual.
    #[error("ID de processo duplicado na linha {linha} do lote")]
    Duplicada {
        /// Índice da linha duplicada dentro do lote, começando em zero.
        linha: usize,
    },
    /// Não foi possível serializar os campos da origem em JSON.
    #[error("não foi possível preservar os campos originais do processo")]
    Json,
}

/// Schema estável do contrato `senado.processo.v1`.
pub fn esquema_senado_processos() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("ano", DataType::UInt16, false),
        Field::new("processo_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

/// Preserva cada processo da listagem anual, tipando ID e ano sem inferir histórico.
#[cfg(test)]
pub fn transformar_senado_processos(
    lote: &RecordBatch,
    ano: u16,
    ids_vistos: &mut HashSet<String>,
) -> Result<RecordBatch, ErroSenadoProcessosSilver> {
    let schema = lote.schema();
    let indice_id = schema
        .index_of("id")
        .map_err(|_| ErroSenadoProcessosSilver::Schema)?;
    if schema
        .fields()
        .iter()
        .any(|campo| campo.data_type() != &DataType::Utf8)
    {
        return Err(ErroSenadoProcessosSilver::Schema);
    }
    let ids = lote
        .column(indice_id)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or(ErroSenadoProcessosSilver::Schema)?;

    let mut ids_silver = Vec::with_capacity(lote.num_rows());
    let mut anos_silver = Vec::with_capacity(lote.num_rows());
    let mut originais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        if ids.is_null(linha) || ids.value(linha).trim().is_empty() {
            return Err(ErroSenadoProcessosSilver::Identidade { linha });
        }
        let id = ids.value(linha).trim();
        if !ids_vistos.insert(id.to_owned()) {
            return Err(ErroSenadoProcessosSilver::Duplicada { linha });
        }

        let mut original = serde_json::Map::new();
        for (indice, campo) in schema.fields().iter().enumerate() {
            let valores = lote
                .column(indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroSenadoProcessosSilver::Schema)?;
            let valor = if valores.is_null(linha) {
                serde_json::Value::Null
            } else {
                valor_original(valores.value(linha))
            };
            original.insert(campo.name().clone(), valor);
        }
        ids_silver.push(id.to_owned());
        anos_silver.push(ano);
        originais.push(Some(
            serde_json::to_string(&original).map_err(|_| ErroSenadoProcessosSilver::Json)?,
        ));
    }

    let mut saida: Vec<Arc<dyn arrow::array::Array>> = vec![
        Arc::new(StringArray::from(ids_silver)),
        Arc::new(UInt16Array::from(anos_silver)),
        Arc::new(StringArray::from(originais)),
    ];
    saida.extend(crate::executor_sql::colunas_linhagem_teste(lote.num_rows()));
    RecordBatch::try_new(esquema_senado_processos(), saida)
        .map_err(|_| ErroSenadoProcessosSilver::Schema)
}

#[cfg(test)]
fn valor_original(texto: &str) -> serde_json::Value {
    let aparado = texto.trim_start();
    if aparado.starts_with('{') || aparado.starts_with('[') {
        serde_json::from_str(texto).unwrap_or_else(|_| serde_json::Value::String(texto.to_owned()))
    } else {
        serde_json::Value::String(texto.to_owned())
    }
}

#[cfg(test)]
mod testes {
    use std::collections::HashSet;
    use std::sync::Arc;

    use arrow::array::{Array, StringArray, UInt16Array};
    use arrow::datatypes::DataType;
    use arrow::record_batch::RecordBatch;

    use super::*;

    fn lote(ids: Vec<Option<&str>>) -> RecordBatch {
        RecordBatch::try_from_iter(vec![
            ("id", Arc::new(StringArray::from(ids)) as _),
            (
                "ementa",
                Arc::new(StringArray::from(vec![Some("Proposta")])) as _,
            ),
        ])
        .unwrap()
    }

    #[test]
    fn tipa_id_e_ano_e_preserva_campos_de_origem() {
        let bronze = lote(vec![Some("00825")]);
        let silver = transformar_senado_processos(&bronze, 2024, &mut HashSet::new()).unwrap();
        assert_eq!(silver.schema(), esquema_senado_processos());
        assert_eq!(silver.schema().field(0).data_type(), &DataType::Utf8);
        assert_eq!(silver.schema().field(1).data_type(), &DataType::UInt16);
        assert_eq!(
            silver
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "00825"
        );
        assert_eq!(
            silver
                .column(1)
                .as_any()
                .downcast_ref::<UInt16Array>()
                .unwrap()
                .value(0),
            2024
        );
        let original = silver
            .column(2)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        let original: serde_json::Value = serde_json::from_str(original.value(0)).unwrap();
        assert_eq!(original["id"], "00825");
        assert_eq!(original["ementa"], "Proposta");
    }

    #[test]
    fn rejeita_id_ausente_vazio_e_duplicado() {
        let mut vistos = HashSet::new();
        assert!(matches!(
            transformar_senado_processos(&lote(vec![None]), 2024, &mut vistos),
            Err(ErroSenadoProcessosSilver::Identidade { linha: 0 })
        ));
        assert!(matches!(
            transformar_senado_processos(&lote(vec![Some(" ")]), 2024, &mut vistos),
            Err(ErroSenadoProcessosSilver::Identidade { linha: 0 })
        ));
        transformar_senado_processos(&lote(vec![Some("P1")]), 2024, &mut vistos).unwrap();
        assert!(matches!(
            transformar_senado_processos(&lote(vec![Some("P1")]), 2024, &mut vistos),
            Err(ErroSenadoProcessosSilver::Duplicada { linha: 0 })
        ));
    }
}
