use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

use super::api;

const NOME: &str = "b3";
const BASE: &str = "https://bvmf.bmfbovespa.com.br/InstDados/SerHist";

const PRIMEIRO_ANO: u16 = 1986;
const ULTIMO_ANO: u16 = 2026;

/// COTAHIST — cotações históricas diárias da B3 (ações, ETFs, BDRs, UNITs,
/// FIIs, FIAGROs). Um ZIP por ano, formato fixed-width definido pela B3.
///
/// O vínculo geográfico (UF da sede) não vem neste arquivo: exige join com o
/// cadastro de companhias da CVM (issue #37) ou a base CNPJ (issue #34) pelo
/// CNPJ da empresa. Ver issue #53.
pub struct FonteB3;

impl FonteDeDados for FonteB3 {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "B3 COTAHIST - cotações históricas diárias da bolsa, de 1986 a 2026"
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        (PRIMEIRO_ANO..=ULTIMO_ANO)
            .map(|ano| {
                let mut tarefa = TarefaDeArquivo::nova(
                    format!("cotahist_{ano}"),
                    NOME,
                    format!("cotahist/ano={ano}"),
                    format!("{BASE}/COTAHIST_A{ano}.ZIP"),
                );
                if ano == ULTIMO_ANO {
                    tarefa = tarefa.aceitar_ausencia();
                }
                TarefaPlanejada::nova(tarefa).com_etiquetas([("ano", ano.to_string().as_str())])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn gera_uma_tarefa_por_ano() {
        let tarefas = FonteB3.criar_tarefas();
        let total = (ULTIMO_ANO - PRIMEIRO_ANO + 1) as usize;
        assert_eq!(tarefas.len(), total);
    }

    #[test]
    fn primeira_tarefa_e_1986_e_ultima_e_o_ano_corrente() {
        let tarefas = FonteB3.criar_tarefas();
        assert_eq!(tarefas.first().unwrap().tarefa.nome(), "cotahist_1986");
        assert_eq!(
            tarefas.last().unwrap().tarefa.nome(),
            format!("cotahist_{ULTIMO_ANO}")
        );
    }

    #[test]
    fn url_aponta_para_o_zip_anual_da_b3() {
        let tarefas = FonteB3.criar_tarefas();
        let etiqueta_2023 = tarefas
            .iter()
            .find(|t| t.tarefa.nome() == "cotahist_2023")
            .unwrap();
        assert_eq!(
            etiqueta_2023.etiquetas.get("ano").map(String::as_str),
            Some("2023")
        );
    }

    #[test]
    fn nao_exige_credencial() {
        assert!(FonteB3.perfil_http().credencial.is_none());
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteB3.sob_demanda());
    }

    #[test]
    fn host_do_limite_e_bmfbovespa() {
        let limite = FonteB3.perfil_http().limite.expect("limite da B3");
        assert_eq!(limite.host, "bvmf.bmfbovespa.com.br");
    }
}
