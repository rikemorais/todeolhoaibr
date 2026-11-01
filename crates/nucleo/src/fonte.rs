//! O contrato [`FonteDeDados`]: cada API pública ingerida implementa este trait,
//! que decide quais tarefas existem; montar e executar o grafo é igual para todas.

use crate::dag::DagDeIngestao;
use crate::erros::ErroDeDag;
use crate::tarefa::TarefaPlanejada;

/// Uma API pública ingerida pelo projeto (o "criador" do Factory Method).
///
/// Cada fonte concreta só decide *quais* tarefas existem, em [`criar_tarefas`].
/// Validar o grafo e executá-lo é igual para todas, então novas fontes entram sem
/// alterar o executor. Ver `docs/adr/0004`.
///
/// [`criar_tarefas`]: FonteDeDados::criar_tarefas
pub trait FonteDeDados: Send + Sync {
    /// Identificador estável: é usado na CLI e no caminho dos dados na camada Raw.
    fn nome(&self) -> &str;

    /// Texto curto que descreve a fonte, exibido na CLI e no menu de seleção.
    fn descricao(&self) -> &str;

    /// Fontes muito grandes só rodam quando pedidas pelo nome, nunca em `--todas`.
    fn sob_demanda(&self) -> bool {
        false
    }

    /// Factory Method: cria as tarefas concretas desta fonte.
    fn criar_tarefas(&self) -> Vec<TarefaPlanejada>;

    /// Operação comum do criador: monta a DAG validada a partir do Factory Method.
    fn montar_dag(&self) -> Result<DagDeIngestao, ErroDeDag> {
        DagDeIngestao::construir(self.nome(), self.criar_tarefas())
    }
}
