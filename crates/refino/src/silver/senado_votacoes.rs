//! Contrato de chave para as votações nominais do Senado.

use std::collections::HashSet;

use arrow::array::{Array, StringArray};
use arrow::datatypes::DataType;
use arrow::record_batch::RecordBatch;
use thiserror::Error;

/// Erros ao validar a chave de uma votação do Senado.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroSenadoVotacoesSilver {
    /// O Bronze não contém a coluna identificadora textual esperada.
    #[error("schema Bronze de votações do Senado incompatível")]
    Schema,
    /// Código da votação nulo ou vazio.
    #[error("código de sessão de votação inválido na linha {linha} do lote")]
    Codigo {
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
    /// A chave da votação apareceu mais de uma vez no recurso anual.
    #[error("código de sessão de votação duplicado na linha {linha} do lote")]
    Duplicado {
        /// Índice da segunda ocorrência dentro do lote, começando em zero.
        linha: usize,
    },
}

/// Valida `codigoSessaoVotacao` e mantém o registro integral, incluindo `votos` aninhados.
pub fn transformar_senado_votacoes(
    lote: &RecordBatch,
    codigos_vistos: &mut HashSet<String>,
) -> Result<RecordBatch, ErroSenadoVotacoesSilver> {
    let indice = lote
        .schema()
        .index_of("codigoSessaoVotacao")
        .map_err(|_| ErroSenadoVotacoesSilver::Schema)?;
    if lote.schema().field(indice).data_type() != &DataType::Utf8 {
        return Err(ErroSenadoVotacoesSilver::Schema);
    }
    let codigos = lote
        .column(indice)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or(ErroSenadoVotacoesSilver::Schema)?;
    for linha in 0..lote.num_rows() {
        if codigos.is_null(linha) || codigos.value(linha).trim().is_empty() {
            return Err(ErroSenadoVotacoesSilver::Codigo { linha });
        }
        if !codigos_vistos.insert(codigos.value(linha).to_owned()) {
            return Err(ErroSenadoVotacoesSilver::Duplicado { linha });
        }
    }
    Ok(lote.clone())
}

#[cfg(test)]
mod testes {
    use std::sync::Arc;

    use arrow::array::StringArray;

    use super::*;

    fn lote(codigos: Vec<Option<&str>>) -> RecordBatch {
        let quantidade = codigos.len();
        RecordBatch::try_from_iter(vec![
            (
                "codigoSessaoVotacao",
                Arc::new(StringArray::from(codigos)) as _,
            ),
            (
                "dataSessao",
                Arc::new(StringArray::from(vec![Some("2024-01-01"); quantidade])) as _,
            ),
            (
                "votos",
                Arc::new(StringArray::from(vec![
                    Some(
                        r#"[{"codigoParlamentar":"1","voto":"Sim"}]"#
                    );
                    quantidade
                ])) as _,
            ),
        ])
        .unwrap()
    }

    #[test]
    fn preserva_toda_a_sessao_e_a_lista_aninhada_de_votos() {
        let bronze = lote(vec![Some("98765")]);
        let saida = transformar_senado_votacoes(&bronze, &mut HashSet::new()).unwrap();
        assert_eq!(saida, bronze);
        assert_eq!(saida.num_rows(), 1);
        assert_eq!(saida.schema().field(2).name(), "votos");
    }

    #[test]
    fn rejeita_codigo_ausente_vazio_e_repetido() {
        assert_eq!(
            transformar_senado_votacoes(&lote(vec![None]), &mut HashSet::new()),
            Err(ErroSenadoVotacoesSilver::Codigo { linha: 0 })
        );
        assert_eq!(
            transformar_senado_votacoes(&lote(vec![Some(" ")]), &mut HashSet::new()),
            Err(ErroSenadoVotacoesSilver::Codigo { linha: 0 })
        );
        let mut vistos = HashSet::new();
        assert!(transformar_senado_votacoes(&lote(vec![Some("1")]), &mut vistos).is_ok());
        assert_eq!(
            transformar_senado_votacoes(&lote(vec![Some("1")]), &mut vistos),
            Err(ErroSenadoVotacoesSilver::Duplicado { linha: 0 })
        );
    }
}
