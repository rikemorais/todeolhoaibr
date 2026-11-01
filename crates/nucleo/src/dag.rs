//! A DAG de uma fonte: constrói, valida e consulta o grafo de tarefas. É o único
//! ponto do projeto que conhece a biblioteca Dagron (ver `docs/adr/0003`).

use std::sync::Arc;

use dagron_core::{DAG, DagronError};

use crate::erros::ErroDeDag;
use crate::tarefa::{Tarefa, TarefaPlanejada};

/// DAG validada de uma fonte. Único ponto do projeto que conhece o Dagron.
pub struct DagDeIngestao {
    fonte: String,
    grafo: DAG<Arc<dyn Tarefa>>,
}

impl DagDeIngestao {
    /// Monta o grafo, rejeitando nomes duplicados, dependências inexistentes e ciclos.
    pub fn construir(fonte: &str, tarefas: Vec<TarefaPlanejada>) -> Result<Self, ErroDeDag> {
        if tarefas.is_empty() {
            return Err(ErroDeDag::SemTarefas {
                fonte: fonte.to_owned(),
            });
        }

        let invalido = |erro: DagronError| ErroDeDag::GrafoInvalido {
            fonte: fonte.to_owned(),
            detalhe: erro.to_string(),
        };

        let mut grafo = DAG::new();
        for planejada in &tarefas {
            grafo
                .add_node(
                    planejada.tarefa.nome().to_owned(),
                    Arc::clone(&planejada.tarefa),
                )
                .map_err(invalido)?;
        }
        for planejada in &tarefas {
            for dependencia in &planejada.depende_de {
                grafo
                    .add_edge(dependencia, planejada.tarefa.nome(), None, None)
                    .map_err(invalido)?;
            }
        }

        Ok(Self {
            fonte: fonte.to_owned(),
            grafo,
        })
    }

    /// Nome da fonte à qual este grafo pertence.
    pub fn fonte(&self) -> &str {
        &self.fonte
    }

    /// Nomes das tarefas agrupados em níveis; tarefas do mesmo nível são independentes.
    pub fn niveis(&self) -> Result<Vec<Vec<String>>, ErroDeDag> {
        let niveis = self
            .grafo
            .topological_levels()
            .map_err(|e| self.invalido(e))?;
        Ok(niveis
            .into_iter()
            .map(|nivel| nivel.into_iter().map(|no| no.name).collect())
            .collect())
    }

    /// Recupera a tarefa de nome `nome`, ou erro se ela não existe no grafo.
    pub fn tarefa(&self, nome: &str) -> Result<Arc<dyn Tarefa>, ErroDeDag> {
        self.grafo
            .get_payload(nome)
            .map(Arc::clone)
            .map_err(|e| self.invalido(e))
    }

    /// Tarefas das quais `nome` depende diretamente.
    pub fn dependencias(&self, nome: &str) -> Result<Vec<String>, ErroDeDag> {
        let dependencias = self
            .grafo
            .predecessors(nome)
            .map_err(|e| self.invalido(e))?;
        Ok(dependencias.into_iter().map(|no| no.name).collect())
    }

    /// Todas as tarefas que dependem, direta ou indiretamente, de `nome`.
    pub fn descendentes(&self, nome: &str) -> Result<Vec<String>, ErroDeDag> {
        let descendentes = self.grafo.descendants(nome).map_err(|e| self.invalido(e))?;
        Ok(descendentes.into_iter().map(|no| no.name).collect())
    }

    fn invalido(&self, erro: DagronError) -> ErroDeDag {
        ErroDeDag::GrafoInvalido {
            fonte: self.fonte.clone(),
            detalhe: erro.to_string(),
        }
    }
}

#[cfg(test)]
mod testes {
    use async_trait::async_trait;

    use super::*;
    use crate::contexto::ContextoDeExecucao;
    use crate::erros::ErroDeIngestao;
    use crate::tarefa::ResumoDaTarefa;

    struct TarefaVazia(&'static str);

    #[async_trait]
    impl Tarefa for TarefaVazia {
        fn nome(&self) -> &str {
            self.0
        }

        async fn executar(&self, _: &ContextoDeExecucao) -> Result<ResumoDaTarefa, ErroDeIngestao> {
            Ok(ResumoDaTarefa::default())
        }
    }

    fn planejada(nome: &'static str) -> TarefaPlanejada {
        TarefaPlanejada::nova(TarefaVazia(nome))
    }

    #[test]
    fn organiza_as_tarefas_em_niveis_respeitando_dependencias() {
        let dag = DagDeIngestao::construir(
            "teste",
            vec![
                planejada("c").depois_de(["a", "b"]),
                planejada("a"),
                planejada("b"),
            ],
        )
        .unwrap();

        let mut niveis = dag.niveis().unwrap();
        niveis[0].sort();
        assert_eq!(niveis, vec![vec!["a", "b"], vec!["c"]]);
    }

    #[test]
    fn rejeita_ciclo() {
        let resultado = DagDeIngestao::construir(
            "teste",
            vec![
                planejada("a").depois_de(["b"]),
                planejada("b").depois_de(["a"]),
            ],
        );
        assert!(matches!(resultado, Err(ErroDeDag::GrafoInvalido { .. })));
    }

    #[test]
    fn rejeita_dependencia_inexistente() {
        let resultado =
            DagDeIngestao::construir("teste", vec![planejada("a").depois_de(["fantasma"])]);
        assert!(matches!(resultado, Err(ErroDeDag::GrafoInvalido { .. })));
    }

    #[test]
    fn rejeita_nome_duplicado() {
        let resultado = DagDeIngestao::construir("teste", vec![planejada("a"), planejada("a")]);
        assert!(matches!(resultado, Err(ErroDeDag::GrafoInvalido { .. })));
    }

    #[test]
    fn rejeita_fonte_sem_tarefas() {
        let resultado = DagDeIngestao::construir("teste", Vec::new());
        assert!(matches!(resultado, Err(ErroDeDag::SemTarefas { .. })));
    }
}
