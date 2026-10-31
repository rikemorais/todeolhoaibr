use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;

use crate::contexto::ContextoDeExecucao;
use crate::decisao::MotivoDeIngestao;
use crate::erros::ErroDeIngestao;

/// O que uma tarefa concluída produziu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResumoDaTarefa {
    pub partes: usize,
    pub bytes: u64,
    /// Nada foi baixado porque os dados não mudaram na origem desde a última ingestão.
    pub reaproveitada: bool,
    /// Por que foi reingerida, quando a pré-checagem decidiu reingerir. `None`
    /// quando reaproveitada ou quando a tarefa não faz pré-checagem.
    pub motivo: Option<MotivoDeIngestao>,
}

/// Unidade de trabalho de uma DAG (o "produto" do Factory Method).
#[async_trait]
pub trait Tarefa: Send + Sync {
    fn nome(&self) -> &str;

    /// Quanto esta tarefa deve gravar, antes de executá-la. `None` significa desconhecido.
    async fn bytes_a_baixar(
        &self,
        _contexto: &ContextoDeExecucao,
    ) -> Result<Option<u64>, ErroDeIngestao> {
        Ok(None)
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao>;
}

/// Uma tarefa e os nomes das tarefas que precisam terminar antes dela.
pub struct TarefaPlanejada {
    pub tarefa: Arc<dyn Tarefa>,
    pub depende_de: Vec<String>,
    /// Permitem escolher parte da fonte, como `ano=2024`. Tarefas etiquetadas ganham filtro
    /// na CLI e no menu sem nenhum código específico.
    pub etiquetas: BTreeMap<String, String>,
}

impl TarefaPlanejada {
    pub fn nova(tarefa: impl Tarefa + 'static) -> Self {
        Self {
            tarefa: Arc::new(tarefa),
            depende_de: Vec::new(),
            etiquetas: BTreeMap::new(),
        }
    }

    pub fn com_etiquetas<const N: usize>(mut self, etiquetas: [(&str, &str); N]) -> Self {
        self.etiquetas.extend(
            etiquetas
                .into_iter()
                .map(|(nome, valor)| (nome.to_owned(), valor.to_owned())),
        );
        self
    }

    pub fn depois_de<const N: usize>(mut self, tarefas: [&str; N]) -> Self {
        self.depende_de
            .extend(tarefas.into_iter().map(str::to_owned));
        self
    }
}
