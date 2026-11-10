//! Contrato Silver de parlamentar por legislatura para o Senado Federal.

use std::sync::Arc;

#[cfg(test)]
use arrow::array::{Array, StringArray, UInt16Array};
use arrow::datatypes::{DataType, Field, Schema};
#[cfg(test)]
use arrow::record_batch::RecordBatch;
#[cfg(test)]
use serde_json::Value;
#[cfg(test)]
use thiserror::Error;

/// Erros ao interpretar a identificação parlamentar do Bronze do Senado.
#[cfg(test)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroSenadoParlamentaresSilver {
    /// O Bronze não contém as colunas de identificação esperadas.
    #[error("schema Bronze de parlamentares incompatível")]
    Schema,
    /// Uma linha não contém código parlamentar válido no objeto de identificação.
    #[error("código parlamentar inválido na linha {linha} do lote")]
    Codigo {
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
}

/// Schema versionado do contrato `senado.parlamentar.legislatura.v1`.
pub fn esquema_senado_parlamentares() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("codigo_parlamentar", DataType::Utf8, false),
        Field::new("legislatura", DataType::UInt16, false),
        Field::new("identificacao_original", DataType::Utf8, false),
        Field::new("mandato_original", DataType::Utf8, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

/// Extrai o código e tipa o escopo temporal, preservando os objetos JSON originais.
#[cfg(test)]
pub fn transformar_senado_parlamentares(
    lote: &RecordBatch,
    legislatura: u16,
) -> Result<RecordBatch, ErroSenadoParlamentaresSilver> {
    let schema = lote.schema();
    let indice_identificacao = schema
        .index_of("IdentificacaoParlamentar")
        .map_err(|_| ErroSenadoParlamentaresSilver::Schema)?;
    let campo_identificacao = schema.field(indice_identificacao);
    if campo_identificacao.data_type() != &DataType::Utf8 {
        return Err(ErroSenadoParlamentaresSilver::Schema);
    }
    let identificacoes = lote
        .column(indice_identificacao)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or(ErroSenadoParlamentaresSilver::Schema)?;
    let indice_mandato = schema.index_of("Mandato").ok();
    if indice_mandato.is_some_and(|indice| schema.field(indice).data_type() != &DataType::Utf8) {
        return Err(ErroSenadoParlamentaresSilver::Schema);
    }
    let mandatos = indice_mandato.map(|indice| {
        lote.column(indice)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or(ErroSenadoParlamentaresSilver::Schema)
    });
    let mandatos = mandatos.transpose()?;

    let mut codigos = Vec::with_capacity(lote.num_rows());
    let mut legislaturas = Vec::with_capacity(lote.num_rows());
    let mut identificacoes_originais = Vec::with_capacity(lote.num_rows());
    let mut mandatos_originais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        if identificacoes.is_null(linha) {
            return Err(ErroSenadoParlamentaresSilver::Codigo { linha });
        }
        let identificacao_json: Value = serde_json::from_str(identificacoes.value(linha))
            .map_err(|_| ErroSenadoParlamentaresSilver::Codigo { linha })?;
        let codigo = identificacao_json
            .get("CodigoParlamentar")
            .and_then(texto_de_codigo)
            .ok_or(ErroSenadoParlamentaresSilver::Codigo { linha })?;
        codigos.push(codigo);
        legislaturas.push(legislatura);
        identificacoes_originais.push(Some(identificacoes.value(linha)));
        mandatos_originais
            .push(mandatos.and_then(|array| (!array.is_null(linha)).then(|| array.value(linha))));
    }
    let mut saida: Vec<Arc<dyn arrow::array::Array>> = vec![
        Arc::new(StringArray::from(codigos)),
        Arc::new(UInt16Array::from(legislaturas)),
        Arc::new(StringArray::from(identificacoes_originais)),
        Arc::new(StringArray::from(mandatos_originais)),
    ];
    saida.extend(crate::executor_sql::colunas_linhagem_teste(lote.num_rows()));
    RecordBatch::try_new(esquema_senado_parlamentares(), saida)
        .map_err(|_| ErroSenadoParlamentaresSilver::Schema)
}

#[cfg(test)]
fn texto_de_codigo(valor: &Value) -> Option<String> {
    match valor {
        Value::String(texto) if !texto.is_empty() => Some(texto.clone()),
        Value::Number(numero) => Some(numero.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn lote(identificacoes: Vec<Option<&str>>, mandatos: Vec<Option<&str>>) -> RecordBatch {
        RecordBatch::try_from_iter(vec![
            (
                "IdentificacaoParlamentar",
                Arc::new(StringArray::from(identificacoes)) as _,
            ),
            ("Mandato", Arc::new(StringArray::from(mandatos)) as _),
        ])
        .unwrap()
    }

    #[test]
    fn extrai_chave_e_preserva_identificacao_e_mandato() {
        let bronze = lote(
            vec![Some(r#"{"CodigoParlamentar":123,"NomeParlamentar":"A"}"#)],
            vec![Some(r#"[{"UfParlamentar":"AC"}]"#)],
        );
        let silver = transformar_senado_parlamentares(&bronze, 57).unwrap();
        assert_eq!(silver.schema(), esquema_senado_parlamentares());
        assert_eq!(
            silver
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "123"
        );
        assert_eq!(
            silver
                .column(1)
                .as_any()
                .downcast_ref::<UInt16Array>()
                .unwrap()
                .value(0),
            57
        );
        assert_eq!(
            silver
                .column(3)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            r#"[{"UfParlamentar":"AC"}]"#
        );
    }

    #[test]
    fn aceita_codigo_textual_e_mandato_nulo() {
        let bronze = lote(vec![Some(r#"{"CodigoParlamentar":"00825"}"#)], vec![None]);
        let silver = transformar_senado_parlamentares(&bronze, 55).unwrap();
        assert_eq!(
            silver
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "00825"
        );
        assert!(silver.column(3).is_null(0));
    }

    #[test]
    fn rejeita_identificacao_sem_codigo() {
        let bronze = lote(vec![Some(r#"{"NomeParlamentar":"A"}"#)], vec![None]);
        assert_eq!(
            transformar_senado_parlamentares(&bronze, 57),
            Err(ErroSenadoParlamentaresSilver::Codigo { linha: 0 })
        );
    }
}
