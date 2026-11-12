//! Séries socioeconômicas regionais do Ipeadata (IPEA): IDHM, renda, desigualdade
//! (Gini), pobreza e vulnerabilidade social (IVS), já consolidadas por território.
//!
//! Cada série é um recurso da fonte `ipeadata` que baixa **todos os valores** (todos os
//! territórios e períodos) de um código do Ipeadata, em JSON (OData), gravado opaco na
//! Raw. O recorte por município/UF e a interpretação ficam no refino.

use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;

const NOME: &str = "ipeadata";
const BASE: &str = "https://www.ipeadata.gov.br/api/odata4/Metadados";

/// Uma série do Ipeadata: o `slug` (recurso na Raw e etiqueta), o código Ipeadata
/// (`SERCODIGO`) e o tamanho típico da resposta, medido numa busca real.
struct Serie {
    slug: &'static str,
    codigo: &'static str,
    tamanho_tipico: u64,
}

/// Séries confirmadas no catálogo do Ipeadata em 2026-11, todas municipais do Atlas do
/// Desenvolvimento Humano (Censo), exceto o IVS, por região/UF.
const SERIES: &[Serie] = &[
    Serie {
        slug: "renda-per-capita",
        codigo: "ADH_RDPC",
        tamanho_tipico: 2_400_000,
    },
    Serie {
        slug: "gini",
        codigo: "ADH_GINI",
        tamanho_tipico: 2_400_000,
    },
    Serie {
        slug: "proporcao-pobres",
        codigo: "ADH_PMPOB",
        tamanho_tipico: 2_400_000,
    },
    Serie {
        slug: "proporcao-extremamente-pobres",
        codigo: "ADH_PIND",
        tamanho_tipico: 2_400_000,
    },
    Serie {
        slug: "ivs",
        codigo: "AVS_IVS",
        tamanho_tipico: 70_000,
    },
    Serie {
        slug: "idhm",
        codigo: "ADH_IDHM",
        tamanho_tipico: 2_400_000,
    },
    Serie {
        slug: "idhm-educacao",
        codigo: "ADH_IDHM_E",
        tamanho_tipico: 2_400_000,
    },
    Serie {
        slug: "idhm-longevidade",
        codigo: "ADH_IDHM_L",
        tamanho_tipico: 2_400_000,
    },
    Serie {
        slug: "idhm-renda",
        codigo: "ADH_IDHM_R",
        tamanho_tipico: 2_400_000,
    },
];

fn url(serie: &Serie) -> String {
    format!("{BASE}('{}')/Valores", serie.codigo)
}

/// Séries socioeconômicas regionais do Ipeadata.
///
/// Uma consulta por série traz todos os valores (territórios e períodos) em JSON OData,
/// guardados opacos na Raw ([ADR 0014]). São séries já derivadas pelo IPEA a partir do
/// Censo/PNAD; a procedência fica registrada no nome do recurso e o recorte por
/// município/UF é do refino ([ADR 0015]). Ver issue #32.
///
/// Catálogo: <http://www.ipeadata.gov.br>
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteIpeadata;

impl FonteDeDados for FonteIpeadata {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Ipeadata - séries socioeconômicas regionais (IDHM, renda, Gini, pobreza, IVS)"
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        SERIES
            .iter()
            .map(|serie| {
                let recurso = format!("series/{}", serie.slug);
                TarefaPlanejada::nova(
                    TarefaDeIngestaoHttp::sem_paginacao(NOME, &recurso, url(serie))
                        .com_tamanho_tipico(serie.tamanho_tipico),
                )
                .com_etiquetas([("serie", serie.slug)])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn cada_serie_vira_uma_tarefa_com_recurso_e_etiqueta() {
        let tarefas = FonteIpeadata.criar_tarefas();
        assert_eq!(tarefas.len(), SERIES.len());
        for (tarefa, serie) in tarefas.iter().zip(SERIES) {
            assert_eq!(tarefa.tarefa.nome(), format!("series/{}", serie.slug));
            assert_eq!(
                tarefa.etiquetas.get("serie").map(String::as_str),
                Some(serie.slug)
            );
        }
    }

    #[test]
    fn slugs_e_codigos_sao_unicos() {
        let slugs: BTreeSet<&str> = SERIES.iter().map(|s| s.slug).collect();
        let codigos: BTreeSet<&str> = SERIES.iter().map(|s| s.codigo).collect();
        assert_eq!(slugs.len(), SERIES.len(), "slug repetido");
        assert_eq!(codigos.len(), SERIES.len(), "código repetido");
    }

    #[test]
    fn a_url_aponta_para_os_valores_da_serie_no_odata() {
        let gini = SERIES.iter().find(|s| s.slug == "gini").unwrap();
        assert_eq!(
            url(gini),
            "https://www.ipeadata.gov.br/api/odata4/Metadados('ADH_GINI')/Valores"
        );
    }

    #[test]
    fn a_url_do_idhm_aponta_para_o_codigo_adh_idhm() {
        let idhm = SERIES.iter().find(|s| s.slug == "idhm").unwrap();
        assert_eq!(
            url(idhm),
            "https://www.ipeadata.gov.br/api/odata4/Metadados('ADH_IDHM')/Valores"
        );
    }

    #[test]
    fn as_quatro_dimensoes_do_idhm_estao_presentes() {
        let slugs: Vec<&str> = SERIES.iter().map(|s| s.slug).collect();
        assert!(slugs.contains(&"idhm"), "IDHM geral");
        assert!(slugs.contains(&"idhm-educacao"), "IDHM educação");
        assert!(slugs.contains(&"idhm-longevidade"), "IDHM longevidade");
        assert!(slugs.contains(&"idhm-renda"), "IDHM renda");
    }

    #[test]
    fn declara_o_limite_do_host_do_ipeadata() {
        let limite = FonteIpeadata
            .perfil_http()
            .limite
            .expect("limite do Ipeadata");
        assert_eq!(limite.host, "www.ipeadata.gov.br");
    }
}
