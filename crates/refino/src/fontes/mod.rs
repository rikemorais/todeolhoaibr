//! Decodificadores por fonte: o conhecimento de formato do ADR 0014 mora aqui.

pub mod csv;
pub mod ibge_agregados;
pub mod json;
pub mod zip_csv;

pub use csv::DecodificadorCsv;
pub use ibge_agregados::DecodificadorIbgeAgregados;
pub use json::DecodificadorJson;
pub use zip_csv::DecodificadorZipCsv;
