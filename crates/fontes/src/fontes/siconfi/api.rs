use std::time::Duration;

use chrono::{Datelike, Utc};
use todeolho_nucleo::{LimiteHttp, PerfilHttp};

pub(super) const BASE: &str = "https://apidatalake.tesouro.gov.br/ords/siconfi/tt/";
pub(super) const FONTE_RAW: &str = "siconfi";
pub(super) const PRIMEIRO_ANO: u16 = 2015;

pub(super) fn ultimo_ano() -> u16 {
    Utc::now().year() as u16
}

pub(super) const UFS: [&str; 27] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "DF", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE",
    "PI", "PR", "RJ", "RN", "RO", "RR", "RS", "SC", "SE", "SP", "TO",
];

pub(super) const fn perfil_http() -> PerfilHttp {
    PerfilHttp {
        redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
        credencial: None,
        limite: Some(LimiteHttp {
            host: "apidatalake.tesouro.gov.br",
            requisicoes: 5,
            intervalo: Duration::from_secs(1),
        }),
        agente: None,
        aceitar_certificado_invalido: false,
    }
}
