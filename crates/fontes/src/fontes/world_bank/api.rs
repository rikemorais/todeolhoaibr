use std::time::Duration;

use todeolho_nucleo::{LimiteHttp, PerfilHttp};

pub(super) const fn perfil_http() -> PerfilHttp {
    PerfilHttp {
        redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
        credencial: None,
        limite: Some(LimiteHttp {
            host: "api.worldbank.org",
            requisicoes: 2,
            intervalo: Duration::from_secs(1),
        }),
        agente: None,
        aceitar_certificado_invalido: false,
    }
}
