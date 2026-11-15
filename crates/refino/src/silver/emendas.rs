//! Contratos SQL da camada Silver para emendas.

/// Consulta de autoria e localidade textual da CGU.
pub const SQL_EMENDAS_CGU: &str = include_str!("../../sql/silver/emendas_cgu.sql");

/// Consulta dos pagamentos municipais publicados pelo Tesouro Nacional.
pub const SQL_EMENDAS_TESOURO: &str = include_str!("../../sql/silver/emendas_tesouro.sql");

/// Consulta os beneficiários de emendas associados a programas no Transferegov.
pub const SQL_EMENDAS_TRANSFEREGOV: &str =
    include_str!("../../sql/silver/emendas_transferegov.sql");

/// Consulta os favorecidos e valores publicados no painel do SIGA Brasil.
pub const SQL_EMENDAS_SIGA_BRASIL: &str = include_str!("../../sql/silver/emendas_siga_brasil.sql");
