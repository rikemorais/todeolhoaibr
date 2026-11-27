//! Informações estáticas do catálogo de fontes, usadas pelo painel para listar
//! as fontes disponíveis sem depender das implementações concretas.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::erros::ErroDeIngestao;
use crate::historico::Camada;

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

/// Pedido de execução recebido pelo painel.
#[derive(Debug, Clone, Deserialize)]
pub struct PedidoDeExecucao {
    /// Camadas a executar (ex.: `[Raw]`, `[Raw, Bronze]`).
    pub camadas: Vec<Camada>,
}

/// Resposta de um despacho bem-sucedido.
#[derive(Debug, Clone, Serialize)]
pub struct RespostaDeDespacho {
    /// IDs das execuções iniciadas (uma por camada).
    pub ids: Vec<String>,
}

/// Dispara execuções em background a partir do painel.
#[async_trait]
pub trait DispachadorDeExecucoes: Send + Sync {
    /// Inicia a execução da fonte nas camadas pedidas. Retorna imediatamente
    /// com os IDs das execuções iniciadas.
    async fn despachar(
        &self,
        fonte: &str,
        pedido: &PedidoDeExecucao,
    ) -> Result<RespostaDeDespacho, ErroDeIngestao>;
}
