//! Demografia das Empresas — natalidade, mortalidade e sobrevivência (IBGE/SIDRA).
//!
//! A API SIDRA (`apisidra.ibge.gov.br/values`) publica quatro tabelas da pesquisa
//! Demografia das Empresas e Estatísticas de Empreendedorismo. Os dados cobrem apenas
//! **empresas empregadoras** (ao menos 1 assalariado no ano de referência).
//!
//! Limitação da API: as tabelas por UF (9925, 9950) não trazem porte; as que trazem
//! porte (9924, 9949) são apenas nível Brasil. Ingerimos as quatro para cruzar depois.

use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

const NOME: &str = "ibge-demografia-empresas";
const FONTE_RAW: &str = "ibge";
const BASE: &str = "https://apisidra.ibge.gov.br/values";

const TAMANHO_TIPICO: u64 = 150_000;

struct Tabela {
    recurso: &'static str,
    tabela: u32,
    caminho: &'static str,
}

const TABELAS: &[Tabela] = &[
    Tabela {
        recurso: "demografia-empresas/natalidade-mortalidade-uf",
        tabela: 9925,
        caminho: "/n3/all/v/allxp/p/all/c12762/117897/c371/all",
    },
    Tabela {
        recurso: "demografia-empresas/sobrevivencia-uf",
        tabela: 9950,
        caminho: "/n3/all/v/allxp/p/all/c12762/117897",
    },
    Tabela {
        recurso: "demografia-empresas/natalidade-mortalidade-porte",
        tabela: 9924,
        caminho: "/n1/all/v/allxp/p/all/c370/all/c12762/117897/c369/all",
    },
    Tabela {
        recurso: "demografia-empresas/sobrevivencia-porte",
        tabela: 9949,
        caminho: "/n1/all/v/allxp/p/all/c370/all/c12762/117897",
    },
];

/// Demografia das Empresas — natalidade, mortalidade e sobrevivência (IBGE).
///
/// Quatro tabelas SIDRA: nascimento/morte de unidades locais por UF (9925),
/// sobrevivência por UF (9950), nascimento/morte de empresas por porte (9924) e
/// sobrevivência por porte (9949). CNAE fixado em Total (117897).
pub struct FonteIbgeDemografiaEmpresas;

impl FonteDeDados for FonteIbgeDemografiaEmpresas {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "IBGE — Demografia das Empresas: natalidade, mortalidade e sobrevivência (SIDRA)"
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: None,
            limite: Some(LimiteHttp {
                host: "apisidra.ibge.gov.br",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        TABELAS
            .iter()
            .map(|t| {
                let url = format!("{BASE}/t/{}{}", t.tabela, t.caminho);
                TarefaPlanejada::nova(
                    TarefaDeIngestaoHttp::sem_paginacao(FONTE_RAW, t.recurso, url)
                        .com_tamanho_tipico(TAMANHO_TIPICO),
                )
                .com_etiquetas([("tabela", t.tabela.to_string().as_str())])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn gera_quatro_tarefas() {
        let tarefas = FonteIbgeDemografiaEmpresas.criar_tarefas();
        assert_eq!(tarefas.len(), 4);
    }

    #[test]
    fn recursos_tem_prefixo_correto() {
        let tarefas = FonteIbgeDemografiaEmpresas.criar_tarefas();
        for t in &tarefas {
            assert!(
                t.tarefa.nome().starts_with("demografia-empresas/"),
                "recurso inesperado: {}",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn tabelas_sidra_corretas() {
        let tarefas = FonteIbgeDemografiaEmpresas.criar_tarefas();
        let tabelas: Vec<&str> = tarefas
            .iter()
            .map(|t| t.etiquetas["tabela"].as_str())
            .collect();
        assert_eq!(tabelas, ["9925", "9950", "9924", "9949"]);
    }

    #[test]
    fn tabelas_por_uf_pedem_n3() {
        let tarefas = FonteIbgeDemografiaEmpresas.criar_tarefas();
        let uf: Vec<_> = tarefas
            .iter()
            .filter(|t| t.tarefa.nome().contains("-uf"))
            .collect();
        assert_eq!(uf.len(), 2);
    }

    #[test]
    fn host_do_limite_e_apisidra() {
        let limite = FonteIbgeDemografiaEmpresas
            .perfil_http()
            .limite
            .expect("limite do SIDRA");
        assert_eq!(limite.host, "apisidra.ibge.gov.br");
    }

    #[test]
    fn nao_e_sob_demanda() {
        assert!(!FonteIbgeDemografiaEmpresas.sob_demanda());
    }
}
