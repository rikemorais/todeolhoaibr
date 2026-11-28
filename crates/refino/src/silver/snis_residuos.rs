//! Contrato Silver para indicadores municipais de resíduos sólidos do SINISA (ex-SNIS).

pub const REGRA: &str = "snis-residuos-indicadores-v1";
pub const SQL: &str = include_str!("../../sql/silver/snis_residuos.sql");
