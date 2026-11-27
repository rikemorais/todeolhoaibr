//! Cartão de pagamento do governo federal (CPGF/CPCC/CPDC) na Raw, via API v2
//! do Portal da Transparência.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use chrono::{Datelike, NaiveDate};
use serde_json::Value;
use todeolho_nucleo::{CredencialHttp, FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;
use crate::paginacao::EstrategiaDePaginacao;

const NOME: &str = "transparencia-cpgf";
const FONTE_RAW: &str = "transparencia";
const HOST_API: &str = "api.portaldatransparencia.gov.br";
const BASE_API: &str = "https://api.portaldatransparencia.gov.br/api-de-dados";
const PRIMEIRA_COMPETENCIA: (i32, u32) = (2014, 1);
const TAMANHO_TIPICO: u64 = 64 * 1024;
const LIMITE_PAGINAS: usize = 10_000;

fn ultima_competencia_publicada() -> (i32, u32) {
    let mes = chrono::Utc::now()
        .date_naive()
        .with_day(1)
        .expect("dia válido");
    let mes = mes
        .checked_sub_months(chrono::Months::new(2))
        .unwrap_or(mes);
    (mes.year(), mes.month())
}

fn competencias(inicio: (i32, u32), fim: (i32, u32)) -> Vec<String> {
    let mut atual = NaiveDate::from_ymd_opt(inicio.0, inicio.1, 1).expect("competência válida");
    let limite = NaiveDate::from_ymd_opt(fim.0, fim.1, 1).expect("competência válida");
    let mut resultado = Vec::new();
    while atual <= limite {
        resultado.push(format!("{}-{:02}", atual.year(), atual.month()));
        let Some(proximo) = atual.checked_add_months(chrono::Months::new(1)) else {
            break;
        };
        atual = proximo;
    }
    resultado
}

fn competencia_para_mm_aaaa(competencia: &str) -> String {
    let partes: Vec<&str> = competencia.split('-').collect();
    format!("{}/{}", partes[1], partes[0])
}

fn url_cartoes(competencia: &str, pagina: usize) -> String {
    let mm_aaaa = competencia_para_mm_aaaa(competencia);
    format!("{BASE_API}/cartoes?mesExtratoInicio={mm_aaaa}&mesExtratoFim={mm_aaaa}&pagina={pagina}")
}

fn recurso(competencia: &str) -> String {
    format!("cartoes/competencia={competencia}")
}

struct PaginaCartoes {
    url_base: String,
    proxima: AtomicUsize,
}

impl PaginaCartoes {
    fn nova(url_base: String) -> Self {
        Self {
            url_base,
            proxima: AtomicUsize::new(2),
        }
    }
}

impl EstrategiaDePaginacao for PaginaCartoes {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        if corpo.is_empty() {
            return Ok(None);
        }
        let itens: Vec<Value> = serde_json::from_slice(corpo)
            .map_err(|erro| format!("resposta de cartões do Portal não é um array JSON: {erro}"))?;
        if itens.is_empty() {
            return Ok(None);
        }
        let pagina = self.proxima.fetch_add(1, Ordering::Relaxed);
        Ok(Some(
            self.url_base
                .replace("pagina=1", &format!("pagina={pagina}")),
        ))
    }
}

/// Cartão de pagamento do governo federal (CPGF, CPCC, CPDC) em partições
/// mensais na Raw do Portal da Transparência.
pub struct FonteTransparenciaCpgf;

