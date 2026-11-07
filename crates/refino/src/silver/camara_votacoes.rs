//! Contratos de chave dos sete conjuntos anuais de votações da Câmara.

#[cfg(test)]
use std::collections::HashSet;

#[cfg(test)]
use arrow::array::{Array, StringArray};
#[cfg(test)]
use arrow::datatypes::DataType;
#[cfg(test)]
use arrow::record_batch::RecordBatch;
#[cfg(test)]
use thiserror::Error;

/// Conjunto anual presente no recurso Bronze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConjuntoCamaraVotacoes {
    /// Cadastro das proposições.
    Proposicoes,
    /// Autores de proposições.
    ProposicoesAutores,
    /// Eventos de votação.
    Votacoes,
    /// Votos individuais.
    VotacoesVotos,
    /// Objetos possíveis das votações.
    VotacoesObjetos,
    /// Proposições afetadas pelas votações.
    VotacoesProposicoes,
    /// Orientações de bancada.
    VotacoesOrientacoes,
}

impl ConjuntoCamaraVotacoes {
    /// Resolve o conjunto a partir do caminho do recurso Bronze.
    pub fn de_recurso(recurso: &str) -> Option<Self> {
        let segmento = recurso.strip_prefix("votacoes/")?.split('/').next()?;
        Some(match segmento {
            "proposicoes" => Self::Proposicoes,
            "proposicoesAutores" => Self::ProposicoesAutores,
            "votacoes" => Self::Votacoes,
            "votacoesVotos" => Self::VotacoesVotos,
            "votacoesObjetos" => Self::VotacoesObjetos,
            "votacoesProposicoes" => Self::VotacoesProposicoes,
            "votacoesOrientacoes" => Self::VotacoesOrientacoes,
            _ => return None,
        })
    }

    /// Colunas da chave candidata validada para o recorte de 2024.
    pub fn chave(self) -> &'static [&'static str] {
        match self {
            Self::Proposicoes | Self::Votacoes => &["id"],
            Self::ProposicoesAutores => &["idProposicao", "ordemAssinatura"],
            Self::VotacoesVotos => &["idVotacao", "deputado_id"],
            Self::VotacoesObjetos | Self::VotacoesProposicoes => &["idVotacao", "proposicao_id"],
            Self::VotacoesOrientacoes => &["idVotacao", "siglaBancada"],
        }
    }

    /// Segmento usado na versão da regra de qualidade.
    pub fn nome(self) -> &'static str {
        match self {
            Self::Proposicoes => "proposicoes",
            Self::ProposicoesAutores => "proposicoes-autores",
            Self::Votacoes => "votacoes",
            Self::VotacoesVotos => "votacoes-votos",
            Self::VotacoesObjetos => "votacoes-objetos",
            Self::VotacoesProposicoes => "votacoes-proposicoes",
            Self::VotacoesOrientacoes => "votacoes-orientacoes",
        }
    }
}

/// Erro de validação de um lote Bronze da Câmara.
#[cfg(test)]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroCamaraVotacoesSilver {
    /// Coluna de chave ausente ou tipo Bronze diferente de texto.
    #[error("schema Bronze incompatível com o conjunto {conjunto}")]
    Schema {
        /// Nome do conjunto anual.
        conjunto: String,
    },
    /// A chave está nula ou vazia.
    #[error("chave nula ou vazia na linha {linha} do conjunto {conjunto}")]
    ChaveInvalida {
        /// Nome do conjunto anual.
        conjunto: String,
        /// Índice da linha no lote, começando em zero.
        linha: usize,
    },
    /// A chave se repetiu no mesmo recurso anual.
    #[error("chave duplicada na linha {linha} do conjunto {conjunto}")]
    ChaveDuplicada {
        /// Nome do conjunto anual.
        conjunto: String,
        /// Índice da linha no lote, começando em zero.
        linha: usize,
    },
}

