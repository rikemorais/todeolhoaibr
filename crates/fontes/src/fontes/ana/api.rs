use std::time::Duration;

use todeolho_nucleo::{LimiteHttp, PerfilHttp};

pub(super) const fn perfil_http() -> PerfilHttp {
    PerfilHttp {
        credencial: None,
        limite: Some(LimiteHttp {
            host: "telemetriaws1.ana.gov.br",
            requisicoes: 1,
            intervalo: Duration::from_secs(2),
        }),
        agente: None,
    }
}
