//! Tipos de erro do núcleo, compartilhados pelas tarefas, pelo executor e pelos
//! adaptadores de infraestrutura que implementam as portas.

use thiserror::Error;

/// Falhas ao executar uma tarefa de ingestão.
#[derive(Debug, Error)]
pub enum ErroDeIngestao {
    /// A requisição HTTP falhou (rede, status não transitório ou corpo ilegível).
    #[error("falha na requisição para {url}: {detalhe}")]
    Http {
        /// URL que estava sendo acessada.
        url: String,
        /// Descrição da falha, já sem a URL (que aparece na mensagem).
        detalhe: String,
    },

    /// Falha ao ler ou gravar um objeto na camada Raw.
    #[error("falha no armazenamento ao acessar {chave}: {detalhe}")]
    Armazenamento {
        /// Chave do objeto envolvido.
        chave: String,
        /// Descrição da falha.
        detalhe: String,
    },

    /// A paginação não pôde continuar (loop, limite excedido ou link inválido).
    #[error("falha ao paginar o recurso {recurso}: {detalhe}")]
    Paginacao {
        /// Recurso que estava sendo paginado.
        recurso: String,
        /// Descrição da falha.
        detalhe: String,
    },

    /// Falha ao serializar o manifesto ou o ponteiro de versão de um recurso.
    #[error("falha ao serializar o manifesto do recurso {recurso}: {detalhe}")]
    Manifesto {
        /// Recurso cujo manifesto falhou.
        recurso: String,
        /// Descrição da falha.
        detalhe: String,
    },
}

/// Falhas ao montar ou consultar o grafo de uma fonte.
#[derive(Debug, Error)]
pub enum ErroDeDag {
    /// A fonte não produziu nenhuma tarefa para executar.
    #[error("nenhuma tarefa selecionada na fonte {fonte}")]
    SemTarefas {
        /// Fonte cujo grafo ficou vazio.
        fonte: String,
    },

    /// O grafo tem nome duplicado, dependência inexistente ou ciclo.
    #[error("grafo inválido na fonte {fonte}: {detalhe}")]
    GrafoInvalido {
        /// Fonte cujo grafo é inválido.
        fonte: String,
        /// Descrição do problema no grafo.
        detalhe: String,
    },
}
