//! Contrato Silver para Declaração de Contas Anuais do SICONFI (DCA).

pub const REGRA: &str = "siconfi-dca-declaracao-v1";
pub const SQL: &str = include_str!("../../sql/silver/siconfi_dca.sql");
