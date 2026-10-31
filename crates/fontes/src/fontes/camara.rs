use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;
use crate::paginacao::LinksDePaginacao;

const NOME: &str = "camara";
const BASE: &str = "https://dadosabertos.camara.leg.br/api/v2";

/// Dados Abertos da Câmara dos Deputados: <https://dadosabertos.camara.leg.br/swagger/api.html>
pub struct FonteCamara;

impl FonteDeDados for FonteCamara {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Câmara dos Deputados - legislaturas, partidos e deputados em exercício"
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let tarefa = |recurso: &str, tamanho_tipico: u64| {
            TarefaDeIngestaoHttp::paginada(
                NOME,
                recurso,
                format!("{BASE}/{recurso}?itens=100"),
                LinksDePaginacao,
            )
            .com_tamanho_tipico(tamanho_tipico)
        };

        vec![
            TarefaPlanejada::nova(tarefa("legislaturas", 8_000)),
            TarefaPlanejada::nova(tarefa("partidos", 3_000)),
            TarefaPlanejada::nova(tarefa("deputados", 190_000))
                .depois_de(["legislaturas", "partidos"]),
        ]
    }
}
