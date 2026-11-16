use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "comex";
const BASE: &str = "https://balanca.mdic.gov.br/balanca/bd/comexstat-bd";

const PRIMEIRO_ANO: u16 = 1997;
const ULTIMO_ANO: u16 = 2026;

struct Dataset {
    prefixo: &'static str,
    fluxo: &'static str,
    url_fmt: fn(u16) -> String,
}

const DATASETS: [Dataset; 4] = [
    Dataset {
        prefixo: "ncm",
        fluxo: "exp",
        url_fmt: |ano| format!("{BASE}/ncm/EXP_{ano}.csv"),
    },
    Dataset {
        prefixo: "ncm",
        fluxo: "imp",
        url_fmt: |ano| format!("{BASE}/ncm/IMP_{ano}.csv"),
    },
    Dataset {
        prefixo: "mun",
        fluxo: "exp",
        url_fmt: |ano| format!("{BASE}/mun/EXP_{ano}_MUN.csv"),
    },
    Dataset {
        prefixo: "mun",
        fluxo: "imp",
        url_fmt: |ano| format!("{BASE}/mun/IMP_{ano}_MUN.csv"),
    },
];

/// Comex Stat (MDIC) — comércio exterior brasileiro: exportação e importação
/// por NCM (com UF) e por município (SH4), série mensal desde 1997.
pub struct FonteComex;

impl FonteDeDados for FonteComex {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Comex Stat (MDIC) - comércio exterior por NCM e município, de 1997 a 2026"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "balanca.mdic.gov.br",
                requisicoes: 5,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: true,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        DATASETS
            .iter()
            .flat_map(|ds| {
                (PRIMEIRO_ANO..=ULTIMO_ANO).map(move |ano| {
                    let mut tarefa = TarefaDeArquivo::nova(
                        format!("{}_{}_{ano}", ds.prefixo, ds.fluxo),
                        NOME,
                        format!("{}/fluxo={}/ano={ano}", ds.prefixo, ds.fluxo),
                        (ds.url_fmt)(ano),
                    );
                    if ano == ULTIMO_ANO {
                        tarefa = tarefa.aceitar_ausencia();
                    }
                    TarefaPlanejada::nova(tarefa).com_etiquetas([
                        ("tipo", ds.prefixo),
                        ("fluxo", ds.fluxo),
                        ("ano", ano.to_string().as_str()),
                    ])
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn gera_120_tarefas() {
        let tarefas = FonteComex.criar_tarefas();
        let esperado = 4 * (ULTIMO_ANO - PRIMEIRO_ANO + 1) as usize;
        assert_eq!(tarefas.len(), esperado);
    }

    #[test]
    fn recurso_segue_padrao_hive() {
        let tarefas = FonteComex.criar_tarefas();
        let ncm_exp_2024 = tarefas
            .iter()
            .find(|t| t.tarefa.nome() == "ncm_exp_2024")
            .unwrap();
        assert_eq!(
            ncm_exp_2024.etiquetas.get("tipo").map(String::as_str),
            Some("ncm")
        );
        assert_eq!(
            ncm_exp_2024.etiquetas.get("fluxo").map(String::as_str),
            Some("exp")
        );
        assert_eq!(
            ncm_exp_2024.etiquetas.get("ano").map(String::as_str),
            Some("2024")
        );
    }

    #[test]
    fn tem_tarefa_ncm_exp_e_mun_imp() {
        let tarefas = FonteComex.criar_tarefas();
        assert!(tarefas.iter().any(|t| t.tarefa.nome() == "ncm_exp_2023"));
        assert!(tarefas.iter().any(|t| t.tarefa.nome() == "mun_imp_2020"));
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteComex.sob_demanda());
    }

    #[test]
    fn host_do_limite() {
        let limite = FonteComex.perfil_http().limite.expect("limite comex");
        assert_eq!(limite.host, "balanca.mdic.gov.br");
    }

    #[test]
    fn aceita_certificado_invalido() {
        assert!(FonteComex.perfil_http().aceitar_certificado_invalido);
    }
}
