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

const PESQUISAS_MUNICIPAIS: [Pesquisa; 4] = [
    Pesquisa {
        slug: "pam",
        tabela: 5457,
        primeiro_ano: 1974,
        ultimo_ano: 2025,
        sufixo: "/c782/0",
        tamanho_tipico: 14_000_000,
    },
    Pesquisa {
        slug: "ppm-rebanhos",
        tabela: 3939,
        primeiro_ano: 1974,
        ultimo_ano: 2025,
        sufixo: "/c79/0",
        tamanho_tipico: 1_600_000,
    },
    Pesquisa {
        slug: "ppm-leite",
        tabela: 74,
        primeiro_ano: 1974,
        ultimo_ano: 2025,
        sufixo: "",
        tamanho_tipico: 1_600_000,
    },
    Pesquisa {
        slug: "pevs",
        tabela: 289,
        primeiro_ano: 1986,
        ultimo_ano: 2025,
        sufixo: "",
        tamanho_tipico: 5_000_000,
    },
];

const LSPA: Pesquisa = Pesquisa {
    slug: "lspa",
    tabela: 6588,
    primeiro_ano: 2006,
    ultimo_ano: 2026,
    sufixo: "",
    tamanho_tipico: 3_000_000,
};

fn url_anual(tabela: u32, ano: u16, sufixo: &str) -> String {
    format!("{BASE}/t/{tabela}/n6/all/v/all/p/{ano}{sufixo}")
}

fn url_lspa(tabela: u32, ano: u16) -> String {
    let meses = (1..=12)
        .map(|m| format!("{ano}{m:02}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{BASE}/t/{tabela}/n3/all/v/all/p/{meses}")
}

fn tarefa_municipal(pesquisa: &Pesquisa, ano: u16) -> TarefaPlanejada {
    let recurso = format!("{}/ano={ano}", pesquisa.slug);
    let url = url_anual(pesquisa.tabela, ano, pesquisa.sufixo);
    TarefaPlanejada::nova(
        TarefaDeIngestaoHttp::sem_paginacao(FONTE_RAW, &recurso, url)
            .com_tamanho_tipico(pesquisa.tamanho_tipico),
    )
    .com_etiquetas([("pesquisa", pesquisa.slug), ("ano", &ano.to_string())])
}

fn tarefa_lspa(ano: u16) -> TarefaPlanejada {
    let recurso = format!("{}/ano={ano}", LSPA.slug);
    let url = url_lspa(LSPA.tabela, ano);
    TarefaPlanejada::nova(
        TarefaDeIngestaoHttp::sem_paginacao(FONTE_RAW, &recurso, url)
            .com_tamanho_tipico(LSPA.tamanho_tipico),
    )
    .com_etiquetas([("pesquisa", LSPA.slug), ("ano", &ano.to_string())])
}

/// Pesquisas agropecuárias do IBGE via API SIDRA.
///
/// PAM, PPM-rebanhos e PPM-leite, PEVS no nível municipal (`N6`);
/// LSPA no nível estadual (`N3`) — a tabela 6588 não suporta `N6`.
/// Dados JSON do SIDRA, sem autenticação.
pub struct FonteIbgeAgropecuaria;

impl FonteDeDados for FonteIbgeAgropecuaria {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "IBGE — pesquisas agropecuárias (PAM, PPM, LSPA, PEVS) via SIDRA"
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
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Bloquear,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let municipais = PESQUISAS_MUNICIPAIS
            .iter()
            .flat_map(|p| (p.primeiro_ano..=p.ultimo_ano).map(move |ano| tarefa_municipal(p, ano)));
        let lspa = (LSPA.primeiro_ano..=LSPA.ultimo_ano).map(tarefa_lspa);
        municipais.chain(lspa).collect()
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
    fn gera_217_tarefas() {
        assert_eq!(tarefas().len(), 217);
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
    fn tem_ppm_rebanhos() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "ppm-rebanhos/ano=2025")
        );
    }

    #[test]
    fn tem_ppm_leite() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "ppm-leite/ano=2025")
        );
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
        assert_eq!(contagem("ppm-rebanhos"), 52);
        assert_eq!(contagem("ppm-leite"), 52);
        assert_eq!(contagem("lspa"), 21);
        assert_eq!(contagem("pevs"), 40);
    }

    #[test]
    fn url_pam_esta_correta() {
        assert_eq!(
            url_anual(5457, 2025, "/c782/0"),
            "https://apisidra.ibge.gov.br/values/t/5457/n6/all/v/all/p/2025/c782/0"
        );
    }

    #[test]
    fn url_lspa_esta_correta() {
        assert_eq!(
            url_lspa(6588, 2026),
            "https://apisidra.ibge.gov.br/values/t/6588/n3/all/v/all/p/202601,202602,202603,202604,202605,202606,202607,202608,202609,202610,202611,202612"
        );
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
