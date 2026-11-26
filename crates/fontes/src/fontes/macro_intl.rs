use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;
use crate::ingestao_http::TarefaDeIngestaoHttp;

const NOME: &str = "macro_intl";

struct RecursoImf {
    slug: &'static str,
    url: &'static str,
}

const IMF: &[RecursoImf] = &[
    RecursoImf {
        slug: "weo-oct-2026",
        url: "https://data.imf.org/-/media/iData/External-Storage/Documents/0FA3DC49A04D446E9D69DBEB29979E1C/en/WEOOct2026all.xlsx",
    },
    RecursoImf {
        slug: "weo-historical",
        url: "https://data.imf.org/-/media/iData/External-Storage/Documents/D2AC2BB4C12C4A8CB2B6DAF1159263EB/en/WEOhistorical.xlsx",
    },
];

struct RecursoOecd {
    slug: &'static str,
    agency: &'static str,
    dataflow: &'static str,
    key: &'static str,
    tamanho_tipico: u64,
}

const OECD: &[RecursoOecd] = &[
    RecursoOecd {
        slug: "economic-outlook",
        agency: "OECD.ECO.MAD",
        dataflow: "DSD_EO@DF_EO",
        key: "..A",
        tamanho_tipico: 110_000_000,
    },
    RecursoOecd {
        slug: "gdp-expenditure",
        agency: "OECD.SDD.NAD",
        dataflow: "DSD_NAMAIN10@DF_TABLE1_EXPENDITURE",
        key: "A...........",
        tamanho_tipico: 50_000_000,
    },
    RecursoOecd {
        slug: "cpi",
        agency: "OECD.SDD.TPS",
        dataflow: "DSD_PRICES@DF_PRICES_ALL",
        key: ".A.......",
        tamanho_tipico: 80_000_000,
    },
    RecursoOecd {
        slug: "unemployment",
        agency: "OECD.SDD.TPS",
        dataflow: "DSD_LFS@DF_IALFS_UNE_M",
        key: "........M",
        tamanho_tipico: 170_000_000,
    },
    RecursoOecd {
        slug: "balance-of-payments",
        agency: "OECD.SDD.TPS",
        dataflow: "DSD_BOP@DF_BOP",
        key: "........",
        tamanho_tipico: 50_000_000,
    },
];

fn url_oecd(recurso: &RecursoOecd) -> String {
    format!(
        "https://sdmx.oecd.org/public/rest/data/{},{},/{}?format=csvfilewithlabels",
        recurso.agency, recurso.dataflow, recurso.key
    )
}

/// Macro internacional comparado — IMF (WEO) e OECD (Economic Outlook, GDP,
/// CPI, desemprego, balanço de pagamentos) por país e ano.
pub struct FonteMacroIntl;

impl FonteDeDados for FonteMacroIntl {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Macro comparado — IMF (WEO) e OECD (Economic Outlook, GDP, CPI, desemprego, BoP)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: None,
            limite: Some(LimiteHttp {
                host: "sdmx.oecd.org",
                requisicoes: 1,
                intervalo: Duration::from_secs(2),
            }),
            // data.imf.org (Akamai WAF) rejeita o user-agent padrão do projeto.
            agente: Some("curl/8.0"),
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas: Vec<TarefaPlanejada> = Vec::new();

        for recurso in IMF {
            tarefas.push(
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    format!("imf/{}", recurso.slug),
                    NOME,
                    format!("imf/{}", recurso.slug),
                    recurso.url.to_owned(),
                ))
                .com_etiquetas([("orgao", "imf"), ("recurso", recurso.slug)]),
            );
        }

        for recurso in OECD {
            tarefas.push(
                TarefaPlanejada::nova(
                    TarefaDeIngestaoHttp::sem_paginacao(
                        NOME,
                        &format!("oecd/{}", recurso.slug),
                        url_oecd(recurso),
                    )
                    .com_tamanho_tipico(recurso.tamanho_tipico),
                )
                .com_etiquetas([("orgao", "oecd"), ("recurso", recurso.slug)]),
            );
        }

        tarefas
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;
    use todeolho_nucleo::FiltroDeTarefas;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteMacroIntl.criar_tarefas()
    }

    #[test]
    fn total_7_tarefas() {
        assert_eq!(tarefas().len(), 7);
    }

    #[test]
    fn dois_imf_cinco_oecd() {
        let t = tarefas();
        let imf = t.iter().filter(|t| t.etiquetas["orgao"] == "imf").count();
        let oecd = t.iter().filter(|t| t.etiquetas["orgao"] == "oecd").count();
        assert_eq!(imf, 2);
        assert_eq!(oecd, 5);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn filtra_por_orgao_imf() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["orgao=imf".to_owned()])
            .unwrap()
            .aplicar(tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 2);
    }

    #[test]
    fn filtra_por_orgao_oecd() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["orgao=oecd".to_owned()])
            .unwrap()
            .aplicar(tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 5);
    }

    #[test]
    fn url_do_oecd_usa_sdmx() {
        let url = url_oecd(&OECD[0]);
        assert!(url.starts_with("https://sdmx.oecd.org/public/rest/data/"));
        assert!(url.contains("csvfilewithlabels"));
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            assert!(
                t.etiquetas.contains_key("orgao"),
                "{} sem etiqueta orgao",
                t.tarefa.nome()
            );
            assert!(
                t.etiquetas.contains_key("recurso"),
                "{} sem etiqueta recurso",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteMacroIntl.sob_demanda());
    }

    #[test]
    fn declara_limite_oecd() {
        let limite = FonteMacroIntl.perfil_http().limite.expect("limite OECD");
        assert_eq!(limite.host, "sdmx.oecd.org");
    }

    #[test]
    fn nao_precisa_credencial() {
        assert!(FonteMacroIntl.perfil_http().credencial.is_none());
    }

    #[test]
    fn agente_compativel_com_imf() {
        assert!(FonteMacroIntl.perfil_http().agente.is_some());
    }
}
