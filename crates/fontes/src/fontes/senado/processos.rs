use chrono::{Datelike, Utc};
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;

const NOME: &str = "senado-processos";
const FONTE_RAW: &str = "senado";
const BASE: &str = "https://legis.senado.leg.br/dadosabertos/processo";
/// Ano de instalação do Senado Federal.
const PRIMEIRO_ANO: u16 = 1826;

fn ultimo_ano() -> u16 {
    Utc::now().year() as u16
}

/// Catálogo anual de processos legislativos do Senado, incluindo matérias que
/// nunca foram votadas. As respostas são preservadas opacas na camada Raw.
pub struct FonteSenadoProcessos;

impl FonteDeDados for FonteSenadoProcessos {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Senado Federal - catálogo de processos legislativos, uma consulta por ano desde 1826"
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        (PRIMEIRO_ANO..=ultimo_ano())
            .map(|ano| {
                let url = format!("{BASE}?ano={ano}");
                TarefaPlanejada::nova(TarefaDeIngestaoHttp::sem_paginacao(
                    FONTE_RAW,
                    &format!("processos/ano={ano}"),
                    url,
                ))
                .com_etiquetas([("ano", ano.to_string().as_str())])
            })
            .collect()
    }
}
