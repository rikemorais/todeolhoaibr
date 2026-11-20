//! Índices e cotações internacionais via FRED (Federal Reserve Economic Data).
//!
//! CSVs diários sem autenticação, um por série. Cobrem bolsas (S&P 500, Dow
//! Jones, NASDAQ, Nikkei), câmbio (USD/EUR, JPY/USD, BRL/USD, CNY/USD, CAD/USD,
//! CHF/USD, MXN/USD), volatilidade (VIX) e commodities (WTI, Brent).

use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

const NOME: &str = "bolsas-globais";
const HOST: &str = "fred.stlouisfed.org";

struct Serie {
    slug: &'static str,
    codigo_fred: &'static str,
    categoria: &'static str,
}

const SERIES: &[Serie] = &[
    Serie {
        slug: "sp500",
        codigo_fred: "SP500",
        categoria: "indice",
    },
    Serie {
        slug: "djia",
        codigo_fred: "DJIA",
        categoria: "indice",
    },
    Serie {
        slug: "nasdaq",
        codigo_fred: "NASDAQCOM",
        categoria: "indice",
    },
    Serie {
        slug: "nikkei225",
        codigo_fred: "NIKKEI225",
        categoria: "indice",
    },
    Serie {
        slug: "usd-eur",
        codigo_fred: "DEXUSEU",
        categoria: "cambio",
    },
    Serie {
        slug: "jpy-usd",
        codigo_fred: "DEXJPUS",
        categoria: "cambio",
    },
    Serie {
        slug: "brl-usd",
        codigo_fred: "DEXBZUS",
        categoria: "cambio",
    },
    Serie {
        slug: "cny-usd",
        codigo_fred: "DEXCHUS",
        categoria: "cambio",
    },
    Serie {
        slug: "cad-usd",
        codigo_fred: "DEXCAUS",
        categoria: "cambio",
    },
    Serie {
        slug: "chf-usd",
        codigo_fred: "DEXSZUS",
        categoria: "cambio",
    },
    Serie {
        slug: "mxn-usd",
        codigo_fred: "DEXMXUS",
        categoria: "cambio",
    },
    Serie {
        slug: "usd-index",
        codigo_fred: "DTWEXBGS",
        categoria: "cambio",
    },
    Serie {
        slug: "vix",
        codigo_fred: "VIXCLS",
        categoria: "volatilidade",
    },
    Serie {
        slug: "petroleo-wti",
        codigo_fred: "DCOILWTICO",
        categoria: "commodity",
    },
    Serie {
        slug: "petroleo-brent",
        codigo_fred: "DCOILBRENTEU",
        categoria: "commodity",
    },
];

fn url_fred(serie: &Serie) -> String {
    format!(
        "https://{HOST}/graph/fredgraph.csv?id={}",
        serie.codigo_fred
    )
}

/// Índices, câmbio e commodities internacionais via FRED (Federal Reserve).
pub struct FonteBolsasGlobais;

impl FonteDeDados for FonteBolsasGlobais {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "FRED — índices de bolsa, câmbio e commodities internacionais (CSV diário)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: None,
            limite: Some(LimiteHttp {
                host: HOST,
                requisicoes: 1,
                intervalo: Duration::from_secs(2),
            }),
            agente: None,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        SERIES
            .iter()
            .map(|serie| {
                TarefaPlanejada::nova(TarefaDeIngestaoHttp::sem_paginacao(
                    NOME,
                    serie.slug,
                    url_fred(serie),
                ))
                .com_etiquetas([
                    ("serie", serie.slug),
                    ("categoria", serie.categoria),
                    ("codigo-fred", serie.codigo_fred),
                ])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;
    use todeolho_nucleo::FiltroDeTarefas;

    #[test]
    fn total_de_tarefas() {
        assert_eq!(FonteBolsasGlobais.criar_tarefas().len(), SERIES.len());
        assert_eq!(SERIES.len(), 15);
    }

    #[test]
    fn nomes_sao_unicos() {
        let tarefas = FonteBolsasGlobais.criar_tarefas();
        let nomes: HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), tarefas.len());
    }

    #[test]
    fn urls_apontam_para_fred() {
        for serie in SERIES {
            let url = url_fred(serie);
            assert!(url.starts_with("https://fred.stlouisfed.org/graph/fredgraph.csv"));
            assert!(url.contains(serie.codigo_fred));
        }
    }

    #[test]
    fn filtra_por_categoria() {
        let tarefas = FonteBolsasGlobais.criar_tarefas();
        let filtro = FiltroDeTarefas::a_partir_de(&["categoria=indice".to_owned()]).unwrap();
        let filtradas = filtro.aplicar(tarefas).unwrap();
        assert_eq!(filtradas.len(), 4);
    }

    #[test]
    fn filtra_por_serie() {
        let tarefas = FonteBolsasGlobais.criar_tarefas();
        let filtro = FiltroDeTarefas::a_partir_de(&["serie=sp500".to_owned()]).unwrap();
        let filtradas = filtro.aplicar(tarefas).unwrap();
        assert_eq!(filtradas.len(), 1);
        assert_eq!(filtradas[0].tarefa.nome(), "sp500");
    }

    #[test]
    fn etiquetas_presentes() {
        for tarefa in FonteBolsasGlobais.criar_tarefas() {
            assert!(tarefa.etiquetas.contains_key("serie"));
            assert!(tarefa.etiquetas.contains_key("categoria"));
            assert!(tarefa.etiquetas.contains_key("codigo-fred"));
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteBolsasGlobais.sob_demanda());
    }

    #[test]
    fn declara_limite_e_sem_credencial() {
        let perfil = FonteBolsasGlobais.perfil_http();
        assert_eq!(perfil.limite.as_ref().unwrap().host, HOST);
        assert!(perfil.credencial.is_none());
    }
}
