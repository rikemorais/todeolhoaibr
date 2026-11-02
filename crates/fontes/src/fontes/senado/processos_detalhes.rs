use std::collections::{HashSet, VecDeque};
use std::sync::Mutex;

use chrono::{Datelike, Utc};
use serde::Deserialize;
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;
use crate::paginacao::EstrategiaDePaginacao;

use super::api;

const NOME: &str = "senado-processos-detalhes";
const FONTE_RAW: &str = "senado";
const BASE: &str = "https://legis.senado.leg.br/dadosabertos/processo";
const PRIMEIRO_ANO: u16 = 1826;
/// Uma listagem anual mais até 9.999 detalhes por execução.
const LIMITE_DE_PAGINAS: usize = 10_000;

fn ultimo_ano() -> u16 {
    Utc::now().year() as u16
}

#[derive(Deserialize)]
struct ProcessoNaListagem {
    id: u64,
}

#[derive(Default)]
struct EstadoDaPaginacao {
    listagem_processada: bool,
    ids_pendentes: VecDeque<u64>,
}

/// Após guardar a listagem anual, visita em sequência o detalhe de cada ID.
/// A tarefa HTTP persiste a resposta de cada URL como parte bruta independente.
#[derive(Default)]
struct DetalhesDosProcessos {
    estado: Mutex<EstadoDaPaginacao>,
}

impl EstrategiaDePaginacao for DetalhesDosProcessos {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        let mut estado = self
            .estado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if !estado.listagem_processada {
            let processos: Vec<ProcessoNaListagem> = serde_json::from_slice(corpo)
                .map_err(|erro| format!("listagem de processos do Senado inválida: {erro}"))?;
            let mut ids_vistos = HashSet::new();
            estado.ids_pendentes.extend(
                processos
                    .into_iter()
                    .map(|processo| processo.id)
                    .filter(|id| ids_vistos.insert(*id)),
            );
            estado.listagem_processada = true;
        }

        Ok(estado
            .ids_pendentes
            .pop_front()
            .map(|id| format!("{BASE}/{id}")))
    }
}

/// Histórico detalhado dos processos legislativos do Senado, incluindo situação,
/// informes e movimentações publicados pela API. As respostas ficam opacas na Raw.
pub struct FonteSenadoProcessosDetalhes;

impl FonteDeDados for FonteSenadoProcessosDetalhes {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Senado Federal - detalhe e histórico de processos legislativos, uma consulta por processo"
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
                let recurso = format!("processos-detalhes/ano={ano}");
                let url = format!("{BASE}?ano={ano}");
                let tarefa = TarefaDeIngestaoHttp::paginada(
                    FONTE_RAW,
                    &recurso,
                    url,
                    DetalhesDosProcessos::default(),
                )
                .com_limite_de_paginas(LIMITE_DE_PAGINAS);
                TarefaPlanejada::nova(tarefa).com_etiquetas([("ano", ano.to_string().as_str())])
            })
            .collect()
    }
}
