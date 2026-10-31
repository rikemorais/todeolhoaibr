use serde::Deserialize;

/// Como descobrir, a partir de uma resposta, a URL da página seguinte.
pub trait EstrategiaDePaginacao: Send + Sync {
    /// `Ok(None)` indica que esta foi a última página.
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String>;
}

/// Para APIs que devolvem tudo em uma única resposta.
pub struct SemPaginacao;

impl EstrategiaDePaginacao for SemPaginacao {
    fn proxima_url(&self, _: &[u8]) -> Result<Option<String>, String> {
        Ok(None)
    }
}

/// Para APIs que informam a próxima página em `links`, no item com `rel: "next"`.
pub struct LinksDePaginacao;

#[derive(Deserialize)]
struct EnvelopeComLinks {
    #[serde(default)]
    links: Vec<Link>,
}

#[derive(Deserialize)]
struct Link {
    rel: String,
    href: String,
}

impl EstrategiaDePaginacao for LinksDePaginacao {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        let envelope: EnvelopeComLinks = serde_json::from_slice(corpo)
            .map_err(|erro| format!("resposta não é o JSON esperado: {erro}"))?;

        Ok(envelope
            .links
            .into_iter()
            .find(|link| link.rel == "next")
            .map(|link| link.href))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn encontra_o_link_da_proxima_pagina() {
        let corpo = br#"{"dados":[],"links":[
            {"rel":"self","href":"https://api/x?pagina=1"},
            {"rel":"next","href":"https://api/x?pagina=2"}
        ]}"#;

        assert_eq!(
            LinksDePaginacao.proxima_url(corpo).unwrap().as_deref(),
            Some("https://api/x?pagina=2")
        );
    }

    #[test]
    fn termina_quando_nao_ha_link_seguinte() {
        let corpo = br#"{"dados":[],"links":[{"rel":"self","href":"https://api/x"}]}"#;
        assert_eq!(LinksDePaginacao.proxima_url(corpo).unwrap(), None);
    }

    #[test]
    fn corpo_invalido_e_erro_e_nao_fim_silencioso() {
        assert!(LinksDePaginacao.proxima_url(b"<html>erro</html>").is_err());
    }
}
