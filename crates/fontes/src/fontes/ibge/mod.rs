//! IBGE: malha territorial (localidades) e indicadores macro (IPCA, INPC, PIB, PNAD).

mod indicadores;
pub mod localidades;
pub mod pib_municipios;

pub use localidades::FonteIbge;
pub use pib_municipios::FonteIbgePibMunicipios;
