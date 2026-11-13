use std::time::Duration;

use todeolho_nucleo::{LimiteHttp, PerfilHttp};

/// Ritmo local compartilhado para o host da API da Câmara. A API publica limite
/// de 100 itens por página, mas não documenta uma quota fixa por segundo.
pub(super) const fn perfil_http() -> PerfilHttp {
    PerfilHttp {
        credencial: None,
        limite: Some(LimiteHttp {
            host: "dadosabertos.camara.leg.br",
            requisicoes: 5,
            intervalo: Duration::from_secs(1),
        }),
        agente: None,
    }
}
