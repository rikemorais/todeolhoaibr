use thiserror::Error;

/// Falhas ao executar uma tarefa de ingestão.
#[derive(Debug, Error)]
pub enum ErroDeIngestao {
    #[error("falha na requisição para {url}: {detalhe}")]
    Http { url: String, detalhe: String },

    #[error("falha no armazenamento ao acessar {chave}: {detalhe}")]
    Armazenamento { chave: String, detalhe: String },

    #[error("falha ao paginar o recurso {recurso}: {detalhe}")]
    Paginacao { recurso: String, detalhe: String },

    #[error("falha ao serializar o manifesto do recurso {recurso}: {detalhe}")]
    Manifesto { recurso: String, detalhe: String },
}

/// Falhas ao montar ou consultar o grafo de uma fonte.
#[derive(Debug, Error)]
pub enum ErroDeDag {
    #[error("nenhuma tarefa selecionada na fonte {fonte}")]
    SemTarefas { fonte: String },

    #[error("grafo inválido na fonte {fonte}: {detalhe}")]
    GrafoInvalido { fonte: String, detalhe: String },
}
