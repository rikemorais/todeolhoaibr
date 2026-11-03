//! Séries macroeconômicas do Banco Central via SGS (Sistema Gerenciador de Séries
//! Temporais): juros, câmbio, inflação, atividade, crédito e indicadores fiscais.
//!
//! Cada série é um recurso da fonte `bcb` que baixa a **série completa** (todos os
//! períodos) de um código do SGS, em JSON, gravada opaca na Raw. A interpretação do
//! JSON fica no refino.

use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;

const NOME: &str = "bcb";
const BASE: &str = "https://api.bcb.gov.br/dados/serie/bcdata.sgs";

/// Uma série do SGS: o `slug` (recurso na Raw e etiqueta de filtro), o código do
/// SGS e o tamanho típico da resposta, medido numa busca real.
struct Serie {
    slug: &'static str,
    codigo: u32,
    tamanho_tipico: u64,
}

/// Séries confirmadas na API do SGS em 2026-11. Só entram séries cuja história
/// completa cabe em uma única requisição; as de periodicidade **diária** (câmbio e
/// juros diários, reservas) exigem janela de até 10 anos e ficam para um recurso
/// próprio no futuro — aqui usamos as equivalentes mensais.
const SERIES: &[Serie] = &[
    // Juros
    Serie {
        slug: "selic-mensal",
        codigo: 4390,
        tamanho_tipico: 20_000,
    },
    Serie {
        slug: "cdi-mensal",
        codigo: 4391,
        tamanho_tipico: 20_000,
    },
    // Câmbio (média mensal)
    Serie {
        slug: "cambio-dolar-mensal",
        codigo: 3698,
        tamanho_tipico: 40_000,
    },
    // Inflação
    Serie {
        slug: "ipca-mensal",
        codigo: 433,
        tamanho_tipico: 25_000,
    },
    Serie {
        slug: "ipca-12-meses",
        codigo: 13522,
        tamanho_tipico: 25_000,
    },
    Serie {
        slug: "inpc-mensal",
        codigo: 188,
        tamanho_tipico: 25_000,
    },
    Serie {
        slug: "igpm-mensal",
        codigo: 189,
        tamanho_tipico: 20_000,
    },
    // Atividade
    Serie {
        slug: "ibc-br",
        codigo: 24364,
        tamanho_tipico: 15_000,
    },
    // Crédito
    Serie {
        slug: "inadimplencia-credito-total",
        codigo: 21082,
        tamanho_tipico: 10_000,
    },
    Serie {
        slug: "credito-total-pib",
        codigo: 20622,
        tamanho_tipico: 15_000,
    },
    // Fiscal
    Serie {
        slug: "divida-liquida-setor-publico-pib",
        codigo: 4513,
        tamanho_tipico: 15_000,
    },
];

fn url(serie: &Serie) -> String {
    format!("{BASE}.{}/dados?formato=json", serie.codigo)
}

/// Séries macroeconômicas do Banco Central (SGS).
///
/// Uma consulta por série traz a história completa em JSON, guardada opaca na Raw
/// ([ADR 0014]). A granularidade é temporal/nacional — é o contexto macro que situa
/// as demais fontes. Ver issue #56.
///
/// Séries: <https://www3.bcb.gov.br/sgspub/>
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
pub struct FonteBcb;

impl FonteDeDados for FonteBcb {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Banco Central - séries macro do SGS (juros, câmbio, inflação, atividade, crédito, fiscal)"
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
        let tarefas = FonteBcb.criar_tarefas();
        assert_eq!(tarefas.len(), SERIES.len());
        for (tarefa, serie) in tarefas.iter().zip(SERIES) {
            assert_eq!(tarefa.tarefa.nome(), format!("series/{}", serie.slug));
            assert_eq!(
                tarefa.etiquetas.get("serie").map(String::as_str),
                Some(serie.slug),
                "etiqueta `serie` diverge do slug em {}",
                serie.slug
            );
        }
    }

    #[test]
    fn os_slugs_sao_unicos() {
        let slugs: BTreeSet<&str> = SERIES.iter().map(|s| s.slug).collect();
        assert_eq!(slugs.len(), SERIES.len(), "há slug repetido em SERIES");
    }

    #[test]
    fn os_codigos_do_sgs_sao_unicos() {
        let codigos: BTreeSet<u32> = SERIES.iter().map(|s| s.codigo).collect();
        assert_eq!(codigos.len(), SERIES.len(), "há código repetido em SERIES");
    }

    #[test]
    fn a_url_aponta_para_a_serie_completa_em_json() {
        let selic = SERIES.iter().find(|s| s.slug == "selic-mensal").unwrap();
        assert_eq!(
            url(selic),
            "https://api.bcb.gov.br/dados/serie/bcdata.sgs.4390/dados?formato=json"
        );
    }

    #[test]
    fn declara_o_limite_do_host_do_bcb() {
        let limite = FonteBcb.perfil_http().limite.expect("limite do BCB");
        assert_eq!(limite.host, "api.bcb.gov.br");
    }
}
