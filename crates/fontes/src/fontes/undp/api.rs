use std::time::Duration;

use todeolho_nucleo::{LimiteHttp, PerfilHttp};

pub(super) const fn perfil_http() -> PerfilHttp {
    PerfilHttp {
        credencial: None,
        limite: Some(LimiteHttp {
            host: "hdr.undp.org",
            requisicoes: 2,
            intervalo: Duration::from_secs(1),
        }),
    }
}
