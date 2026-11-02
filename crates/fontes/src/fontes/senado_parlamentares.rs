use chrono::{Datelike, Utc};
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

const NOME: &str = "senado-parlamentares";
const FONTE_RAW: &str = "senado";
const BASE: &str = "https://legis.senado.leg.br/dadosabertos/senador/lista/legislatura";
/// A 49ª legislatura (1991–1995) coincide com o início da série de votações nominais.
const PRIMEIRA_LEGISLATURA: u16 = 49;

fn legislatura_atual() -> u16 {
    // Legislaturas do Senado têm quatro anos; a 57ª começou em 2023.
    57 + ((Utc::now().year() - 2023) / 4) as u16
}

/// Catálogo de parlamentares por legislatura, com identificação partidária,
/// mandatos e UF conforme publicados pela API oficial.
pub struct FonteSenadoParlamentares;

impl FonteDeDados for FonteSenadoParlamentares {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Senado Federal - senadores, partidos, UFs e mandatos por legislatura desde 1991"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        (PRIMEIRA_LEGISLATURA..=legislatura_atual())
            .map(|legislatura| {
                let url = format!("{BASE}/{legislatura}");
                TarefaPlanejada::nova(TarefaDeIngestaoHttp::sem_paginacao(
                    FONTE_RAW,
                    &format!("parlamentares/legislatura={legislatura}"),
                    url,
                ))
                .com_etiquetas([("legislatura", legislatura.to_string().as_str())])
            })
            .collect()
    }
}
