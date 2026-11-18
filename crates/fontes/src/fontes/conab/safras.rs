//! Séries de safra de grãos da CONAB (boletins de levantamento).
//!
//! A CONAB publica, a cada mês do calendário agrícola (outubro a setembro), um
//! levantamento com estimativas de área, produtividade e produção por produto e
//! UF. O arquivo é um XLSX com ~76 abas: uma por produto, uma por UF, resumos
//! nacionais e de suprimento.
//!
//! Cada levantamento é um recurso independente. Safras futuras ou levantamentos
//! ainda não publicados usam `aceitar_ausencia`.

use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "conab";

const BASE: &str = "https://www.gov.br/conab/pt-br/atuacao/informacoes-agropecuarias\
                     /safras/safra-de-graos/boletim-da-safra-de-graos";

const PRIMEIRA_SAFRA: u16 = 2024;
const ULTIMA_SAFRA: u16 = 2026;

const MESES: [&str; 12] = [
    "out", "nov", "dez", "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set",
];

fn url_levantamento(safra_inicio: u16, levantamento: u8) -> String {
    let safra_fim = safra_inicio + 1;
    let yy = safra_fim % 100;
    let mes = MESES[(levantamento - 1) as usize];
    let ano_civil = if levantamento <= 3 {
        safra_inicio
    } else {
        safra_fim
    };
    format!(
        "{BASE}/{levantamento}o-levantamento-safra-{safra_inicio}-{yy:02}\
         /site_previsao_de_safra-por_produto-{mes}-{ano_civil}.xlsx/%40%40download/file"
    )
}

fn ano_civil_publicacao(safra_inicio: u16, levantamento: u8) -> u16 {
    if levantamento <= 3 {
        safra_inicio
    } else {
        safra_inicio + 1
    }
}

/// Safras de grãos da CONAB — levantamentos mensais (XLSX).
///
/// Cada levantamento (1–12) cobre uma safra (ex: 2025/26). O 1º sai em outubro
/// do ano inicial e o 12º em setembro do ano seguinte. As safras de 2024/25 a
/// 2026/27 são cobertas, com `aceitar_ausencia` em levantamentos futuros.
pub struct FonteConab;

impl FonteDeDados for FonteConab {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "CONAB — séries de safra de grãos (levantamentos mensais, XLSX)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: None,
            limite: Some(LimiteHttp {
                host: "www.gov.br",
                requisicoes: 5,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let ano_atual: u16 = 2026;
        let mes_atual: u8 = 11;

        (PRIMEIRA_SAFRA..=ULTIMA_SAFRA)
            .flat_map(|safra| {
                (1u8..=12).filter_map(move |lev| {
                    let ano_pub = ano_civil_publicacao(safra, lev);
                    let mes_pub = match lev {
                        1 => 10,
                        2 => 11,
                        3 => 12,
                        n => n as u16 - 3,
                    };

                    // Descartar levantamentos claramente no futuro distante
                    if ano_pub > ano_atual + 1 {
                        return None;
                    }
                    if ano_pub == ano_atual + 1 && mes_pub > 9 {
                        return None;
                    }

                    let recurso = format!(
                        "safras/safra={safra}-{yy:02}/levantamento={lev:02}",
                        yy = (safra + 1) % 100
                    );
                    let url = url_levantamento(safra, lev);
                    let futuro =
                        ano_pub > ano_atual || (ano_pub == ano_atual && mes_pub > mes_atual as u16);
                    let mut tarefa = TarefaDeArquivo::nova(
                        format!("safra-{safra}-{lev:02}"),
                        NOME,
                        recurso,
                        url,
                    );
                    if futuro {
                        tarefa = tarefa.aceitar_ausencia();
                    }
                    Some(TarefaPlanejada::nova(tarefa).com_etiquetas([
                        (
                            "safra",
                            format!("{safra}/{yy:02}", yy = (safra + 1) % 100).as_str(),
                        ),
                        ("levantamento", lev.to_string().as_str()),
                    ]))
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn gera_tarefas_para_tres_safras() {
        let tarefas = FonteConab.criar_tarefas();
        assert!(
            tarefas.len() >= 26,
            "esperava >= 26 tarefas, obteve {}",
            tarefas.len()
        );
        assert!(
            tarefas.len() <= 36,
            "esperava <= 36 tarefas, obteve {}",
            tarefas.len()
        );
    }

    #[test]
    fn recursos_seguem_o_padrao() {
        let tarefas = FonteConab.criar_tarefas();
        for t in &tarefas {
            assert!(
                t.tarefa.nome().starts_with("safra-"),
                "nome inesperado: {}",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn url_do_primeiro_levantamento_2025_26() {
        assert_eq!(
            url_levantamento(2025, 1),
            "https://www.gov.br/conab/pt-br/atuacao/informacoes-agropecuarias/safras/safra-de-graos/boletim-da-safra-de-graos/1o-levantamento-safra-2025-26/site_previsao_de_safra-por_produto-out-2025.xlsx/%40%40download/file"
        );
    }

    #[test]
    fn url_da_safra_atual_usa_o_download_oficial_plone() {
        assert_eq!(
            url_levantamento(2026, 1),
            "https://www.gov.br/conab/pt-br/atuacao/informacoes-agropecuarias/safras/safra-de-graos/boletim-da-safra-de-graos/1o-levantamento-safra-2026-27/site_previsao_de_safra-por_produto-out-2026.xlsx/%40%40download/file"
        );
    }

    #[test]
    fn url_do_decimo_segundo_levantamento_2025_26() {
        let url = url_levantamento(2025, 12);
        assert!(url.contains("12o-levantamento-safra-2025-26"), "URL: {url}");
        assert!(url.contains("set-2026.xlsx"), "URL: {url}");
    }

    #[test]
    fn host_do_limite() {
        let limite = FonteConab.perfil_http().limite.expect("limite definido");
        assert_eq!(limite.host, "www.gov.br");
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteConab.sob_demanda());
    }
}
