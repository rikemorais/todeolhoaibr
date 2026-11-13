use std::time::Duration;

use todeolho_nucleo::{LimiteHttp, PerfilHttp};

/// Ritmo local para a API do SGS do Banco Central. A API não documenta uma quota
/// fixa, mas estrangula consultas em rajada; um limite modesto evita o 429 ao
/// buscar várias séries de uma vez.
pub(super) const fn perfil_http() -> PerfilHttp {
    PerfilHttp {
        credencial: None,
        limite: Some(LimiteHttp {
            host: "api.bcb.gov.br",
            requisicoes: 2,
            intervalo: Duration::from_secs(1),
        }),
        agente: None,
    }
}
