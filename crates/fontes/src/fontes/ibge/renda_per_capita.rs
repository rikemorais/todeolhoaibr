//! Renda domiciliar per capita dos municípios (IBGE/Censo via API SIDRA).
//!
//! A API SIDRA (`apisidra.ibge.gov.br/values`) publica o rendimento nominal médio
//! e mediano mensal domiciliar per capita por município, em tabelas distintas por
//! edição do Censo. Os censos de 2010 (tabela 3578) e 2022 (tabela 10295) estão
//! disponíveis no nível municipal (N6); os de 1991 e 2000 não estão no SIDRA.
//!
//! Cada edição do censo é uma tarefa própria. A resposta JSON fica opaca na Raw;
//! o refino tipará os valores na Silver.

use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

const NOME: &str = "ibge-renda-per-capita";
const FONTE_RAW: &str = "ibge";
const BASE: &str = "https://apisidra.ibge.gov.br/values";

/// Censo 2010 — tabela 3578 (rendimento domiciliar per capita, amostra).
const TABELA_2010: u32 = 3578;
/// Variáveis de 2010: média (2012), mediana (2013), moradores (2035).
const VARIAVEIS_2010: &str = "2012,2013,2035";

/// Censo 2022 — tabela 10295 (rendimento domiciliar per capita, amostra).
const TABELA_2022: u32 = 10295;
/// Variáveis de 2022: média (13431), mediana (13534), moradores (13604).
const VARIAVEIS_2022: &str = "13431,13534,13604";
/// Classificações fixadas em "Total" para 2022 (sexo, faixa etária, cor/raça).
const CLASSIFICACOES_2022: &str = "/c2/6794/c287/95253/c86/95251";

const TAMANHO_TIPICO: u64 = 2_000_000;

fn url_2010() -> String {
    format!("{BASE}/t/{TABELA_2010}/n6/all/v/{VARIAVEIS_2010}/p/all")
}

fn url_2022() -> String {
    format!("{BASE}/t/{TABELA_2022}/n6/all/v/{VARIAVEIS_2022}/p/all{CLASSIFICACOES_2022}")
}

fn tarefa(censo: u16, url: String) -> TarefaPlanejada {
    let recurso = format!("renda-per-capita/censo={censo}");
    TarefaPlanejada::nova(
        TarefaDeIngestaoHttp::sem_paginacao(FONTE_RAW, &recurso, url)
            .com_tamanho_tipico(TAMANHO_TIPICO),
    )
    .com_etiquetas([
        ("censo", censo.to_string().as_str()),
        ("indicador", "renda-per-capita"),
    ])
}

/// Renda domiciliar per capita dos municípios (IBGE/Censo).
///
/// Captura o rendimento nominal médio e mediano mensal domiciliar per capita, por
/// município (N6), dos Censos 2010 e 2022 via API SIDRA. Os Censos de 1991 e 2000
/// não estão disponíveis no SIDRA no nível municipal. Fonte separada e `sob_demanda`
/// (duas tarefas). Os dados ficam opacos na Raw ([ADR 0014]); a tipagem e o
/// cruzamento com dados eleitorais são do refino ([ADR 0015]).
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteIbgeRendaPerCapita;

impl FonteDeDados for FonteIbgeRendaPerCapita {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "IBGE - Rendimento domiciliar per capita por município (Censos 2010 e 2022, API SIDRA)"
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
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        vec![tarefa(2010, url_2010()), tarefa(2022, url_2022())]
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn cobre_os_censos_2010_e_2022() {
        let censos: BTreeSet<u16> = FonteIbgeRendaPerCapita
            .criar_tarefas()
            .iter()
            .map(|t| t.etiquetas["censo"].parse().expect("censo numérico"))
            .collect();
        assert_eq!(censos, [2010, 2022].into());
    }

    #[test]
    fn a_url_de_2010_pede_todos_os_municipios() {
        let url = url_2010();
        assert!(url.contains("/t/3578/"), "tabela 3578");
        assert!(url.contains("/n6/all"), "nível municipal");
        assert!(url.contains("/v/2012,2013,2035"), "três variáveis");
    }

    #[test]
    fn a_url_de_2022_inclui_classificacoes_de_total() {
        let url = url_2022();
        assert!(url.contains("/t/10295/"), "tabela 10295");
        assert!(url.contains("/n6/all"), "nível municipal");
        assert!(url.contains("/v/13431,13534,13604"), "três variáveis");
        assert!(url.contains("/c2/6794"), "sexo = Total");
        assert!(url.contains("/c287/95253"), "faixa etária = Total");
        assert!(url.contains("/c86/95251"), "cor/raça = Total");
    }

    #[test]
    fn cada_tarefa_tem_censo_coerente_com_o_recurso() {
        for tarefa in FonteIbgeRendaPerCapita.criar_tarefas() {
            let nome = tarefa.tarefa.nome();
            let censo = &tarefa.etiquetas["censo"];
            assert_eq!(nome, format!("renda-per-capita/censo={censo}"));
        }
    }

    #[test]
    fn declara_o_limite_do_host_do_sidra() {
        let limite = FonteIbgeRendaPerCapita
            .perfil_http()
            .limite
            .expect("limite do SIDRA");
        assert_eq!(limite.host, "apisidra.ibge.gov.br");
    }

    #[test]
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteIbgeRendaPerCapita.sob_demanda());
    }
}
