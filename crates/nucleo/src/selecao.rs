//! Escolha do que ingerir: qualquer fonte que etiquete suas tarefas pode ser filtrada,
//! sem código específico (ver `docs/adr/0010`).

use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::tarefa::TarefaPlanejada;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroDeSelecao {
    #[error("filtro inválido: {0}. Use o formato ano=2022,2024")]
    FormatoInvalido(String),

    #[error("filtro desconhecido: {filtro}. Disponíveis: {disponiveis}")]
    FiltroDesconhecido { filtro: String, disponiveis: String },

    #[error("valor desconhecido para {filtro}: {valor}. Disponíveis: {disponiveis}")]
    ValorDesconhecido {
        filtro: String,
        valor: String,
        disponiveis: String,
    },
}

/// Filtros por etiqueta: cada filtro aceita um conjunto de valores.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FiltroDeTarefas {
    valores: BTreeMap<String, BTreeSet<String>>,
}

impl FiltroDeTarefas {
    /// Interpreta textos no formato `ano=2022,2024`.
    pub fn a_partir_de(textos: &[String]) -> Result<Self, ErroDeSelecao> {
        let mut filtro = Self::default();
        for texto in textos {
            let (nome, valores) = texto
                .split_once('=')
                .ok_or_else(|| ErroDeSelecao::FormatoInvalido(texto.clone()))?;
            let valores: Vec<&str> = valores
                .split(',')
                .map(str::trim)
                .filter(|valor| !valor.is_empty())
                .collect();
            if nome.trim().is_empty() || valores.is_empty() {
                return Err(ErroDeSelecao::FormatoInvalido(texto.clone()));
            }
            filtro.restringir(nome.trim(), valores);
        }
        Ok(filtro)
    }

    /// Limita `nome` aos valores informados. Valores vazios significam "sem restrição".
    pub fn restringir<'a>(&mut self, nome: &str, valores: impl IntoIterator<Item = &'a str>) {
        let valores: BTreeSet<String> = valores.into_iter().map(str::to_owned).collect();
        if !valores.is_empty() {
            self.valores
                .entry(nome.to_owned())
                .or_default()
                .extend(valores);
        }
    }

    /// Mantém só as tarefas aceitas pelo filtro. Dependências de tarefas que ficaram de
    /// fora são removidas, para que o restante ainda forme uma DAG válida.
    pub fn aplicar(
        &self,
        tarefas: Vec<TarefaPlanejada>,
    ) -> Result<Vec<TarefaPlanejada>, ErroDeSelecao> {
        self.validar(&tarefas)?;

        let mut mantidas: Vec<TarefaPlanejada> = tarefas
            .into_iter()
            .filter(|tarefa| self.aceita(tarefa))
            .collect();

        let nomes: BTreeSet<String> = mantidas
            .iter()
            .map(|tarefa| tarefa.tarefa.nome().to_owned())
            .collect();
        for tarefa in &mut mantidas {
            tarefa
                .depende_de
                .retain(|dependencia| nomes.contains(dependencia));
        }
        Ok(mantidas)
    }

    fn aceita(&self, tarefa: &TarefaPlanejada) -> bool {
        self.valores.iter().all(|(nome, aceitos)| {
            tarefa
                .etiquetas
                .get(nome)
                .is_some_and(|valor| aceitos.contains(valor))
        })
    }

    fn validar(&self, tarefas: &[TarefaPlanejada]) -> Result<(), ErroDeSelecao> {
        let disponiveis = dimensoes(tarefas);
        for (nome, aceitos) in &self.valores {
            let Some(valores) = disponiveis.get(nome) else {
                let nomes: Vec<&str> = disponiveis.keys().map(String::as_str).collect();
                return Err(ErroDeSelecao::FiltroDesconhecido {
                    filtro: nome.clone(),
                    disponiveis: if nomes.is_empty() {
                        "nenhum".to_owned()
                    } else {
                        nomes.join(", ")
                    },
                });
            };
            if let Some(valor) = aceitos.iter().find(|valor| !valores.contains(*valor)) {
                return Err(ErroDeSelecao::ValorDesconhecido {
                    filtro: nome.clone(),
                    valor: valor.clone(),
                    disponiveis: valores.iter().cloned().collect::<Vec<_>>().join(", "),
                });
            }
        }
        Ok(())
    }
}

