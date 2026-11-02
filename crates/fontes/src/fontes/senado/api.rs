use std::time::Duration;

use todeolho_nucleo::{LimiteHttp, PerfilHttp};

/// Limite público documentado pelos Dados Abertos do Senado, compartilhado por
/// todas as fontes que consultam `legis.senado.leg.br`.
pub(super) const fn perfil_http() -> PerfilHttp {
    PerfilHttp {
        credencial: None,
        limite: Some(LimiteHttp {
            host: "legis.senado.leg.br",
            requisicoes: 10,
            intervalo: Duration::from_secs(1),
        }),
    }
}
