use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "energia";

const PRIMEIRO_ANO_ONS: u16 = 2000;
const ULTIMO_ANO_ONS: u16 = 2026;

struct RecursoAneel {
    slug: &'static str,
    url: &'static str,
}

const ANEEL: &[RecursoAneel] = &[
    RecursoAneel {
        slug: "tarifas",
        url: "https://dadosabertos.aneel.gov.br/dataset/5a583f3e-1646-4f67-bf0f-69db4203e89e/resource/fcf2906c-7c32-4b9b-a637-054e7a5234f4/download/tarifas-homologadas-distribuidoras-energia-eletrica.csv",
    },
    RecursoAneel {
        slug: "siga-empreendimentos",
        url: "https://dadosabertos.aneel.gov.br/dataset/6d90b77c-c5f5-4d81-bdec-7bc619494bb9/resource/11ec447d-698d-4ab8-977f-b424d5deee6a/download/siga-empreendimentos-geracao.csv",
    },
    RecursoAneel {
        slug: "geracao-distribuida",
        url: "https://dadosabertos.aneel.gov.br/dataset/5e0fafd2-21b9-4d5b-b622-40438d40aba2/resource/b1bd71e7-d0ad-4214-9053-cbd58e9564a7/download/empreendimento-geracao-distribuida.zip",
    },
];

struct RecursoEpe {
    slug: &'static str,
    url: &'static str,
}

const EPE: &[RecursoEpe] = &[
    RecursoEpe {
        slug: "consumo-mensal-por-classe",
        url: "https://www.epe.gov.br/sites-pt/publicacoes-dados-abertos/publicacoes/Documents/CONSUMO%20MENSAL%20DE%20ENERGIA%20EL%C3%89TRICA%20POR%20CLASSE.xlsx",
    },
    RecursoEpe {
        slug: "ben-dados-agregados",
        url: "https://www.epe.gov.br/sites-pt/publicacoes-dados-abertos/publicacoes/PublicacoesArquivos/publicacao-145/topico-515/Cap%C3%ADtulo%201%20(An%C3%A1lise%20Energ%C3%A9tica%20e%20Dados%20Agregados)%20-%201970%20a%202025.xlsx",
    },
    RecursoEpe {
        slug: "ben-oferta-demanda",
        url: "https://www.epe.gov.br/sites-pt/publicacoes-dados-abertos/publicacoes/PublicacoesArquivos/publicacao-145/topico-515/Cap%C3%ADtulo%202%20(Oferta%20e%20Demanda%20de%20Energia%20por%20Fonte)%20-%201970%20a%202025.xlsx",
    },
    RecursoEpe {
        slug: "ben-consumo-por-setor",
        url: "https://www.epe.gov.br/sites-pt/publicacoes-dados-abertos/publicacoes/PublicacoesArquivos/publicacao-145/topico-515/Cap%C3%ADtulo%203%20(Consumo%20de%20Energia%20por%20Setor)%20-%201970%20a%202025.xlsx",
    },
    RecursoEpe {
        slug: "ben-dados-estaduais",
        url: "https://www.epe.gov.br/sites-pt/publicacoes-dados-abertos/publicacoes/PublicacoesArquivos/publicacao-145/topico-515/Cap%C3%ADtulo%208%20(Dados%20Estaduais)%20-%202000%20a%202025.xlsx",
    },
    RecursoEpe {
        slug: "ben-capacidade-instalada",
        url: "https://www.epe.gov.br/sites-pt/publicacoes-dados-abertos/publicacoes/PublicacoesArquivos/publicacao-145/topico-515/Anexo%20I%20-%20Capacidade%20instalada.xlsx",
    },
];

fn url_ons(ano: u16) -> String {
    format!(
        "https://ons-aws-prod-opendata.s3.amazonaws.com/dataset/\
         balanco_energia_subsistema_ho/BALANCO_ENERGIA_SUBSISTEMA_{ano}.csv"
    )
}

/// Energia: ANEEL (tarifas, geração distribuída, empreendimentos), EPE (BEN,
/// consumo) e ONS (balanço de carga e geração por subsistema).
pub struct FonteEnergia;

impl FonteDeDados for FonteEnergia {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Energia — ANEEL (tarifas, geração distribuída, SIGA), EPE (BEN, consumo) e ONS (balanço por subsistema)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: None,
            limite: Some(LimiteHttp {
                host: "dadosabertos.aneel.gov.br",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas: Vec<TarefaPlanejada> = Vec::new();

        for recurso in ANEEL {
            tarefas.push(
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    format!("aneel/{}", recurso.slug),
                    NOME,
                    format!("aneel/{}", recurso.slug),
                    recurso.url.to_owned(),
                ))
                .com_etiquetas([("orgao", "aneel"), ("recurso", recurso.slug)]),
            );
        }

        for recurso in EPE {
            tarefas.push(
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    format!("epe/{}", recurso.slug),
                    NOME,
                    format!("epe/{}", recurso.slug),
                    recurso.url.to_owned(),
                ))
                .com_etiquetas([("orgao", "epe"), ("recurso", recurso.slug)]),
            );
        }

        for ano in PRIMEIRO_ANO_ONS..=ULTIMO_ANO_ONS {
            let ano_str = ano.to_string();
            tarefas.push(
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    format!("ons/balanco-subsistema/{ano_str}"),
                    NOME,
                    format!("ons/balanco-subsistema/{ano_str}"),
                    url_ons(ano),
                ))
                .com_etiquetas([("orgao", "ons"), ("ano", ano_str.as_str())]),
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
        FonteEnergia.criar_tarefas()
    }

    #[test]
    fn total_36_tarefas() {
        assert_eq!(tarefas().len(), 36);
    }

    #[test]
    fn tres_aneel_seis_epe_vinte_sete_ons() {
        let t = tarefas();
        let aneel = t.iter().filter(|t| t.etiquetas["orgao"] == "aneel").count();
        let epe = t.iter().filter(|t| t.etiquetas["orgao"] == "epe").count();
        let ons = t.iter().filter(|t| t.etiquetas["orgao"] == "ons").count();
        assert_eq!(aneel, 3);
        assert_eq!(epe, 6);
        assert_eq!(ons, 27);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn filtra_por_orgao_aneel() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["orgao=aneel".to_owned()])
            .unwrap()
            .aplicar(tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 3);
    }

    #[test]
    fn filtra_por_orgao_epe() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["orgao=epe".to_owned()])
            .unwrap()
            .aplicar(tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 6);
    }

    #[test]
    fn filtra_por_orgao_ons() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["orgao=ons".to_owned()])
            .unwrap()
            .aplicar(tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 27);
    }

    #[test]
    fn url_do_ons_segue_padrao_s3() {
        let url = url_ons(2026);
        assert!(url.contains("ons-aws-prod-opendata.s3.amazonaws.com"));
        assert!(url.ends_with("BALANCO_ENERGIA_SUBSISTEMA_2026.csv"));
    }

    #[test]
    fn url_das_tarifas_aponta_para_csv() {
        assert!(ANEEL[0].url.ends_with(".csv"));
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            assert!(
                t.etiquetas.contains_key("orgao"),
                "{} sem etiqueta orgao",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteEnergia.sob_demanda());
    }

    #[test]
    fn declara_limite_do_host() {
        let limite = FonteEnergia.perfil_http().limite.expect("limite ANEEL");
        assert_eq!(limite.host, "dadosabertos.aneel.gov.br");
    }

    #[test]
    fn nao_precisa_credencial() {
        assert!(FonteEnergia.perfil_http().credencial.is_none());
    }
}
