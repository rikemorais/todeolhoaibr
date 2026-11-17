use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

const NOME: &str = "ibge-agropecuaria";

const FONTE_RAW: &str = "ibge";
const BASE: &str = "https://apisidra.ibge.gov.br/values";

struct Pesquisa {
    slug: &'static str,
    tabela: u32,
    primeiro_ano: u16,
    ultimo_ano: u16,
    sufixo: &'static str,
    tamanho_tipico: u64,
}

const PESQUISAS: [Pesquisa; 5] = [
    Pesquisa {
        slug: "pam",
        tabela: 5457,
        primeiro_ano: 1974,
        ultimo_ano: 2025,
        sufixo: "/c782/0",
        tamanho_tipico: 14_000_000,
    },
    Pesquisa {
        slug: "ppm",
        tabela: 3939,
        primeiro_ano: 1974,
        ultimo_ano: 2025,
        sufixo: "/c79/0",
        tamanho_tipico: 1_600_000,
    },
    Pesquisa {
        slug: "lspa",
        tabela: 6588,
        primeiro_ano: 2006,
        ultimo_ano: 2026,
        sufixo: "",
        tamanho_tipico: 3_000_000,
    },
    Pesquisa {
        slug: "pevs",
        tabela: 289,
        primeiro_ano: 1986,
        ultimo_ano: 2025,
        sufixo: "",
        tamanho_tipico: 5_000_000,
    },
    Pesquisa {
        slug: "censo-agropecuario",
        tabela: 6881,
        primeiro_ano: 2017,
        ultimo_ano: 2017,
        sufixo: "",
        tamanho_tipico: 15_000_000,
    },
];

fn url_anual(tabela: u32, ano: u16, sufixo: &str) -> String {
    format!("{BASE}/t/{tabela}/n6/all/v/all/p/{ano}{sufixo}")
}

fn url_lspa(tabela: u32, ano: u16) -> String {
    let meses = (1..=12)
        .map(|m| format!("{ano}{m:02}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{BASE}/t/{tabela}/n6/all/v/all/p/{meses}")
}

fn tarefa(pesquisa: &Pesquisa, ano: u16) -> TarefaPlanejada {
    let recurso = format!("{}/ano={ano}", pesquisa.slug);
    let url = if pesquisa.slug == "lspa" {
        url_lspa(pesquisa.tabela, ano)
    } else {
        url_anual(pesquisa.tabela, ano, pesquisa.sufixo)
    };
    TarefaPlanejada::nova(
        TarefaDeIngestaoHttp::sem_paginacao(FONTE_RAW, &recurso, url)
            .com_tamanho_tipico(pesquisa.tamanho_tipico),
    )
    .com_etiquetas([("pesquisa", pesquisa.slug), ("ano", &ano.to_string())])
}

/// Pesquisas agropecuárias do IBGE via API SIDRA.
///
/// PAM (Produção Agrícola Municipal), PPM (Pecuária Municipal), LSPA
/// (Levantamento Sistemático da Produção Agrícola), PEVS (Extração Vegetal
/// e Silvicultura) e Censo Agropecuário, todas no nível municipal.
/// Dados JSON do SIDRA, sem autenticação.
pub struct FonteIbgeAgropecuaria;

impl FonteDeDados for FonteIbgeAgropecuaria {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "IBGE — pesquisas agropecuárias (PAM, PPM, LSPA, PEVS, Censo Agropecuário) via SIDRA"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "apisidra.ibge.gov.br",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        PESQUISAS
            .iter()
            .flat_map(|p| (p.primeiro_ano..=p.ultimo_ano).map(move |ano| tarefa(p, ano)))
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteIbgeAgropecuaria.criar_tarefas()
    }

    fn contagem(pesquisa: &str) -> usize {
        tarefas()
            .iter()
            .filter(|t| t.etiquetas.get("pesquisa").map(String::as_str) == Some(pesquisa))
            .count()
    }

    #[test]
    fn gera_166_tarefas() {
        assert_eq!(tarefas().len(), 166);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn tem_pam() {
        assert!(tarefas().iter().any(|t| t.tarefa.nome() == "pam/ano=2025"));
    }

    #[test]
    fn tem_ppm() {
        assert!(tarefas().iter().any(|t| t.tarefa.nome() == "ppm/ano=2025"));
    }

    #[test]
    fn tem_lspa() {
        assert!(tarefas().iter().any(|t| t.tarefa.nome() == "lspa/ano=2026"));
    }

    #[test]
    fn tem_pevs() {
        assert!(tarefas().iter().any(|t| t.tarefa.nome() == "pevs/ano=2025"));
    }

    #[test]
    fn tem_censo_agropecuario() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "censo-agropecuario/ano=2017")
        );
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteIbgeAgropecuaria.sob_demanda());
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            let etiq = &t.etiquetas;
            assert!(
                etiq.contains_key("pesquisa"),
                "{} sem etiqueta pesquisa",
                t.tarefa.nome()
            );
            assert!(
                etiq.contains_key("ano"),
                "{} sem etiqueta ano",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn contagem_por_pesquisa() {
        assert_eq!(contagem("pam"), 52);
        assert_eq!(contagem("ppm"), 52);
        assert_eq!(contagem("lspa"), 21);
        assert_eq!(contagem("pevs"), 40);
        assert_eq!(contagem("censo-agropecuario"), 1);
    }

    #[test]
    fn url_pam_esta_correta() {
        assert_eq!(
            url_anual(5457, 2025, "/c782/0"),
            "https://apisidra.ibge.gov.br/values/t/5457/n6/all/v/all/p/2025/c782/0"
        );
    }

    #[test]
    fn url_lspa_tem_12_meses() {
        let url = url_lspa(6588, 2026);
        assert!(url.contains(
            "202601,202602,202603,202604,202605,202606,202607,202608,202609,202610,202611,202612"
        ));
    }

    #[test]
    fn declara_limite_do_host_sidra() {
        let limite = FonteIbgeAgropecuaria
            .perfil_http()
            .limite
            .expect("limite do host");
        assert_eq!(limite.host, "apisidra.ibge.gov.br");
    }
}
