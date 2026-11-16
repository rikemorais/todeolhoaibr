use std::time::Duration;

use todeolho_nucleo::{LimiteHttp, PerfilHttp};

/// Ritmo local para a API OData do Ipeadata. Um limite modesto evita sufocar o host
/// ao buscar várias séries de uma vez.
pub(super) const fn perfil_http() -> PerfilHttp {
    PerfilHttp {
        redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
        credencial: None,
        limite: Some(LimiteHttp {
            host: "www.ipeadata.gov.br",
            requisicoes: 2,
            intervalo: Duration::from_secs(1),
        }),
        agente: None,
        aceitar_certificado_invalido: false,
    }
}
