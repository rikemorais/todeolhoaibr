use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;

const NOME: &str = "siconfi-entes";

/// Cadastro dos entes federativos informantes ao SICONFI (estados, municípios e União).
pub struct FonteSiconfiEntes;

impl FonteDeDados for FonteSiconfiEntes {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Tesouro Nacional / SICONFI - cadastro de entes federativos (estados, municípios e União)"
    }

    fn sob_demanda(&self) -> bool {
        false
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let url = format!("{}entes", api::BASE);
        let tarefa = TarefaDeIngestaoHttp::sem_paginacao(api::FONTE_RAW, "entes", url);
        vec![TarefaPlanejada::nova(tarefa)]
    }
}
