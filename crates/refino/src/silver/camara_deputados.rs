//! Contrato Silver do cadastro de deputados em exercício da Câmara.

#[cfg(test)]
use std::collections::HashSet;
use std::sync::Arc;

#[cfg(test)]
use arrow::array::{Array, RecordBatch, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
#[cfg(test)]
use thiserror::Error;

/// Erros ao validar o cadastro Bronze de deputados.
#[cfg(test)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroCamaraDeputadosSilver {
    /// Faltam colunas textuais necessárias no Bronze.
    #[error("schema Bronze de deputados da Câmara incompatível")]
    Schema,
    /// O ID ou um atributo obrigatório está ausente/vazio.
    #[error("campo obrigatório inválido na linha {linha} do lote")]
    Valor {
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
    /// O mesmo ID ocorre mais de uma vez no recurso.
    #[error("ID de deputado duplicado na linha {linha} do lote")]
    IdDuplicado {
        /// Índice da segunda ocorrência dentro do lote, começando em zero.
        linha: usize,
    },
}

/// Schema versionado do contrato `camara.deputado.exercicio.v1`.
pub fn esquema_camara_deputados() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("uri", DataType::Utf8, false),
        Field::new("nome", DataType::Utf8, false),
        Field::new("sigla_partido", DataType::Utf8, true),
        Field::new("uri_partido", DataType::Utf8, true),
        Field::new("sigla_uf", DataType::Utf8, false),
        Field::new("id_legislatura", DataType::Utf8, false),
        Field::new("url_foto", DataType::Utf8, true),
        Field::new("email", DataType::Utf8, true),
    ]))
}

/// Projeta os atributos atuais do mandato e valida a chave candidata `id`.
#[cfg(test)]
pub fn transformar_camara_deputados(
    lote: &RecordBatch,
) -> Result<RecordBatch, ErroCamaraDeputadosSilver> {
    let schema = lote.schema();
    let nomes = [
        "id",
        "uri",
        "nome",
        "siglaPartido",
        "uriPartido",
        "siglaUf",
        "idLegislatura",
        "urlFoto",
        "email",
    ];
    let indices: Vec<usize> = nomes
        .iter()
        .map(|nome| {
            schema
                .index_of(nome)
                .map_err(|_| ErroCamaraDeputadosSilver::Schema)
        })
        .collect::<Result<_, _>>()?;
    if indices
        .iter()
        .any(|indice| schema.field(*indice).data_type() != &DataType::Utf8)
    {
        return Err(ErroCamaraDeputadosSilver::Schema);
    }
    let arrays: Vec<&StringArray> = indices
        .iter()
        .map(|indice| {
            lote.column(*indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or(ErroCamaraDeputadosSilver::Schema)
        })
        .collect::<Result<_, _>>()?;
    let mut ids = HashSet::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        for indice in [0, 1, 2, 5, 6] {
            if arrays[indice].is_null(linha) || arrays[indice].value(linha).trim().is_empty() {
                return Err(ErroCamaraDeputadosSilver::Valor { linha });
            }
        }
        if !arrays[0]
            .value(linha)
            .bytes()
            .all(|byte| byte.is_ascii_digit())
        {
            return Err(ErroCamaraDeputadosSilver::Valor { linha });
        }
        if !ids.insert(arrays[0].value(linha)) {
            return Err(ErroCamaraDeputadosSilver::IdDuplicado { linha });
        }
    }
    let saida: Vec<Arc<dyn Array>> = arrays
        .iter()
        .map(|array| Arc::new((*array).clone()) as Arc<dyn Array>)
        .collect();
    RecordBatch::try_new(esquema_camara_deputados(), saida)
        .map_err(|_| ErroCamaraDeputadosSilver::Schema)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn lote(ids: Vec<Option<&str>>) -> RecordBatch {
        let tamanho = ids.len();
        let col = |nome: &'static str, valores: Vec<Option<&str>>| {
            (nome, Arc::new(StringArray::from(valores)) as _)
        };
        RecordBatch::try_from_iter(vec![
            col("id", ids),
            col(
                "uri",
                vec![Some("https://dadosabertos.camara.leg.br/api/v2/deputados/1"); tamanho],
            ),
            col("nome", vec![Some("Deputada Exemplo"); tamanho]),
            col("siglaPartido", vec![Some("ABC"); tamanho]),
            col(
                "uriPartido",
                vec![Some("https://dadosabertos.camara.leg.br/api/v2/partidos/1"); tamanho],
            ),
            col("siglaUf", vec![Some("DF"); tamanho]),
            col("idLegislatura", vec![Some("57"); tamanho]),
            col("urlFoto", vec![None; tamanho]),
            col("email", vec![Some("deputada@camara.leg.br"); tamanho]),
        ])
        .unwrap()
    }

    #[test]
    fn preserva_ids_como_texto_e_campos_de_vinculo() {
        let saida = transformar_camara_deputados(&lote(vec![Some("12345")])).unwrap();
        assert_eq!(saida.schema(), esquema_camara_deputados());
        assert_eq!(
            saida
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "12345"
        );
        assert!(saida.column(7).is_null(0));
    }

    #[test]
    fn rejeita_id_nao_numerico_e_repetido() {
        assert_eq!(
            transformar_camara_deputados(&lote(vec![Some("abc")])),
            Err(ErroCamaraDeputadosSilver::Valor { linha: 0 })
        );
        assert_eq!(
            transformar_camara_deputados(&lote(vec![Some("123"), Some("123")])),
            Err(ErroCamaraDeputadosSilver::IdDuplicado { linha: 1 })
        );
    }
}