/// Os filtros que um conjunto de tarefas oferece, com seus valores.
pub fn dimensoes(tarefas: &[TarefaPlanejada]) -> BTreeMap<String, BTreeSet<String>> {
    let mut dimensoes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for tarefa in tarefas {
        for (nome, valor) in &tarefa.etiquetas {
            dimensoes
                .entry(nome.clone())
                .or_default()
                .insert(valor.clone());
        }
    }
    dimensoes
}

#[cfg(test)]
mod testes {
    use async_trait::async_trait;

    use super::*;
    use crate::contexto::ContextoDeExecucao;
    use crate::erros::ErroDeIngestao;
    use crate::tarefa::{ResumoDaTarefa, Tarefa};

    struct Vazia(&'static str);

    #[async_trait]
    impl Tarefa for Vazia {
        fn nome(&self) -> &str {
            self.0
        }

        async fn executar(&self, _: &ContextoDeExecucao) -> Result<ResumoDaTarefa, ErroDeIngestao> {
            Ok(ResumoDaTarefa::default())
        }
    }

    fn tarefa(nome: &'static str, ano: &str, uf: &str) -> TarefaPlanejada {
        TarefaPlanejada::nova(Vazia(nome)).com_etiquetas([("ano", ano), ("uf", uf)])
    }

    fn exemplo() -> Vec<TarefaPlanejada> {
        vec![
            tarefa("a", "2022", "AC"),
            tarefa("b", "2022", "SP"),
            tarefa("c", "2024", "AC"),
        ]
    }

    fn nomes(filtros: &[&str]) -> Result<Vec<String>, ErroDeSelecao> {
        let textos: Vec<String> = filtros.iter().map(|f| (*f).to_owned()).collect();
        Ok(FiltroDeTarefas::a_partir_de(&textos)?
            .aplicar(exemplo())?
            .iter()
            .map(|t| t.tarefa.nome().to_owned())
            .collect())
    }

    #[test]
    fn sem_filtro_mantem_tudo() {
        assert_eq!(nomes(&[]).unwrap(), ["a", "b", "c"]);
    }

    #[test]
    fn combina_filtros_diferentes_e_valores_do_mesmo_filtro() {
        assert_eq!(nomes(&["uf=AC"]).unwrap(), ["a", "c"]);
        assert_eq!(nomes(&["ano=2022", "uf=AC"]).unwrap(), ["a"]);
        assert_eq!(nomes(&["ano=2022,2024", "uf=SP"]).unwrap(), ["b"]);
    }

    #[test]
    fn avisa_sobre_filtro_ou_valor_errado_em_vez_de_ignorar() {
        assert!(matches!(
            nomes(&["cidade=X"]),
            Err(ErroDeSelecao::FiltroDesconhecido { .. })
        ));
        assert!(matches!(
            nomes(&["ano=1990"]),
            Err(ErroDeSelecao::ValorDesconhecido { .. })
        ));
        assert!(matches!(
            nomes(&["ano"]),
            Err(ErroDeSelecao::FormatoInvalido(_))
        ));
        assert!(matches!(
            nomes(&["ano="]),
            Err(ErroDeSelecao::FormatoInvalido(_))
        ));
    }

    #[test]
    fn remove_dependencias_de_tarefas_que_ficaram_de_fora() {
        let tarefas = vec![
            tarefa("base", "2022", "AC"),
            tarefa("filha", "2024", "AC").depois_de(["base"]),
        ];
        let mut filtro = FiltroDeTarefas::default();
        filtro.restringir("ano", ["2024"]);

        let mantidas = filtro.aplicar(tarefas).unwrap();

        assert_eq!(mantidas.len(), 1);
        assert!(mantidas[0].depende_de.is_empty());
    }
}
