use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;
use super::paginacao::PaginacaoRgf;

const NOME: &str = "siconfi-rgf";
const LIMITE_DE_PAGINAS: usize = 10_000;

/// Relatório de Gestão Fiscal (RGF) dos entes federativos.
pub struct FonteSiconfiRgf;

impl FonteDeDados for FonteSiconfiRgf {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Tesouro Nacional / SICONFI - Relatório de Gestão Fiscal (RGF) por UF, exercício e quadrimestre"
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
                for quadrimestre in 1..=3u8 {
                    let recurso = format!("rgf/uf={uf}/ano={ano}/quadrimestre={quadrimestre}");
                    let url = format!("{}entes", api::BASE);
                    let tarefa = TarefaDeIngestaoHttp::paginada(
                        api::FONTE_RAW,
                        &recurso,
                        url,
                        PaginacaoRgf::new(uf, ano, quadrimestre),
                    )
                    .com_limite_de_paginas(LIMITE_DE_PAGINAS);
                    tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                        ("uf", *uf),
                        ("ano", &ano.to_string()),
                        ("quadrimestre", &quadrimestre.to_string()),
                    ]));
                }
            }
        }
        tarefas
    }
}
