use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;
use super::paginacao::PaginacaoDca;

const NOME: &str = "siconfi-dca";
const LIMITE_DE_PAGINAS: usize = 10_000;

/// Declaração de Contas Anuais (DCA/FINBRA) dos entes federativos.
pub struct FonteSiconfiDca;

impl FonteDeDados for FonteSiconfiDca {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Tesouro Nacional / SICONFI - Declaração de Contas Anuais (DCA/FINBRA) por UF e exercício"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas = Vec::new();
        for ano in api::PRIMEIRO_ANO..=api::ultimo_ano() {
            for uf in &api::UFS {
                let recurso = format!("dca/uf={uf}/ano={ano}");
                let url = format!("{}entes", api::BASE);
                let tarefa = TarefaDeIngestaoHttp::paginada(
                    api::FONTE_RAW,
                    &recurso,
                    url,
                    PaginacaoDca::new(uf, ano),
                )
                .com_limite_de_paginas(LIMITE_DE_PAGINAS);
                tarefas.push(
                    TarefaPlanejada::nova(tarefa)
                        .com_etiquetas([("uf", *uf), ("ano", &ano.to_string())]),
                );
            }
        }
        tarefas
    }
}
