//! Arquivos mensais de remuneração dos servidores civis ativos do Executivo Federal.

use chrono::{Datelike, NaiveDate, Utc};
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "transparencia-servidores-ativos";
const FONTE_RAW: &str = "transparencia";
const BASE: &str = "https://portaldatransparencia.gov.br/download-de-dados/servidores";
const PRIMEIRA_COMPETENCIA: (i32, u32) = (2020, 1);

#[derive(Debug, Clone, Copy)]
enum Origem {
    Siape,
    Bacen,
}

impl Origem {
    fn slug(self) -> &'static str {
        match self {
            Self::Siape => "siape",
            Self::Bacen => "bacen",
        }
    }

    fn arquivo(self) -> &'static str {
        match self {
            Self::Siape => "Servidores_SIAPE",
            Self::Bacen => "Servidores_BACEN",
        }
    }
}

fn url(origem: Origem, competencia: &str) -> String {
    format!(
        "{BASE}/{}_{}",
        competencia.replace('-', ""),
        origem.arquivo()
    )
}

fn ultima_competencia_publicada() -> (i32, u32) {
    let mes = Utc::now().date_naive().with_day(1).expect("dia válido");
    let mes = mes
        .checked_sub_months(chrono::Months::new(2))
        .unwrap_or(mes);
    (mes.year(), mes.month())
}

fn competencias(inicio: (i32, u32), fim: (i32, u32)) -> Vec<String> {
    let mut atual = NaiveDate::from_ymd_opt(inicio.0, inicio.1, 1).expect("competência válida");
    let limite = NaiveDate::from_ymd_opt(fim.0, fim.1, 1).expect("competência válida");
    let mut resultado = Vec::new();
    while atual <= limite {
        resultado.push(format!("{}-{:02}", atual.year(), atual.month()));
        let Some(proximo) = atual.checked_add_months(chrono::Months::new(1)) else {
            break;
        };
        atual = proximo;
    }
    resultado
}

fn tarefas_ate(fim: (i32, u32)) -> Vec<TarefaPlanejada> {
    [Origem::Siape, Origem::Bacen]
        .into_iter()
        .flat_map(|origem| {
            competencias(PRIMEIRA_COMPETENCIA, fim)
                .into_iter()
                .map(move |competencia| {
                    let tarefa = TarefaDeArquivo::nova(
                        format!("{}_{}", origem.slug(), competencia),
                        FONTE_RAW,
                        format!(
                            "servidores-ativos/origem={}/competencia={competencia}",
                            origem.slug()
                        ),
                        url(origem, &competencia),
                    );
                    TarefaPlanejada::nova(tarefa).com_etiquetas([
                        ("origem", origem.slug()),
                        ("competencia", competencia.as_str()),
                    ])
                })
        })
        .collect()
}

/// Preserva os arquivos oficiais mensais por origem e competência na camada Raw.
pub struct FonteTransparenciaServidoresAtivos;

impl FonteDeDados for FonteTransparenciaServidoresAtivos {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Portal da Transparência - remuneração de servidores civis ativos do Executivo Federal por competência"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        tarefas_ate(ultima_competencia_publicada())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn gera_uma_tarefa_por_origem_e_competencia() {
        let tarefas = tarefas_ate((2020, 2));
        assert_eq!(tarefas.len(), 4);
        assert_eq!(tarefas[0].tarefa.nome(), "siape_2020-01");
        assert_eq!(tarefas[0].etiquetas["origem"], "siape");
        assert_eq!(tarefas[0].etiquetas["competencia"], "2020-01");
        assert_eq!(tarefas[1].tarefa.nome(), "siape_2020-02");
        assert_eq!(tarefas[2].tarefa.nome(), "bacen_2020-01");
        assert_eq!(tarefas[3].tarefa.nome(), "bacen_2020-02");
    }

    #[test]
    fn usa_urls_oficiais_das_planilhas_mensais() {
        assert_eq!(
            url(Origem::Siape, "2020-01"),
            "https://portaldatransparencia.gov.br/download-de-dados/servidores/202001_Servidores_SIAPE"
        );
        assert_eq!(
            url(Origem::Bacen, "2020-01"),
            "https://portaldatransparencia.gov.br/download-de-dados/servidores/202001_Servidores_BACEN"
        );
    }

    #[test]
    fn competencias_avancam_de_ano_sem_lacunas() {
        assert_eq!(
            competencias((2020, 11), (2021, 2)),
            ["2020-11", "2020-12", "2021-01", "2021-02"]
        );
    }

    #[test]
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteTransparenciaServidoresAtivos.sob_demanda());
    }
}