impl FonteDeDados for FonteTransparenciaCpgf {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Portal da Transparência — cartão de pagamento do governo federal (CPGF/CPCC/CPDC) por competência"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn usar_tls_nativo(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: Some(CredencialHttp {
                host: HOST_API,
                variavel: "PORTAL_TRANSPARENCIA_TOKEN",
                cabecalho: "chave-api-dados",
                prefixo: "",
            }),
            limite: Some(LimiteHttp {
                host: HOST_API,
                requisicoes: 90,
                intervalo: Duration::from_secs(60),
            }),
            agente: None,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        competencias(PRIMEIRA_COMPETENCIA, ultima_competencia_publicada())
            .into_iter()
            .map(|competencia| {
                let url = url_cartoes(&competencia, 1);
                TarefaPlanejada::nova(
                    TarefaDeIngestaoHttp::paginada(
                        FONTE_RAW,
                        &recurso(&competencia),
                        url.clone(),
                        PaginaCartoes::nova(url),
                    )
                    .com_tamanho_tipico(TAMANHO_TIPICO)
                    .com_limite_de_paginas(LIMITE_PAGINAS),
                )
                .com_etiquetas([("competencia", competencia.as_str())])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn converte_competencia_para_formato_do_portal() {
        assert_eq!(competencia_para_mm_aaaa("2024-03"), "03/2024");
        assert_eq!(competencia_para_mm_aaaa("2014-01"), "01/2014");
    }

    #[test]
    fn url_tem_formato_correto_com_mes_e_pagina() {
        assert_eq!(
            url_cartoes("2024-03", 1),
            "https://api.portaldatransparencia.gov.br/api-de-dados/cartoes?mesExtratoInicio=03/2024&mesExtratoFim=03/2024&pagina=1"
        );
        assert_eq!(
            url_cartoes("2014-01", 5),
            "https://api.portaldatransparencia.gov.br/api-de-dados/cartoes?mesExtratoInicio=01/2014&mesExtratoFim=01/2014&pagina=5"
        );
    }

    #[test]
    fn recurso_particiona_por_competencia() {
        assert_eq!(recurso("2024-03"), "cartoes/competencia=2024-03");
        assert_ne!(recurso("2024-03"), recurso("2024-04"));
    }

    #[test]
    fn paginacao_encerra_com_array_vazio_e_avanca_com_dados() {
        let url = url_cartoes("2024-03", 1);
        let pagina = PaginaCartoes::nova(url);

        assert_eq!(pagina.proxima_url(b"").unwrap(), None);
        assert_eq!(pagina.proxima_url(b"[]").unwrap(), None);

        assert_eq!(
            pagina.proxima_url(br#"[{"id":1}]"#).unwrap().as_deref(),
            Some(
                "https://api.portaldatransparencia.gov.br/api-de-dados/cartoes?mesExtratoInicio=03/2024&mesExtratoFim=03/2024&pagina=2"
            )
        );
        assert_eq!(
            pagina.proxima_url(br#"[{"id":2}]"#).unwrap().as_deref(),
            Some(
                "https://api.portaldatransparencia.gov.br/api-de-dados/cartoes?mesExtratoInicio=03/2024&mesExtratoFim=03/2024&pagina=3"
            )
        );
        assert!(pagina.proxima_url(b"<html>").is_err());
    }

    #[test]
    fn fonte_exige_credencial_e_e_sob_demanda() {
        let fonte = FonteTransparenciaCpgf;
        assert!(fonte.sob_demanda());
        assert_eq!(fonte.nome(), NOME);
        let perfil = fonte.perfil_http();
        let credencial = perfil.credencial.unwrap();
        assert_eq!(credencial.variavel, "PORTAL_TRANSPARENCIA_TOKEN");
        assert_eq!(credencial.cabecalho, "chave-api-dados");
        assert_eq!(credencial.host, HOST_API);
        assert_eq!(perfil.limite.unwrap().requisicoes, 90);
    }

    #[test]
    fn tarefas_cobrem_desde_2014_com_etiqueta_de_competencia() {
        let tarefas = FonteTransparenciaCpgf.criar_tarefas();
        assert!(tarefas.len() >= 120);
        assert!(
            tarefas
                .iter()
                .all(|t| t.etiquetas.contains_key("competencia"))
        );
        assert!(
            tarefas
                .iter()
                .any(|t| t.etiquetas["competencia"] == "2014-01")
        );
    }

    #[test]
    fn competencias_gera_sequencia_mensal_correta() {
        let seq = competencias((2024, 1), (2024, 4));
        assert_eq!(seq, ["2024-01", "2024-02", "2024-03", "2024-04"]);
    }
}
