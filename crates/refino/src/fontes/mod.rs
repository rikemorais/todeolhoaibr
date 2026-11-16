//! Decodificadores por fonte: o conhecimento de formato do ADR 0014 mora aqui.

pub mod cnpj;
pub mod conab_safra;
pub mod cotahist;
pub mod csv;
pub mod emendas_siga;
pub mod ibge_agregados;
pub mod incra_assentamentos;
pub mod json;
pub mod mapa_empresas;
pub mod senado_processos_detalhes;
pub mod xml_dataset;
pub mod zip_csv;

pub use cnpj::DecodificadorCnpj;
pub use conab_safra::DecodificadorConabSafra;
pub use cotahist::DecodificadorCotahist;
pub use csv::DecodificadorCsv;
pub use emendas_siga::DecodificadorEmendasSiga;
pub use ibge_agregados::DecodificadorIbgeAgregados;
pub use incra_assentamentos::DecodificadorAssentamentosIncra;
pub use json::DecodificadorJson;
pub use mapa_empresas::DecodificadorMapaEmpresas;
pub use senado_processos_detalhes::DecodificadorSenadoProcessosDetalhes;
pub use xml_dataset::DecodificadorXmlDataset;
pub use zip_csv::DecodificadorZipCsv;
