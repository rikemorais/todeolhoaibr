use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;
use super::paginacao::PaginacaoRreo;

const NOME: &str = "siconfi-rreo";
const LIMITE_DE_PAGINAS: usize = 10_000;

/// Relatório Resumido de Execução Orçamentária (RREO) dos entes federativos.
pub struct FonteSiconfiRreo;

impl FonteDeDados for FonteSiconfiRreo {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Tesouro Nacional / SICONFI - Relatório Resumido de Execução Orçamentária (RREO) por UF, exercício e bimestre"
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
                for bimestre in 1..=6u8 {
                    let recurso = format!("rreo/uf={uf}/ano={ano}/bimestre={bimestre}");
                    let url = format!("{}entes", api::BASE);
                    let tarefa = TarefaDeIngestaoHttp::paginada(
                        api::FONTE_RAW,
                        &recurso,
                        url,
                        PaginacaoRreo::new(uf, ano, bimestre),
                    )
                    .com_limite_de_paginas(LIMITE_DE_PAGINAS);
                    tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                        ("uf", *uf),
                        ("ano", &ano.to_string()),
                        ("bimestre", &bimestre.to_string()),
                    ]));
                }
            }
        }
        tarefas
    }
}
