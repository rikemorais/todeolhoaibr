//! Indicadores de desenvolvimento do World Bank Open Data, por país e ano.
//!
//! Cada indicador é um recurso que baixa **todos os valores** (todos os países e
//! períodos) em JSON da API v2, gravado opaco na Raw. O recorte por país e a
//! interpretação ficam no refino.

use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;

const NOME: &str = "world-bank";
const BASE: &str = "https://api.worldbank.org/v2/country/all/indicator";

struct Indicador {
    slug: &'static str,
    codigo: &'static str,
    tamanho_tipico: u64,
}

const INDICADORES: &[Indicador] = &[
    Indicador {
        slug: "pib-per-capita",
        codigo: "NY.GDP.PCAP.CD",
        tamanho_tipico: 3_800_000,
    },
    Indicador {
        slug: "pib",
        codigo: "NY.GDP.MKTP.CD",
        tamanho_tipico: 3_800_000,
    },
    Indicador {
        slug: "gini",
        codigo: "SI.POV.GINI",
        tamanho_tipico: 3_800_000,
    },
    Indicador {
        slug: "pobreza-extrema",
        codigo: "SI.POV.DDAY",
        tamanho_tipico: 3_800_000,
    },
    Indicador {
        slug: "populacao",
        codigo: "SP.POP.TOTL",
        tamanho_tipico: 700_000,
    },
    Indicador {
        slug: "expectativa-vida",
        codigo: "SP.DYN.LE00.IN",
        tamanho_tipico: 3_800_000,
    },
    Indicador {
        slug: "alfabetizacao",
        codigo: "SE.ADT.LITR.ZS",
        tamanho_tipico: 3_800_000,
    },
    Indicador {
        slug: "desemprego",
        codigo: "SL.UEM.TOTL.ZS",
        tamanho_tipico: 3_800_000,
    },
];

fn url(indicador: &Indicador) -> String {
    format!("{BASE}/{}?format=json&per_page=20000", indicador.codigo)
}

/// Indicadores de desenvolvimento por país do World Bank Open Data.
///
/// Uma consulta por indicador traz todos os valores (países e períodos) em JSON,
/// guardados opacos na Raw ([ADR 0014]). São indicadores publicados pelo Banco
/// Mundial para comparação internacional; o recorte por país é do refino
/// ([ADR 0015]). Ver issue #63.
///
/// Catálogo: <https://data.worldbank.org/indicator>
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteWorldBank;

impl FonteDeDados for FonteWorldBank {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "World Bank Open Data - indicadores de desenvolvimento por país"
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        INDICADORES
            .iter()
            .map(|indicador| {
                let recurso = format!("indicadores/{}", indicador.slug);
                TarefaPlanejada::nova(
                    TarefaDeIngestaoHttp::sem_paginacao(NOME, &recurso, url(indicador))
                        .com_tamanho_tipico(indicador.tamanho_tipico),
                )
                .com_etiquetas([("indicador", indicador.slug)])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn cada_indicador_vira_uma_tarefa_com_recurso_e_etiqueta() {
        let tarefas = FonteWorldBank.criar_tarefas();
        assert_eq!(tarefas.len(), INDICADORES.len());
        for (tarefa, indicador) in tarefas.iter().zip(INDICADORES) {
            assert_eq!(
                tarefa.tarefa.nome(),
                format!("indicadores/{}", indicador.slug)
            );
            assert_eq!(
                tarefa.etiquetas.get("indicador").map(String::as_str),
                Some(indicador.slug)
            );
        }
    }

    #[test]
    fn slugs_e_codigos_sao_unicos() {
        let slugs: BTreeSet<&str> = INDICADORES.iter().map(|i| i.slug).collect();
        let codigos: BTreeSet<&str> = INDICADORES.iter().map(|i| i.codigo).collect();
        assert_eq!(slugs.len(), INDICADORES.len(), "slug repetido");
        assert_eq!(codigos.len(), INDICADORES.len(), "código repetido");
    }

    #[test]
    fn a_url_aponta_para_o_endpoint_v2_com_per_page() {
        let pib = INDICADORES
            .iter()
            .find(|i| i.slug == "pib-per-capita")
            .unwrap();
        assert_eq!(
            url(pib),
            "https://api.worldbank.org/v2/country/all/indicator/NY.GDP.PCAP.CD?format=json&per_page=20000"
        );
    }

    #[test]
    fn nao_exige_credencial() {
        let perfil = FonteWorldBank.perfil_http();
        assert!(perfil.credencial.is_none());
    }

    #[test]
    fn declara_o_limite_do_host_do_world_bank() {
        let limite = FonteWorldBank
            .perfil_http()
            .limite
            .expect("limite do World Bank");
        assert_eq!(limite.host, "api.worldbank.org");
    }

    #[test]
    fn os_oito_indicadores_de_desenvolvimento_estao_presentes() {
        let slugs: Vec<&str> = INDICADORES.iter().map(|i| i.slug).collect();
        assert!(slugs.contains(&"pib-per-capita"), "PIB per capita");
        assert!(slugs.contains(&"pib"), "PIB total");
        assert!(slugs.contains(&"gini"), "Gini");
        assert!(slugs.contains(&"pobreza-extrema"), "Pobreza extrema");
        assert!(slugs.contains(&"populacao"), "População");
        assert!(slugs.contains(&"expectativa-vida"), "Expectativa de vida");
        assert!(slugs.contains(&"alfabetizacao"), "Alfabetização");
        assert!(slugs.contains(&"desemprego"), "Desemprego");
    }
}
