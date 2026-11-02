//! Senado Federal: parlamentares, processos, detalhes e votações.

mod api;
pub mod parlamentares;
pub mod processos;
pub mod processos_detalhes;
pub mod votacoes;

pub use parlamentares::FonteSenadoParlamentares;
pub use processos::FonteSenadoProcessos;
pub use processos_detalhes::FonteSenadoProcessosDetalhes;
pub use votacoes::FonteSenadoVotacoes;
