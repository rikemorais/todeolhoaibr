use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "cvm";

const BASE_DFP: &str =
    "https://dados.cvm.gov.br/dados/CIA_ABERTA/DOC/DFP/DADOS";
const BASE_ITR: &str =
    "https://dados.cvm.gov.br/dados/CIA_ABERTA/DOC/ITR/DADOS";

const PRIMEIRO_ANO_DFP: u16 = 2010;
const PRIMEIRO_ANO_ITR: u16 = 2011;
const ULTIMO_ANO: u16 = 2026;

/// Demonstrações financeiras padronizadas (DFP, anual) e informações
/// trimestrais (ITR) das companhias abertas, publicadas pela CVM.
///
/// Cobre apenas empresas de **capital aberto** (listadas em bolsa). Cada ano
/// é um ZIP com ~19 CSVs (BPA, BPP, DRE, DFC, DMPL, DRA, DVA + metadados),
/// em Latin-1 com `;` como separador.
pub struct FonteCvm;

impl FonteDeDados for FonteCvm {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "CVM — demonstrações financeiras de companhias abertas (DFP/ITR), de 2010 a 2026"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: None,
            limite: Some(LimiteHttp {
                host: "dados.cvm.gov.br",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let dfp = (PRIMEIRO_ANO_DFP..=ULTIMO_ANO).map(|ano| {
            let mut tarefa = TarefaDeArquivo::nova(
                format!("dfp_{ano}"),
                NOME,
                format!("dfp/ano={ano}"),
                format!("{BASE_DFP}/dfp_cia_aberta_{ano}.zip"),
            );
            if ano == ULTIMO_ANO {
                tarefa = tarefa.aceitar_ausencia();
            }
            TarefaPlanejada::nova(tarefa)
                .com_etiquetas([("tipo", "dfp"), ("ano", ano.to_string().as_str())])
        });

        let itr = (PRIMEIRO_ANO_ITR..=ULTIMO_ANO).map(|ano| {
            let mut tarefa = TarefaDeArquivo::nova(
                format!("itr_{ano}"),
                NOME,
                format!("itr/ano={ano}"),
                format!("{BASE_ITR}/itr_cia_aberta_{ano}.zip"),
            );
            if ano == ULTIMO_ANO {
                tarefa = tarefa.aceitar_ausencia();
            }
            TarefaPlanejada::nova(tarefa)
                .com_etiquetas([("tipo", "itr"), ("ano", ano.to_string().as_str())])
        });

        dfp.chain(itr).collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn gera_tarefas_dfp_e_itr() {
        let tarefas = FonteCvm.criar_tarefas();
        let total_dfp = (ULTIMO_ANO - PRIMEIRO_ANO_DFP + 1) as usize;
        let total_itr = (ULTIMO_ANO - PRIMEIRO_ANO_ITR + 1) as usize;
        assert_eq!(tarefas.len(), total_dfp + total_itr);
    }

    #[test]
    fn primeira_tarefa_dfp_e_2010() {
        let tarefas = FonteCvm.criar_tarefas();
        assert_eq!(tarefas.first().unwrap().tarefa.nome(), "dfp_2010");
    }

    #[test]
    fn ultima_tarefa_itr_e_o_ano_corrente() {
        let tarefas = FonteCvm.criar_tarefas();
        assert_eq!(
            tarefas.last().unwrap().tarefa.nome(),
            format!("itr_{ULTIMO_ANO}")
        );
    }

    #[test]
    fn url_dfp_aponta_para_o_zip_anual() {
        let tarefas = FonteCvm.criar_tarefas();
        let t = tarefas
            .iter()
            .find(|t| t.tarefa.nome() == "dfp_2024")
            .unwrap();
        assert_eq!(
            t.etiquetas.get("tipo").map(String::as_str),
            Some("dfp")
        );
        assert_eq!(
            t.etiquetas.get("ano").map(String::as_str),
            Some("2024")
        );
    }

    #[test]
    fn url_itr_aponta_para_o_zip_anual() {
        let tarefas = FonteCvm.criar_tarefas();
        let t = tarefas
            .iter()
            .find(|t| t.tarefa.nome() == "itr_2024")
            .unwrap();
        assert_eq!(
            t.etiquetas.get("tipo").map(String::as_str),
            Some("itr")
        );
    }

    #[test]
    fn sob_demanda_e_true() {
        assert!(FonteCvm.sob_demanda());
    }

    #[test]
    fn nao_exige_credencial() {
        assert!(FonteCvm.perfil_http().credencial.is_none());
    }

    #[test]
    fn host_do_limite_e_dados_cvm_gov_br() {
        let limite = FonteCvm.perfil_http().limite.expect("limite da CVM");
        assert_eq!(limite.host, "dados.cvm.gov.br");
    }
}
