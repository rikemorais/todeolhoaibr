use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

const NOME: &str = "ibge";
const BASE: &str = "https://servicodados.ibge.gov.br/api/v1/localidades";

/// API de Localidades do IBGE: <https://servicodados.ibge.gov.br/api/docs/localidades>
pub struct FonteIbge;

impl FonteDeDados for FonteIbge {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "IBGE - Regiões, Estados e Municípios do Brasil"
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let tarefa = |recurso: &str, tamanho_tipico: u64| {
            TarefaDeIngestaoHttp::sem_paginacao(NOME, recurso, format!("{BASE}/{recurso}"))
                .com_tamanho_tipico(tamanho_tipico)
        };

        vec![
            TarefaPlanejada::nova(tarefa("regioes", 200)),
            TarefaPlanejada::nova(tarefa("estados", 2_500)).depois_de(["regioes"]),
            TarefaPlanejada::nova(tarefa("municipios", 2_500_000)).depois_de(["estados"]),
        ]
    }
}
