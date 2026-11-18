use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

struct EstadoDoHost {
    proxima: tokio::time::Instant,
    intervalo_entre_requisicoes: Duration,
}

static LIMITES_POR_HOST: OnceLock<Mutex<HashMap<String, EstadoDoHost>>> = OnceLock::new();

pub(crate) async fn aguardar_limite(host: &str) {
    if let Some(instante) = reservar_limite(host) {
        tokio::time::sleep_until(instante).await;
    }
}

pub(crate) fn registrar_limite(host: &str, intervalo: Duration) {
    let mut limites = LIMITES_POR_HOST
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let agora = tokio::time::Instant::now();
    let estado = limites.entry(host.to_owned()).or_insert(EstadoDoHost {
        proxima: agora,
        intervalo_entre_requisicoes: intervalo,
    });
    estado.intervalo_entre_requisicoes = estado.intervalo_entre_requisicoes.max(intervalo);
}

pub(crate) fn reservar_limite(host: &str) -> Option<tokio::time::Instant> {
    let mut limites = LIMITES_POR_HOST
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let estado = limites.get_mut(host)?;
    let instante = estado.proxima.max(tokio::time::Instant::now());
    estado.proxima = instante + estado.intervalo_entre_requisicoes;
    Some(instante)
}
