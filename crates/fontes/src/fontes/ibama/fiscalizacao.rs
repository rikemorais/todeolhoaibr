use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "ibama";

const BASE_SIFISC: &str =
    "https://stibamadadosabertosprd.blob.core.windows.net/dados-abertos/dados/SIFISC";

const BASE_AIE: &str =
    "https://stibamadadosabertosprd.blob.core.windows.net/dados-abertos/dados/AIE_MOB";

const BASE_EMBARGO: &str =
    "https://stibamadadosabertosprd.blob.core.windows.net/dados-abertos/dados/TERMOS_DE_EMBARGO";

struct Recurso {
    produto: &'static str,
    tabela: &'static str,
    caminho: &'static str,
}

const AUTOS: [Recurso; 8] = [
    Recurso {
        produto: "auto-infracao",
        tabela: "autos",
        caminho: "auto_infracao/auto_infracao/auto_infracao_csv.zip",
    },
    Recurso {
        produto: "auto-infracao",
        tabela: "enquadramento",
        caminho: "auto_infracao/enquadramento/enquadramento_csv.zip",
    },
    Recurso {
        produto: "auto-infracao",
        tabela: "aie-enquadramento",
        caminho: "aie_enquadramento",
    },
    Recurso {
        produto: "auto-infracao",
        tabela: "aie-enquadramento-complementar",
        caminho: "aie_enquadramentocomp",
    },
    Recurso {
        produto: "auto-infracao",
        tabela: "especimes",
        caminho: "auto_infracao/especime/especime.csv",
    },
    Recurso {
        produto: "auto-infracao",
        tabela: "coordenadas",
        caminho: "auto_infracao/coordenada/coordenada.csv",
    },
    Recurso {
        produto: "auto-infracao",
        tabela: "biomas",
        caminho: "auto_infracao/bioma/bioma.csv",
    },
    Recurso {
        produto: "auto-infracao",
        tabela: "anexos",
        caminho: "auto_infracao/anexo/anexo_csv.zip",
    },
];

const EMBARGOS: [Recurso; 8] = [
    Recurso {
        produto: "embargo",
        tabela: "termos",
        caminho: "TERMO_EMBARGO/termo_de_embargo.csv",
    },
    Recurso {
        produto: "embargo",
        tabela: "itens",
        caminho: "TERMO_EMBARGO_ITENS/termo_embargo_itens.csv",
    },
    Recurso {
        produto: "embargo",
        tabela: "coordenadas",
        caminho: "TERMO_EMBARGO_COORDENADAS/termo_de_embargo_coordenadas.csv",
    },
    Recurso {
        produto: "embargo",
        tabela: "anexos",
        caminho: "TERMO_EMBARGO_ANEXO/termo_de_embargo_anexo.csv",
    },
    Recurso {
        produto: "embargo",
        tabela: "decisoes",
        caminho: "TERMO_EMBARGO_DECISAO/termo_embargo_decisao.csv",
    },
    Recurso {
        produto: "embargo",
        tabela: "enquadramento",
        caminho: "TERMO_EMBARGO_ENQUADRAMENTO/termo_de_embargo_enquadramento.csv",
    },
    Recurso {
        produto: "embargo",
        tabela: "enquadramento-complementar",
        caminho: "TERMO_EMBARGO_ENQUADRAMENTO_COMPLEMENTAR/termo_de_embargo_enquadramento_complementar.csv",
    },
    Recurso {
        produto: "embargo",
        tabela: "historico",
        caminho: "TERMO_EMBARGO_HISTORICO/termo_embargo_historico.csv",
    },
];

fn url_auto(r: &Recurso) -> String {
    match r.tabela {
        "aie-enquadramento" => format!("{BASE_AIE}/auto_infracao/aie_enquadramento.csv"),
        "aie-enquadramento-complementar" => {
            format!("{BASE_AIE}/auto_infracao/aie_enquadramentocomp.csv")
        }
        _ => format!("{BASE_SIFISC}/{}", r.caminho),
    }
}

fn url_embargo(r: &Recurso) -> String {
    format!("{BASE_EMBARGO}/{}", r.caminho)
}

/// IBAMA — fiscalização ambiental federal.
///
/// Autos de infração (SIFISC + AIe) e termos de embargo, com tabelas
/// auxiliares (enquadramento, coordenadas, biomas, espécimes, anexos).
/// Dados públicos no Azure Blob Storage, atualização diária, CSV.
pub struct FonteIbama;

impl FonteDeDados for FonteIbama {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "IBAMA — fiscalização ambiental (autos de infração e termos de embargo)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "stibamadadosabertosprd.blob.core.windows.net",
                requisicoes: 1,
                intervalo: std::time::Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Bloquear,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let autos = AUTOS.iter().map(|r| {
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                format!("{}-{}", r.produto, r.tabela),
                NOME,
                format!("{}/{}", r.produto, r.tabela),
                url_auto(r),
            ))
            .com_etiquetas([("produto", r.produto), ("tabela", r.tabela)])
        });

        let embargos = EMBARGOS.iter().map(|r| {
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                format!("{}-{}", r.produto, r.tabela),
                NOME,
                format!("{}/{}", r.produto, r.tabela),
                url_embargo(r),
            ))
            .com_etiquetas([("produto", r.produto), ("tabela", r.tabela)])
        });

        autos.chain(embargos).collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteIbama.criar_tarefas()
    }

    #[test]
    fn gera_16_tarefas() {
        assert_eq!(tarefas().len(), 16);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn tem_autos_principal() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "auto-infracao-autos")
        );
    }

    #[test]
    fn tem_embargo_termos() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "embargo-termos")
        );
    }

    #[test]
    fn tem_embargo_historico() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "embargo-historico")
        );
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteIbama.sob_demanda());
    }

    #[test]
    fn nao_aceita_certificado_invalido() {
        assert!(!FonteIbama.perfil_http().aceitar_certificado_invalido);
    }

    #[test]
    fn bloqueia_redirecionamento() {
        assert!(matches!(
            FonteIbama.perfil_http().redirecionamento,
            todeolho_nucleo::RedirecionamentoHttp::Bloquear
        ));
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            let etiq = &t.etiquetas;
            assert!(
                etiq.contains_key("produto"),
                "{} sem etiqueta produto",
                t.tarefa.nome()
            );
            assert!(
                etiq.contains_key("tabela"),
                "{} sem etiqueta tabela",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn contagem_por_produto() {
        let t = tarefas();
        let autos = t
            .iter()
            .filter(|t| t.etiquetas.get("produto").map(|v| v.as_str()) == Some("auto-infracao"))
            .count();
        let embargos = t
            .iter()
            .filter(|t| t.etiquetas.get("produto").map(|v| v.as_str()) == Some("embargo"))
            .count();
        assert_eq!(autos, 8);
        assert_eq!(embargos, 8);
    }
}
