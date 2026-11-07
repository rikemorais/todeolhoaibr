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

/// Erros ao validar detalhes de processo legislativo do Senado.
#[cfg(test)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroSenadoProcessosDetalhesSilver {
    /// O Bronze não tem schema textual com `id` e `ano`.
    #[error("schema Bronze de processos-detalhes incompatível")]
    Schema,
    /// A linha não tem ID de processo.
    #[error("ID de processo inválido na linha {linha} do lote")]
    Identidade {
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
    /// O ano não pode ser convertido para UInt16.
    #[error("ano inválido na linha {linha} do lote")]
    Ano {
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
    /// O ID já apareceu em uma parte anterior do mesmo recurso Bronze.
    #[error("ID de processo duplicado na linha {linha} do lote")]
    Duplicada {
        /// Índice da linha duplicada dentro do lote, começando em zero.
        linha: usize,
    },
    /// Não foi possível serializar os campos da origem em JSON.
    #[error("não foi possível preservar os campos originais do processo")]
    Json,
}

/// Schema estável do contrato `senado.processo.detalhe.v1`.
pub fn esquema_senado_processos_detalhes() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("ano", DataType::UInt16, false),
        Field::new("processo_original", DataType::Utf8, false),
    ]))
}

/// Tipifica identidade e ano sem explodir ou perder campos do detalhe do processo.
#[cfg(test)]
pub fn transformar_senado_processos_detalhes(
    lote: &RecordBatch,
    ids_vistos: &mut HashSet<String>,
) -> Result<RecordBatch, ErroSenadoProcessosDetalhesSilver> {
    let schema = lote.schema();
    let indice_id = schema
        .index_of("id")
        .map_err(|_| ErroSenadoProcessosDetalhesSilver::Schema)?;
    let indice_ano = schema
        .index_of("ano")
        .map_err(|_| ErroSenadoProcessosDetalhesSilver::Schema)?;
    if schema
        .fields()
        .iter()
        .any(|campo| campo.data_type() != &DataType::Utf8)
    {
        return Err(ErroSenadoProcessosDetalhesSilver::Schema);
    }
    let ids = lote
        .column(indice_id)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or(ErroSenadoProcessosDetalhesSilver::Schema)?;
    let anos = lote
        .column(indice_ano)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or(ErroSenadoProcessosDetalhesSilver::Schema)?;

    let mut ids_silver = Vec::with_capacity(lote.num_rows());
    let mut anos_silver = Vec::with_capacity(lote.num_rows());
    let mut originais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        if ids.is_null(linha) || ids.value(linha).trim().is_empty() {
            return Err(ErroSenadoProcessosDetalhesSilver::Identidade { linha });
        }
        let id = ids.value(linha).trim();
        if !ids_vistos.insert(id.to_owned()) {
            return Err(ErroSenadoProcessosDetalhesSilver::Duplicada { linha });
        }
        if anos.is_null(linha) {
            return Err(ErroSenadoProcessosDetalhesSilver::Ano { linha });
        }
        let ano_texto = anos.value(linha).trim();
        if ano_texto.len() != 4 || !ano_texto.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ErroSenadoProcessosDetalhesSilver::Ano { linha });
        }
        let ano = ano_texto
            .parse::<u16>()
            .map_err(|_| ErroSenadoProcessosDetalhesSilver::Ano { linha })?;

        let mut original = serde_json::Map::new();
        for (indice, campo) in schema.fields().iter().enumerate() {
            let valores = lote
                .column(indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroSenadoProcessosDetalhesSilver::Schema)?;
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
            serde_json::to_string(&original)
                .map_err(|_| ErroSenadoProcessosDetalhesSilver::Json)?,
        ));
    }

    RecordBatch::try_new(
        esquema_senado_processos_detalhes(),
        vec![
            Arc::new(StringArray::from(ids_silver)),
            Arc::new(UInt16Array::from(anos_silver)),
            Arc::new(StringArray::from(originais)),
        ],
    )
    .map_err(|_| ErroSenadoProcessosDetalhesSilver::Schema)
}

#[cfg(test)]
fn valor_original(texto: &str) -> serde_json::Value {
    let texto_aparado = texto.trim_start();
    if texto_aparado.starts_with('{') || texto_aparado.starts_with('[') {
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

    fn lote(ids: Vec<Option<&str>>, anos: Vec<Option<&str>>) -> RecordBatch {
        let despachos = vec![Some(r#"[{"ordem":1}]"#); ids.len()];
        RecordBatch::try_from_iter(vec![
            ("id", Arc::new(StringArray::from(ids)) as _),
            ("ano", Arc::new(StringArray::from(anos)) as _),
            ("despachos", Arc::new(StringArray::from(despachos)) as _),
        ])
        .unwrap()
    }

    #[test]
    fn tipa_chave_e_ano_e_preserva_todos_os_campos_no_json_original() {
        let bronze = lote(vec![Some("00825")], vec![Some("2024")]);
        let silver = transformar_senado_processos_detalhes(&bronze, &mut HashSet::new()).unwrap();

        assert_eq!(silver.schema(), esquema_senado_processos_detalhes());
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
        assert_eq!(original["ano"], "2024");
        assert_eq!(original["despachos"][0]["ordem"], 1);
    }

    #[test]
    fn rejeita_chave_ausente_e_ano_nao_tipavel() {
        let mut chaves = HashSet::new();
        assert!(matches!(
            transformar_senado_processos_detalhes(
                &lote(vec![Some(" ")], vec![Some("2024")]),
                &mut chaves
            ),
            Err(ErroSenadoProcessosDetalhesSilver::Identidade { linha: 0 })
        ));
        assert!(matches!(
            transformar_senado_processos_detalhes(
                &lote(vec![Some("P1")], vec![Some("ano desconhecido")]),
                &mut chaves
            ),
            Err(ErroSenadoProcessosDetalhesSilver::Ano { linha: 0 })
        ));
    }

    #[test]
    fn rejeita_chave_repetida_entre_lotes() {
        let mut chaves = HashSet::new();
        transformar_senado_processos_detalhes(
            &lote(vec![Some("P1")], vec![Some("2024")]),
            &mut chaves,
        )
        .unwrap();
        assert!(matches!(
            transformar_senado_processos_detalhes(
                &lote(vec![Some("P1")], vec![Some("2024")]),
                &mut chaves
            ),
            Err(ErroSenadoProcessosDetalhesSilver::Duplicada { linha: 0 })
        ));
    }
}
