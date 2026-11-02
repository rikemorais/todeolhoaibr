//! Indicadores macro do IBGE via SIDRA/agregados v3: inflação (IPCA e INPC),
//! crescimento (PIB trimestral) e mercado de trabalho (PNAD Contínua).
//!
//! Cada indicador é um recurso da fonte `ibge` que baixa a **série completa**
//! (todos os períodos) de uma variável de um agregado do SIDRA, gravada opaca
//! na Raw. A interpretação do JSON fica no refino.

use todeolho_nucleo::TarefaPlanejada;

use crate::ingestao_http::TarefaDeIngestaoHttp;

const NOME: &str = "ibge";
const BASE: &str = "https://servicodados.ibge.gov.br/api/v3/agregados";

/// Um indicador: o recurso na Raw e as coordenadas do SIDRA (agregado, variável
/// e nível territorial — `N1` Brasil ou `N3` por UF).
struct Indicador {
    recurso: &'static str,
    agregado: u32,
    variavel: u32,
    nivel: &'static str,
    tamanho_tipico: u64,
}

/// Agregados confirmados nos metadados do SIDRA (v3) em 2026-11.
const INDICADORES: &[Indicador] = &[
    // IPCA — variação mensal, mensal, Brasil. Tabela SIDRA 1737.
    Indicador {
        recurso: "ipca",
        agregado: 1737,
        variavel: 63,
        nivel: "N1",
        tamanho_tipico: 60_000,
    },
    // INPC — variação mensal, mensal, Brasil. Tabela SIDRA 1736.
    Indicador {
        recurso: "inpc",
        agregado: 1736,
        variavel: 44,
        nivel: "N1",
        tamanho_tipico: 60_000,
    },
    // PIB trimestral — índice de volume encadeado, Brasil. Tabela SIDRA 1620.
    Indicador {
        recurso: "pib-trimestral",
        agregado: 1620,
        variavel: 583,
        nivel: "N1",
        tamanho_tipico: 20_000,
    },
    // PNAD Contínua — taxa de desocupação, trimestral, por UF. Tabela SIDRA 4093.
    Indicador {
        recurso: "pnad-desocupacao",
        agregado: 4093,
        variavel: 4099,
        nivel: "N3",
        tamanho_tipico: 200_000,
    },
];

/// URL da série completa do indicador (todos os períodos) no SIDRA v3. Os
/// colchetes de `N<nivel>[all]` vão percent-encoded para o cliente HTTP.
fn url(indicador: &Indicador) -> String {
    format!(
        "{BASE}/{}/periodos/all/variaveis/{}?localidades={}%5Ball%5D",
        indicador.agregado, indicador.variavel, indicador.nivel
    )
}

/// Recursos de indicadores macro da fonte `ibge`.
pub fn tarefas() -> Vec<TarefaPlanejada> {
    INDICADORES
        .iter()
        .map(|indicador| {
            TarefaPlanejada::nova(
                TarefaDeIngestaoHttp::sem_paginacao(NOME, indicador.recurso, url(indicador))
                    .com_tamanho_tipico(indicador.tamanho_tipico),
            )
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use todeolho_nucleo::FonteDeDados;

    use super::*;

    #[test]
    fn cada_indicador_vira_uma_tarefa() {
        let tarefas = tarefas();
        assert_eq!(tarefas.len(), INDICADORES.len());
        let recursos: Vec<&str> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        for indicador in INDICADORES {
            assert!(
                recursos.contains(&indicador.recurso),
                "falta o recurso {}",
                indicador.recurso
            );
        }
    }

    #[test]
    fn url_usa_agregados_v3_com_todos_os_periodos() {
        let ipca = INDICADORES.iter().find(|i| i.recurso == "ipca").unwrap();
        assert_eq!(
            url(ipca),
            "https://servicodados.ibge.gov.br/api/v3/agregados/1737/periodos/all/variaveis/63?localidades=N1%5Ball%5D"
        );
        let pnad = INDICADORES
            .iter()
            .find(|i| i.recurso == "pnad-desocupacao")
            .unwrap();
        assert!(url(pnad).ends_with("localidades=N3%5Ball%5D"));
    }

    #[test]
    fn fonte_ibge_inclui_indicadores_e_localidades() {
        let recursos: Vec<String> = crate::FonteIbge
            .criar_tarefas()
            .iter()
            .map(|t| t.tarefa.nome().to_owned())
            .collect();
        assert!(recursos.iter().any(|r| r == "ipca"));
        assert!(recursos.iter().any(|r| r == "pnad-desocupacao"));
        assert!(recursos.iter().any(|r| r == "municipios"));
    }
}
