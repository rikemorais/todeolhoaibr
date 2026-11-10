//! Totais agregados das folhas municipais enviados ao SIM do TCE-CE.
//!
//! A fonte ingere somente `/folhas_pagamentos`, cujo contrato contém totais por
//! folha/unidade e não os campos nominais de `/agentes_publicos_folha`.

use std::time::Duration;

use serde::Deserialize;
use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;
use crate::paginacao::EstrategiaDePaginacao;

const NOME: &str = "remuneracao-municipal-tce-ce";
const BASE: &str = "https://api-dados-abertos.tce.ce.gov.br/sim/folhas_pagamentos";
const COMPETENCIA: &str = "2025-12";
const REFERENCIA: u32 = 202512;
const EXERCICIO: u32 = 202500;
const TAMANHO_PAGINA: usize = 1000;
const PRIMEIRO_CODIGO_SIM: u16 = 2;
const ULTIMO_CODIGO_SIM: u16 = 185;

#[derive(Deserialize)]
struct EnvelopeFolhas {
    #[serde(default)]
    links: Vec<Link>,
}

#[derive(Deserialize)]
struct Link {
    rel: String,
    href: String,
}

/// Paginação por `links.next`, resolvendo a URL relativa documentada pela API.
struct PaginacaoFolhas;

impl EstrategiaDePaginacao for PaginacaoFolhas {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        let envelope: EnvelopeFolhas = serde_json::from_slice(corpo)
            .map_err(|erro| format!("resposta de folhas inválida: {erro}"))?;
        envelope
            .links
            .into_iter()
            .find(|link| link.rel == "next")
            .map(|link| normalizar_link(&link.href))
            .transpose()
    }
}

fn url_inicial(codigo_municipio: u16) -> String {
    format!(
        "{BASE}?codigo_municipio={codigo_municipio:03}&exercicio_orcamento={EXERCICIO}&data_referencia_doc={REFERENCIA}&$format=json&$count={TAMANHO_PAGINA}&$start_index=0"
    )
}

fn normalizar_link(href: &str) -> Result<String, String> {
    if href.starts_with('?') {
        return Ok(format!("{BASE}{href}"));
    }
    if href.starts_with(&format!("{BASE}?")) {
        return Ok(href.to_owned());
    }
    Err(format!(
        "link de paginação fora do endpoint esperado: {href}"
    ))
}

/// Ingestão dos totais agregados do TCE-CE para os 184 municípios com código
/// IBGE, na competência 2025-12. Não consulta o endpoint nominal de agentes.
pub struct FonteRemuneracaoMunicipalTceCe;

impl FonteDeDados for FonteRemuneracaoMunicipalTceCe {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "TCE-CE - totais agregados de folhas municipais (competência 2025-12)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "api-dados-abertos.tce.ce.gov.br",
                requisicoes: 5,
                intervalo: Duration::from_secs(1),
            }),
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        (PRIMEIRO_CODIGO_SIM..=ULTIMO_CODIGO_SIM)
            .map(|codigo| {
                let municipio = format!("{codigo:03}");
                let recurso =
                    format!("folhas-pagamentos/municipio={municipio}/competencia={COMPETENCIA}");
                let tarefa = TarefaDeIngestaoHttp::paginada(
                    NOME,
                    &recurso,
                    url_inicial(codigo),
                    PaginacaoFolhas,
                )
                .com_tamanho_tipico(250_000)
                .com_limite_de_paginas(20_000);

                TarefaPlanejada::nova(tarefa).com_etiquetas([
                    ("municipio_sim", municipio.as_str()),
                    ("competencia", COMPETENCIA),
                ])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn cria_tarefa_particionada_para_cada_municipio_validado() {
        let tarefas = FonteRemuneracaoMunicipalTceCe.criar_tarefas();
        assert_eq!(tarefas.len(), 184);

        let codigos: BTreeSet<_> = tarefas
            .iter()
            .map(|tarefa| tarefa.etiquetas["municipio_sim"].as_str())
            .collect();
        assert_eq!(codigos.len(), 184);
        assert!(codigos.contains("057"));
        assert!(codigos.contains("185"));
        assert!(
            tarefas
                .iter()
                .all(|tarefa| tarefa.etiquetas["competencia"] == COMPETENCIA)
        );
    }

    #[test]
    fn url_consulta_a_folha_agregada_do_municipio_e_competencia() {
        assert_eq!(
            url_inicial(57),
            "https://api-dados-abertos.tce.ce.gov.br/sim/folhas_pagamentos?codigo_municipio=057&exercicio_orcamento=202500&data_referencia_doc=202512&$format=json&$count=1000&$start_index=0"
        );
    }

    #[test]
    fn segue_link_relativo_e_rejeita_host_inesperado() {
        let estrategia = PaginacaoFolhas;
        assert_eq!(
            estrategia
                .proxima_url(br#"{"elements":[],"links":[{"rel":"next","href":"?$start_index=1000&codigo_municipio=057"}]}"#)
                .unwrap()
                .as_deref(),
            Some("https://api-dados-abertos.tce.ce.gov.br/sim/folhas_pagamentos?$start_index=1000&codigo_municipio=057")
        );
        assert!(normalizar_link("https://example.com/steal").is_err());
        assert_eq!(
            estrategia
                .proxima_url(br#"{"elements":[],"links":[]}"#)
                .unwrap(),
            None
        );
    }

    #[test]
    fn fonte_e_sob_demanda_para_nao_entrar_na_ingestao_completa() {
        assert!(FonteRemuneracaoMunicipalTceCe.sob_demanda());
        assert_eq!(
            FonteRemuneracaoMunicipalTceCe
                .perfil_http()
                .limite
                .unwrap()
                .requisicoes,
            5
        );
    }
}
