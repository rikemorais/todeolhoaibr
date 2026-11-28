//! Contrato Silver para informações trimestrais da CVM (ITR).

pub const REGRA: &str = "cvm-itr-demonstracao-v1";
pub const SQL: &str = include_str!("../../sql/silver/cvm_itr.sql");
