//! Informações estáticas do catálogo de fontes, usadas pelo painel para listar
//! as fontes disponíveis sem depender das implementações concretas.

use serde::Serialize;

/// Descrição de uma fonte e suas tarefas, sem nenhuma lógica de execução.
#[derive(Debug, Clone, Serialize)]
pub struct DescricaoDeFonte {
    /// Nome da fonte (ex.: `transparencia-beneficios`).
    pub nome: String,
    /// Texto curto.
    pub descricao: String,
    /// Camadas em que a fonte aparece (ex.: `["raw"]` ou `["raw", "bronze"]`).
    pub camadas: Vec<String>,
    /// Tarefas da camada raw, com dependências (vazio se não tiver raw).
    pub tarefas: Vec<DescricaoDeTarefa>,
    /// Se a fonte é sob demanda (não roda com `--todas`).
    pub sob_demanda: bool,
}

/// Uma tarefa no grafo de uma fonte.
#[derive(Debug, Clone, Serialize)]
pub struct DescricaoDeTarefa {
    /// Nome da tarefa.
    pub nome: String,
    /// Nomes das tarefas predecessoras.
    pub depende_de: Vec<String>,
}

/// Fornece a lista de fontes disponíveis com suas descrições estáticas.
pub trait CatalogoDeFontes: Send + Sync {
    /// Todas as fontes conhecidas.
    fn fontes(&self) -> Vec<DescricaoDeFonte>;
}
