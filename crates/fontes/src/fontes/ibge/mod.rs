//! IBGE: malha territorial (localidades) e indicadores macro (IPCA, INPC, PIB, PNAD).

pub mod agropecuaria;
pub mod demografia_empresas;
mod indicadores;
pub mod localidades;
pub mod pib_municipios;
pub mod renda_per_capita;

pub use agropecuaria::FonteIbgeAgropecuaria;
pub use demografia_empresas::FonteIbgeDemografiaEmpresas;
pub use localidades::FonteIbge;
pub use pib_municipios::FonteIbgePibMunicipios;
pub use renda_per_capita::FonteIbgeRendaPerCapita;
