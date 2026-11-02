use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

use chrono::{Datelike, Utc};
use serde::Deserialize;
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;
use crate::paginacao::EstrategiaDePaginacao;

use super::api;

const NOME: &str = "camara-tramitacoes";
const FONTE_RAW: &str = "camara";
const BASE: &str = "https://dadosabertos.camara.leg.br/api/v2";
const PRIMEIRO_ANO: u16 = 2001;
/// Uma listagem anual e até 99.999 páginas de tramitação.
const LIMITE_DE_PAGINAS: usize = 100_000;

fn ultimo_ano() -> u16 {
    Utc::now().year() as u16
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct RespostaPaginada<T> {
    #[serde(default)]
    dados: Vec<T>,
    #[serde(default)]
    links: Vec<Link>,
}

#[derive(Deserialize)]
struct Proposicao {
    id: u64,
}

#[derive(Deserialize)]
struct Link {
    rel: String,
    href: String,
}

impl<T> RespostaPaginada<T> {
    fn proxima_pagina(&self) -> Option<String> {
        self.links
            .iter()
            .find(|link| link.rel == "next")
            .map(|link| link.href.clone())
    }
}

#[derive(Default)]
struct EstadoDaPaginacao {
    esperando_listagem: bool,
    proxima_listagem: Option<String>,
    ids_pendentes: VecDeque<u64>,
    ids_vistos: HashSet<u64>,
}

/// Percorre páginas de proposições e suas páginas de tramitação sem carregar os
/// corpos anteriores; cada resposta é persistida pela tarefa HTTP antes do avanço.
#[derive(Default)]
struct TramitacoesDasProposicoes {
    estado: Mutex<EstadoDaPaginacao>,
}

impl TramitacoesDasProposicoes {
    fn retirar_proposicao(estado: &mut EstadoDaPaginacao) -> Option<String> {
        estado
            .ids_pendentes
            .pop_front()
            .map(|id| format!("{BASE}/proposicoes/{id}/tramitacoes"))
    }
}

impl EstrategiaDePaginacao for TramitacoesDasProposicoes {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        let mut estado = self
            .estado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if estado.esperando_listagem {
            let resposta: RespostaPaginada<Proposicao> = serde_json::from_slice(corpo)
                .map_err(|erro| format!("listagem de proposições da Câmara inválida: {erro}"))?;
            estado.proxima_listagem = resposta.proxima_pagina();
            for proposicao in resposta.dados {
                if estado.ids_vistos.insert(proposicao.id) {
                    estado.ids_pendentes.push_back(proposicao.id);
                }
            }
            estado.esperando_listagem = false;

            if let Some(url) = Self::retirar_proposicao(&mut estado) {
                return Ok(Some(url));
            }
            if let Some(url) = estado.proxima_listagem.take() {
                estado.esperando_listagem = true;
                return Ok(Some(url));
            }
            return Ok(None);
        }

        let resposta: RespostaPaginada<serde_json::Value> = serde_json::from_slice(corpo)
            .map_err(|erro| format!("histórico de tramitação da Câmara inválido: {erro}"))?;
        if let Some(url) = resposta.proxima_pagina() {
            return Ok(Some(url));
        }

        if let Some(url) = Self::retirar_proposicao(&mut estado) {
            return Ok(Some(url));
        }

        if let Some(url) = estado.proxima_listagem.take() {
            estado.esperando_listagem = true;
            return Ok(Some(url));
        }

        Ok(None)
    }
}

/// Histórico oficial das tramitações das proposições publicadas pela Câmara.
pub struct FonteCamaraTramitacoes;

impl FonteDeDados for FonteCamaraTramitacoes {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Câmara dos Deputados - histórico de tramitações das proposições por ano"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        (PRIMEIRO_ANO..=ultimo_ano())
            .map(|ano| {
                let recurso = format!("tramitacoes/ano={ano}");
                let url = format!("{BASE}/proposicoes?ano={ano}&itens=100");
                let tarefa = TarefaDeIngestaoHttp::paginada(
                    FONTE_RAW,
                    &recurso,
                    url,
                    TramitacoesDasProposicoes {
                        estado: Mutex::new(EstadoDaPaginacao {
                            esperando_listagem: true,
                            ..EstadoDaPaginacao::default()
                        }),
                    },
                )
                .com_limite_de_paginas(LIMITE_DE_PAGINAS);
                TarefaPlanejada::nova(tarefa).com_etiquetas([("ano", ano.to_string().as_str())])
            })
            .collect()
    }
}
