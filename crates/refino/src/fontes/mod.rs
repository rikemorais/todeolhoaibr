//! Decodificadores por fonte: o conhecimento de formato do ADR 0014 mora aqui.

pub mod csv;
pub mod zip_csv;

pub use csv::DecodificadorCsv;
pub use zip_csv::DecodificadorZipCsv;
