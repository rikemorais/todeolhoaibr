//! TSE: votação por seção, prestação de contas de campanha, candidaturas e bens.

pub mod candidaturas;
pub mod prestacao_contas;
pub mod votacoes;

pub use candidaturas::FonteTseCandidaturas;
pub use prestacao_contas::FonteTsePrestacaoContas;
pub use votacoes::FonteTse;
