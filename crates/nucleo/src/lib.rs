//! Núcleo do Tô De Olho Aí: abstrações de domínio e execução de DAGs.
//!
//! Este crate não conhece nenhuma API pública nem tecnologia de infraestrutura.
//! As demais camadas dependem dele, nunca o contrário (ver `docs/adr/0002`).

#![warn(missing_docs)]

pub mod contexto;
pub mod dag;
pub mod decisao;
pub mod erros;
pub mod espaco;
pub mod estado_raw;
pub mod executor;
pub mod fonte;
pub mod historico;
pub mod orfas;
pub mod portas;
pub mod raw;
pub mod selecao;
pub mod tarefa;

pub use contexto::{ContextoDeExecucao, ProgressoDaTarefa};
pub use dag::DagDeIngestao;
pub use decisao::{DecisaoDeIngestao, EstadoNaRaw, MotivoDeIngestao, decidir};
pub use erros::{ErroDeDag, ErroDeIngestao};
pub use espaco::{AvaliacaoDeEspaco, EspacoNecessario, estimar, formatar_bytes};
pub use estado_raw::{estado_na_raw, manifesto_vigente, publicar_ponteiro};
pub use executor::{
    EstadoDaTarefa, ExecutorDeDag, ObservadorDeExecucao, RelatorioDaExecucao, ResultadoDaTarefa,
};
pub use fonte::{CredencialHttp, FonteDeDados, LimiteHttp, PerfilHttp};
pub use historico::{
    Camada, ConsultaDeExecucoes, EstadoRegistrado, GravadorDeExecucoes, RegistradorDeExecucao,
    RegistroDeExecucao, RegistroDeFonte, RegistroDeTarefa, SituacaoDaExecucao,
};
pub use orfas::{limpar_orfas_do_recurso, verificar_manifesto_ou_limpar};
pub use portas::{
    ArmazenamentoRaw, ClienteHttp, DownloadHttp, FalhaDeRemocao, FluxoDeBytes,
    InformacoesDoArquivo, ObjetoRaw, RespostaCondicional, RespostaHttp,
};
pub use raw::{LocalizacaoRaw, ManifestoRaw, ParteRaw, UltimaVersao};
pub use selecao::{ErroDeSelecao, FiltroDeTarefas, dimensoes};
pub use tarefa::{PrecisaoDaEstimativa, ResumoDaTarefa, Tarefa, TarefaPlanejada};