/// Valida as chaves sem colapsar eventos nem descartar atributos da origem.
#[cfg(test)]
pub fn validar_camara_votacoes(
    conjunto: ConjuntoCamaraVotacoes,
    lote: &RecordBatch,
    chaves_vistas: &mut HashSet<Vec<String>>,
) -> Result<RecordBatch, ErroCamaraVotacoesSilver> {
    let nome = conjunto.nome().to_owned();
    let indices = conjunto
        .chave()
        .iter()
        .map(|campo| {
            let indice =
                lote.schema()
                    .index_of(campo)
                    .map_err(|_| ErroCamaraVotacoesSilver::Schema {
                        conjunto: nome.clone(),
                    })?;
            if lote.schema().field(indice).data_type() != &DataType::Utf8 {
                return Err(ErroCamaraVotacoesSilver::Schema {
                    conjunto: nome.clone(),
                });
            }
            lote.column(indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .map(|_| indice)
                .ok_or_else(|| ErroCamaraVotacoesSilver::Schema {
                    conjunto: nome.clone(),
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    for linha in 0..lote.num_rows() {
        let mut chave = Vec::with_capacity(indices.len());
        let mut chave_incompleta = false;
        for &indice in &indices {
            let valores = lote
                .column(indice)
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("column type checked above");
            if valores.is_null(linha) || valores.value(linha).trim().is_empty() {
                if conjunto == ConjuntoCamaraVotacoes::VotacoesVotos {
                    chave_incompleta = true;
                    break;
                } else {
                    return Err(ErroCamaraVotacoesSilver::ChaveInvalida {
                        conjunto: nome,
                        linha,
                    });
                }
            }
            chave.push(valores.value(linha).to_owned());
        }
        if !chave_incompleta && !chaves_vistas.insert(chave) {
            return Err(ErroCamaraVotacoesSilver::ChaveDuplicada {
                conjunto: nome,
                linha,
            });
        }
    }
    Ok(lote.clone())
}

#[cfg(test)]
mod testes {
    use super::*;
    use arrow::array::StringArray;
    use std::sync::Arc;

    #[test]
    fn resolve_todos_os_sete_conjuntos_e_suas_chaves() {
        let esperados = [
            ("proposicoes", &["id"][..]),
            (
                "proposicoesAutores",
                &["idProposicao", "ordemAssinatura"][..],
            ),
            ("votacoes", &["id"][..]),
            ("votacoesVotos", &["idVotacao", "deputado_id"][..]),
            ("votacoesObjetos", &["idVotacao", "proposicao_id"][..]),
            ("votacoesProposicoes", &["idVotacao", "proposicao_id"][..]),
            ("votacoesOrientacoes", &["idVotacao", "siglaBancada"][..]),
        ];
        for (nome, chave) in esperados {
            let conjunto =
                ConjuntoCamaraVotacoes::de_recurso(&format!("votacoes/{nome}/ano=2024")).unwrap();
            assert_eq!(conjunto.chave(), chave);
        }
        assert!(ConjuntoCamaraVotacoes::de_recurso("votacoes/desconhecido/ano=2024").is_none());
    }

    #[test]
    fn preserva_linhas_e_detecta_repeticao_entre_lotes() {
        let lote = RecordBatch::try_from_iter(vec![
            ("idVotacao", Arc::new(StringArray::from(vec!["V1"])) as _),
            ("deputado_id", Arc::new(StringArray::from(vec!["D1"])) as _),
            ("tipoVoto", Arc::new(StringArray::from(vec!["Sim"])) as _),
        ])
        .unwrap();
        let conjunto = ConjuntoCamaraVotacoes::VotacoesVotos;
        let mut vistos = HashSet::new();
        let saida = validar_camara_votacoes(conjunto, &lote, &mut vistos).unwrap();
        assert_eq!(saida, lote);
        assert_eq!(
            validar_camara_votacoes(conjunto, &lote, &mut vistos),
            Err(ErroCamaraVotacoesSilver::ChaveDuplicada {
                conjunto: "votacoes-votos".into(),
                linha: 0,
            })
        );
    }

    #[test]
    fn preserva_voto_com_chave_candidata_incompleta_sem_deduplicar() {
        let lote = RecordBatch::try_from_iter(vec![
            (
                "idVotacao",
                Arc::new(StringArray::from(vec!["V1", "V1"])) as _,
            ),
            (
                "deputado_id",
                Arc::new(StringArray::from(vec!["D1", ""])) as _,
            ),
            (
                "tipoVoto",
                Arc::new(StringArray::from(vec!["Sim", "Abstenção"])) as _,
            ),
        ])
        .unwrap();

        let resultado = validar_camara_votacoes(
            ConjuntoCamaraVotacoes::VotacoesVotos,
            &lote,
            &mut HashSet::new(),
        )
        .unwrap();

        assert_eq!(resultado, lote);
        assert_eq!(resultado.num_rows(), 2);
    }
}
