//! Núcleo do Tô De Olho Aí: abstrações de domínio e execução de DAGs.
//!
//! Este crate não conhece nenhuma API pública nem tecnologia de infraestrutura.
//! As demais camadas dependem dele, nunca o contrário (ver `docs/adr/0002`).

pub mod contexto;
pub mod erros;
pub mod portas;
pub mod tarefa;

pub use contexto::ContextoDeExecucao;
pub use erros::{ErroDeDag, ErroDeIngestao};
pub use portas::{
    ArmazenamentoRaw, ClienteHttp, DownloadHttp, FluxoDeBytes, InformacoesDoArquivo, ObjetoRaw,
    RespostaHttp,
};
pub use tarefa::{ResumoDaTarefa, Tarefa, TarefaPlanejada};
