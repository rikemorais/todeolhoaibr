//! Decodificadores por fonte: o conhecimento de formato do ADR 0014 mora aqui.

pub mod camara_despesas;
pub mod csv;

pub use camara_despesas::DecodificadorCamaraDespesas;
pub use csv::DecodificadorCsv;
