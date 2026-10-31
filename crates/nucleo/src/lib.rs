//! Núcleo do Tô De Olho Aí: abstrações de domínio e execução de DAGs.
//!
//! Este crate não conhece nenhuma API pública nem tecnologia de infraestrutura.
//! As demais camadas dependem dele, nunca o contrário (ver `docs/adr/0002`).

pub mod contexto;
pub mod dag;
pub mod erros;
pub mod executor;
pub mod fonte;
pub mod historico;
pub mod portas;
pub mod raw;
pub mod selecao;
pub mod tarefa;

pub use contexto::ContextoDeExecucao;
pub use dag::DagDeIngestao;
pub use erros::{ErroDeDag, ErroDeIngestao};
pub use executor::{
    EstadoDaTarefa, ExecutorDeDag, ObservadorDeExecucao, RelatorioDaExecucao, ResultadoDaTarefa,
};
pub use fonte::FonteDeDados;
pub use historico::{
    ConsultaDeExecucoes, EstadoRegistrado, GravadorDeExecucoes, RegistradorDeExecucao,
    RegistroDeExecucao, RegistroDeFonte, RegistroDeTarefa, SituacaoDaExecucao,
};
pub use portas::{
    ArmazenamentoRaw, ClienteHttp, DownloadHttp, FluxoDeBytes, InformacoesDoArquivo, ObjetoRaw,
    RespostaHttp,
};
pub use raw::{LocalizacaoRaw, ManifestoRaw, ParteRaw};
pub use selecao::{ErroDeSelecao, FiltroDeTarefas, dimensoes};
pub use tarefa::{ResumoDaTarefa, Tarefa, TarefaPlanejada};
