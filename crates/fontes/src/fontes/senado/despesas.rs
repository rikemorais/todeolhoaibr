use chrono::{Datelike, Utc};
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "senado-despesas";

/// A Raw guarda as despesas sob a fonte `senado` (`raw/senado/despesas/...`),
/// embora no catálogo elas sejam uma fonte separada, para poderem ser `sob_demanda`.
const FONTE_RAW: &str = "senado";
/// Arquivos anuais da CEAPS (Cota para o Exercício da Atividade Parlamentar dos Senadores),
/// publicados pela Transparência do Senado em CSV nacional por ano.
const BASE_CEAPS: &str = "https://www.senado.leg.br/transparencia/LAI/verba";
/// Primeiro ano com arquivo anual da CEAPS publicado.
const PRIMEIRO_ANO_CEAPS: u16 = 2008;

fn ultimo_ano() -> u16 {
    Utc::now().year() as u16
}

fn tarefas_ate(ano_final: u16) -> Vec<TarefaPlanejada> {
    (PRIMEIRO_ANO_CEAPS..=ano_final)
        .map(|ano| {
            let mut tarefa = TarefaDeArquivo::nova(
                format!("despesas_{ano}"),
                FONTE_RAW,
                format!("despesas/ano={ano}"),
                format!("{BASE_CEAPS}/despesa_ceaps_{ano}.csv"),
            );
            if ano == ano_final {
                tarefa = tarefa.aceitar_ausencia();
            }
            TarefaPlanejada::nova(tarefa).com_etiquetas([("ano", ano.to_string().as_str())])
        })
        .collect()
}

/// Despesas da CEAPS (Cota para o Exercício da Atividade Parlamentar dos Senadores).
///
/// Equivalente da CEAP da Câmara para a casa alta. Um arquivo CSV por ano, nacional,
/// baixado como arquivo grande. O arquivo é guardado opaco na Raw ([ADR 0014]); a UF é
/// coluna do CSV, então o filtro aqui é só por ano, e a deduplicação por chave composta
/// (a CEAPS não tem ID único por despesa) fica para o refino ([ADR 0015]). Fonte separada
/// da `senado` para ser `sob_demanda` e não entrar em `--todas`. Ver issue #26.
///
/// O layout das colunas difere do da Câmara (ver a issue), mas isso é problema do refino:
/// a ingestão preserva o CSV como veio.
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteSenadoDespesas;

impl FonteDeDados for FonteSenadoDespesas {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Senado Federal - despesas da CEAPS, um arquivo por ano desde 2008"
    }

    /// Os arquivos anuais em lote não devem entrar em `--todas` por acidente.
    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        tarefas_ate(ultimo_ano())
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    /// Anos cobertos pelas tarefas, lidos do nome `despesas_<ano>`.
    fn anos() -> BTreeSet<u16> {
        FonteSenadoDespesas
            .criar_tarefas()
            .iter()
            .map(|t| {
                t.tarefa
                    .nome()
                    .strip_prefix("despesas_")
                    .and_then(|ano| ano.parse().ok())
                    .expect("nome no formato despesas_<ano>")
            })
            .collect()
    }

    #[test]
    fn cobre_exatamente_um_arquivo_por_ano() {
        // Conjunto exato (não só a contagem e os extremos): pega ano faltando no meio.
        let esperado: BTreeSet<u16> = (PRIMEIRO_ANO_CEAPS..=ultimo_ano()).collect();
        assert_eq!(anos(), esperado);
    }

    #[test]
    fn inclui_o_ano_novo_sem_edicao_manual() {
        let anos: BTreeSet<_> = tarefas_ate(2027)
            .into_iter()
            .map(|tarefa| tarefa.etiquetas["ano"].clone())
            .collect();
        assert!(anos.contains("2027"));
        assert!(!anos.contains("2028"));
    }

    #[test]
    fn a_etiqueta_ano_bate_com_o_ano_do_nome() {
        for tarefa in FonteSenadoDespesas.criar_tarefas() {
            let ano = tarefa
                .tarefa
                .nome()
                .strip_prefix("despesas_")
                .expect("nome no formato despesas_<ano>");
            // Não basta a etiqueta existir: seu valor precisa ser o ano do arquivo,
            // senão o filtro `--filtro ano=` selecionaria a tarefa errada.
            assert_eq!(
                tarefa.etiquetas.get("ano").map(String::as_str),
                Some(ano),
                "etiqueta `ano` diverge do nome em {}",
                tarefa.tarefa.nome()
            );
        }
    }

    #[test]
    fn a_url_aponta_para_o_csv_anual_da_ceaps() {
        // Trava o padrão da URL: um ano específico cai no arquivo certo do Senado.
        let tarefa = tarefas_ate(2024)
            .into_iter()
            .find(|t| t.etiquetas["ano"] == "2024")
            .expect("tarefa de 2024");
        assert_eq!(
            tarefa.tarefa.nome(),
            "despesas_2024",
            "o nome identifica o ano do arquivo"
        );
    }

    #[test]
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteSenadoDespesas.sob_demanda());
    }
}
