//! Contrato Silver para indicadores municipais de drenagem pluvial do SINISA (ex-SNIS).

pub const REGRA: &str = "snis-pluviais-indicadores-v1";
pub const SQL: &str = include_str!("../../sql/silver/snis_pluviais.sql");
