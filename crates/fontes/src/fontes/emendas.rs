//! Emendas da CGU, repasses do Tesouro e beneficiários do Transferegov na Raw.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use chrono::Datelike;
use serde::Deserialize;
use serde_json::Value;
use todeolho_nucleo::{
    CredencialHttp, FonteDeDados, LimiteHttp, PerfilHttp, RedirecionamentoHttp, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;
use crate::ingestao_http::TarefaDeIngestaoHttp;
use crate::paginacao::EstrategiaDePaginacao;

const NOME: &str = "emendas";
const FONTE_RAW: &str = "emendas";
const HOST_CGU: &str = "api.portaldatransparencia.gov.br";
const BASE_CGU: &str = "https://api.portaldatransparencia.gov.br/api-de-dados/emendas";
const BASE_TRANSFEREGOV: &str =
    "https://api-publica.transferegov.gestao.gov.br/parcerias/beneficiario_emenda_parlamentar";
const URL_TESOURO: &str = "https://www.tesourotransparente.gov.br/ckan/dataset/83e419da-1552-46bf-bfc3-05160b2c46c9/resource/66d69917-a5d8-4500-b4b2-ef1f5d062430/download/emendas-parlamentares.csv";
const PRIMEIRO_ANO: i32 = 2015;
const PRIMEIRO_ANO_TRANSFEREGOV: i32 = 2025;
const LIMITE_PAGINAS: usize = 10_000;
const TAMANHO_TIPICO: u64 = 64 * 1024;
const TAMANHO_PAGINA_TRANSFEREGOV: usize = 200;

fn ano_corrente() -> i32 {
    chrono::Utc::now().year()
}

fn url_cgu(ano: i32, pagina: usize) -> String {
    format!("{BASE_CGU}?ano={ano}&pagina={pagina}")
}

fn url_transferegov(ano: i32) -> String {
    format!(
        "{BASE_TRANSFEREGOV}?aa_emenda={ano}&pagina=1&tamanho_da_pagina={TAMANHO_PAGINA_TRANSFEREGOV}"
    )
}

fn tarefas() -> Vec<TarefaPlanejada> {
    let repasses = TarefaPlanejada::nova(TarefaDeArquivo::nova(
        "tesouro-repasses".to_owned(),
        FONTE_RAW,
        "repasses/tesouro".to_owned(),
        URL_TESOURO.to_owned(),
    ))
    .com_etiquetas([("origem", "tesouro")]);

    std::iter::once(repasses)
        .chain((PRIMEIRO_ANO_TRANSFEREGOV..=ano_corrente()).map(|ano| {
            let url = url_transferegov(ano);
            TarefaPlanejada::nova(TarefaDeIngestaoHttp::paginada(
                FONTE_RAW,
                &format!("transferegov/beneficiario-emenda/ano={ano}"),
                url.clone(),
                PaginaBeneficiarioTransferegov::nova(url),
            ))
            .com_etiquetas([
                ("ano", ano.to_string().as_str()),
                ("origem", "transferegov"),
            ])
        }))
        .chain((PRIMEIRO_ANO..=ano_corrente()).map(|ano| {
            let url = url_cgu(ano, 1);
            TarefaPlanejada::nova(
                TarefaDeIngestaoHttp::paginada(
                    FONTE_RAW,
                    &format!("cgu/ano={ano}"),
                    url.clone(),
                    PaginaEmendas::nova(url),
                )
                .com_tamanho_tipico(TAMANHO_TIPICO)
                .com_limite_de_paginas(LIMITE_PAGINAS),
            )
            .com_etiquetas([("ano", ano.to_string().as_str()), ("origem", "cgu")])
        }))
        .collect()
}

/// Paginação da API da CGU, que retorna um array e encerra com array vazio.
struct PaginaEmendas {
    url_base: String,
    proxima: AtomicUsize,
}

impl PaginaEmendas {
    fn nova(url_base: String) -> Self {
        Self {
            url_base,
            proxima: AtomicUsize::new(2),
        }
    }
}

impl EstrategiaDePaginacao for PaginaEmendas {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        let itens: Vec<Value> = serde_json::from_slice(corpo)
            .map_err(|erro| format!("resposta de emendas da CGU não é um array JSON: {erro}"))?;
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

#[derive(Deserialize)]
struct RespostaPaginadaTransferegov {
    total_pages: usize,
}

/// Paginação do Transferegov: o envelope informa o total de páginas.
struct PaginaBeneficiarioTransferegov {
    url_base: String,
    proxima: AtomicUsize,
}

impl PaginaBeneficiarioTransferegov {
    fn nova(url_base: String) -> Self {
        Self {
            url_base,
            proxima: AtomicUsize::new(2),
        }
    }
}

impl EstrategiaDePaginacao for PaginaBeneficiarioTransferegov {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        let resposta: RespostaPaginadaTransferegov = serde_json::from_slice(corpo)
            .map_err(|erro| format!("resposta do Transferegov não é paginada em JSON: {erro}"))?;
        let pagina = self.proxima.fetch_add(1, Ordering::Relaxed);
        if pagina > resposta.total_pages {
            return Ok(None);
        }
        Ok(Some(
            self.url_base
                .replace("pagina=1", &format!("pagina={pagina}")),
        ))
    }
}

/// Emendas parlamentares e pagamentos municipais disponíveis na Raw.
///
/// A API da CGU requer uma chave pessoal gratuita. Tesouro e Transferegov são
/// públicos; este último é particionado por exercício e por página.
pub struct FonteEmendas;

impl FonteDeDados for FonteEmendas {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Emendas - CGU, repasses municipais do Tesouro e beneficiários por emenda no Transferegov"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: Some(CredencialHttp {
                host: HOST_CGU,
                variavel: "PORTAL_TRANSPARENCIA_TOKEN",
                cabecalho: "chave-api-dados",
                prefixo: "",
            }),
            limite: Some(LimiteHttp {
                host: HOST_CGU,
                requisicoes: 90,
                intervalo: Duration::from_secs(60),
            }),
            agente: None,
        }
    }

    fn perfil_http_das_tarefas(&self, tarefas: &[TarefaPlanejada]) -> PerfilHttp {
        if tarefas
            .iter()
            .any(|tarefa| tarefa.etiquetas.get("origem").map(String::as_str) == Some("cgu"))
        {
            self.perfil_http()
        } else {
            PerfilHttp {
                redirecionamento: RedirecionamentoHttp::Bloquear,
                ..PerfilHttp::default()
            }
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        tarefas()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use todeolho_nucleo::{FiltroDeTarefas, FonteDeDados};

    #[test]
    fn cgu_tem_uma_tarefa_por_ano_desde_2015_com_filtro() {
        let tarefas = FonteEmendas.criar_tarefas();
        let filtradas = FiltroDeTarefas::a_partir_de(&["ano=2020".to_owned()])
            .unwrap()
            .aplicar(tarefas)
            .unwrap();

        assert_eq!(filtradas.len(), 1);
        assert_eq!(filtradas[0].tarefa.nome(), "cgu/ano=2020");
        assert_eq!(filtradas[0].etiquetas["origem"], "cgu");
    }

    #[test]
    fn inclui_o_csv_integral_do_tesouro_sem_fragmentar_a_raw() {
        let tarefas = FonteEmendas.criar_tarefas();
        let tesouro = tarefas
            .iter()
            .find(|tarefa| tarefa.etiquetas.get("origem").map(String::as_str) == Some("tesouro"))
            .unwrap();

        assert_eq!(tesouro.tarefa.nome(), "tesouro-repasses");
        assert_eq!(tesouro.etiquetas.len(), 1);
    }

    #[test]
    fn cgu_usa_filtro_de_ano_paginado_e_mantem_url_deterministica() {
        assert_eq!(
            url_cgu(2025, 1),
            "https://api.portaldatransparencia.gov.br/api-de-dados/emendas?ano=2025&pagina=1"
        );
        let paginacao = PaginaEmendas::nova(url_cgu(2025, 1));

        assert_eq!(
            paginacao.proxima_url(br#"[{"codigoEmenda":"1"}]"#).unwrap(),
            Some(url_cgu(2025, 2))
        );
        assert_eq!(paginacao.proxima_url(b"[]").unwrap(), None);
        assert!(paginacao.proxima_url(b"html").is_err());
    }

    #[test]
    fn fonte_exige_token_cgu_e_e_sob_demanda() {
        let fonte = FonteEmendas;
        assert!(fonte.sob_demanda());
        assert_eq!(fonte.nome(), NOME);
        let perfil = fonte.perfil_http();
        let credencial = perfil.credencial.unwrap();
        assert_eq!(credencial.variavel, "PORTAL_TRANSPARENCIA_TOKEN");
        assert_eq!(credencial.cabecalho, "chave-api-dados");
        assert_eq!(perfil.limite.unwrap().requisicoes, 90);
    }

    #[test]
    fn tarefas_somente_tesouro_nao_exigem_token_cgu() {
        let fonte = FonteEmendas;
        let tarefas = FiltroDeTarefas::a_partir_de(&["origem=tesouro".to_owned()])
            .unwrap()
            .aplicar(fonte.criar_tarefas())
            .unwrap();

        assert!(fonte.perfil_http_das_tarefas(&tarefas).credencial.is_none());
    }

    #[test]
    fn tarefas_cgu_mantem_requisito_de_token() {
        let fonte = FonteEmendas;
        let tarefas = FiltroDeTarefas::a_partir_de(&["ano=2020".to_owned()])
            .unwrap()
            .aplicar(fonte.criar_tarefas())
            .unwrap();

        let credencial = fonte
            .perfil_http_das_tarefas(&tarefas)
            .credencial
            .expect("tarefas CGU precisam do token");
        assert_eq!(credencial.variavel, "PORTAL_TRANSPARENCIA_TOKEN");
    }
}
