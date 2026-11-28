//! Contrato Silver para indicadores municipais de água do SINISA (ex-SNIS).

pub const REGRA: &str = "snis-agua-indicadores-v1";
pub const SQL: &str = include_str!("../../sql/silver/snis_agua.sql");
