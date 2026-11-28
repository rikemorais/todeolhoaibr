//! Contrato Silver para indicadores municipais de esgoto do SINISA (ex-SNIS).

pub const REGRA: &str = "snis-esgoto-indicadores-v1";
pub const SQL: &str = include_str!("../../sql/silver/snis_esgoto.sql");
