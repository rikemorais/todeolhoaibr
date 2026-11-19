use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;
use crate::ingestao_http::TarefaDeIngestaoHttp;

const NOME: &str = "seguranca";

const BASE_IPEADATA: &str = "https://www.ipeadata.gov.br/api/odata4/Metadados";

struct SerieAviol {
    slug: &'static str,
    codigo: &'static str,
    tamanho_tipico: u64,
}

const SERIES_AVIOL: &[SerieAviol] = &[
    SerieAviol {
        slug: "homicidios",
        codigo: "AVIOL12_HOMIC",
        tamanho_tipico: 50_000_000,
    },
    SerieAviol {
        slug: "homicidios-feminino",
        codigo: "AVIOL12_HOMICF",
        tamanho_tipico: 50_000_000,
    },
    SerieAviol {
        slug: "homicidios-jovens",
        codigo: "AVIOL12_HOMICJ",
        tamanho_tipico: 50_000_000,
    },
    SerieAviol {
        slug: "homicidios-masculino",
        codigo: "AVIOL12_HOMICM",
        tamanho_tipico: 50_000_000,
    },
    SerieAviol {
        slug: "taxa-homicidios",
        codigo: "AVIOL12_THOMIC",
        tamanho_tipico: 50_000_000,
    },
    SerieAviol {
        slug: "suicidios",
        codigo: "AVIOL12_SUICID",
        tamanho_tipico: 50_000_000,
    },
    SerieAviol {
        slug: "taxa-suicidios",
        codigo: "AVIOL12_TSUICID",
        tamanho_tipico: 50_000_000,
    },
    SerieAviol {
        slug: "acidentes-transito",
        codigo: "AVIOL12_ACIDT",
        tamanho_tipico: 50_000_000,
    },
    SerieAviol {
        slug: "acidentes-transito-jovens",
        codigo: "AVIOL12_ACIDTJ",
        tamanho_tipico: 50_000_000,
    },
    SerieAviol {
        slug: "taxa-acidentes-transito",
        codigo: "AVIOL12_TACIDT",
        tamanho_tipico: 50_000_000,
    },
    SerieAviol {
        slug: "populacao",
        codigo: "AVIOL12_VIOLPOP",
        tamanho_tipico: 50_000_000,
    },
];

struct AnuarioFbsp {
    ano: u16,
    mes: &'static str,
}

const ANUARIOS_FBSP: &[AnuarioFbsp] = &[
    AnuarioFbsp {
        ano: 2022,
        mes: "06",
    },
    AnuarioFbsp {
        ano: 2023,
        mes: "07",
    },
    AnuarioFbsp {
        ano: 2024,
        mes: "07",
    },
    AnuarioFbsp {
        ano: 2025,
        mes: "09",
    },
    AnuarioFbsp {
        ano: 2026,
        mes: "07",
    },
];

fn url_aviol(serie: &SerieAviol) -> String {
    format!("{BASE_IPEADATA}('{}')/Valores", serie.codigo)
}

fn url_fbsp(anuario: &AnuarioFbsp) -> String {
    format!(
        "https://forumseguranca.org.br/wp-content/uploads/{}/{}/anuario-{}.xlsx",
        anuario.ano, anuario.mes, anuario.ano
    )
}

/// Segurança pública: Atlas da Violência (IPEA) e Anuário do FBSP.
///
/// Baixa 11 séries do Atlas da Violência via OData do Ipeadata (homicídios,
/// suicídios, acidentes de trânsito — por UF/município, série anual) e
/// 5 edições do Anuário Brasileiro de Segurança Pública em XLSX.
pub struct FonteSeguranca;

impl FonteDeDados for FonteSeguranca {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Segurança pública — Atlas da Violência (IPEA) e Anuário FBSP"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: None,
            limite: Some(LimiteHttp {
                host: "www.ipeadata.gov.br",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas: Vec<TarefaPlanejada> = SERIES_AVIOL
            .iter()
            .map(|serie| {
                let recurso = format!("atlas-violencia/{}", serie.slug);
                TarefaPlanejada::nova(
                    TarefaDeIngestaoHttp::sem_paginacao(NOME, &recurso, url_aviol(serie))
                        .com_tamanho_tipico(serie.tamanho_tipico),
                )
                .com_etiquetas([("origem", "ipea"), ("serie", serie.slug)])
            })
            .collect();

        tarefas.extend(ANUARIOS_FBSP.iter().map(|anuario| {
            let ano = anuario.ano.to_string();
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                format!("anuario-fbsp/{ano}"),
                NOME,
                format!("anuario-fbsp/{ano}"),
                url_fbsp(anuario),
            ))
            .com_etiquetas([("origem", "fbsp"), ("ano", ano.as_str())])
        }));

        tarefas
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;
    use todeolho_nucleo::FiltroDeTarefas;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteSeguranca.criar_tarefas()
    }

    #[test]
    fn total_16_tarefas() {
        assert_eq!(tarefas().len(), 16);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn onze_series_ipea_e_cinco_anuarios_fbsp() {
        let t = tarefas();
        let ipea = t.iter().filter(|t| t.etiquetas["origem"] == "ipea").count();
        let fbsp = t.iter().filter(|t| t.etiquetas["origem"] == "fbsp").count();
        assert_eq!(ipea, 11);
        assert_eq!(fbsp, 5);
    }

    #[test]
    fn filtra_por_origem_ipea() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["origem=ipea".to_owned()])
            .unwrap()
            .aplicar(tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 11);
    }

    #[test]
    fn filtra_por_origem_fbsp() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["origem=fbsp".to_owned()])
            .unwrap()
            .aplicar(tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 5);
    }

    #[test]
    fn url_da_serie_aponta_para_odata() {
        let serie = &SERIES_AVIOL[0];
        assert_eq!(
            url_aviol(serie),
            "https://www.ipeadata.gov.br/api/odata4/Metadados('AVIOL12_HOMIC')/Valores"
        );
    }

    #[test]
    fn url_do_anuario_aponta_para_xlsx() {
        let anuario = &ANUARIOS_FBSP[4];
        assert!(url_fbsp(anuario).ends_with("anuario-2026.xlsx"));
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            assert!(
                t.etiquetas.contains_key("origem"),
                "{} sem etiqueta origem",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteSeguranca.sob_demanda());
    }

    #[test]
    fn declara_limite_do_host() {
        let limite = FonteSeguranca
            .perfil_http()
            .limite
            .expect("limite do Ipeadata");
        assert_eq!(limite.host, "www.ipeadata.gov.br");
    }
}
