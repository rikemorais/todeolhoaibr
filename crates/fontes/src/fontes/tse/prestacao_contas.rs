use chrono::{Datelike, Utc};
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "tse-prestacao-contas";

/// A Raw guarda a prestação de contas sob a fonte `tse` (`raw/tse/prestacao-contas/...`),
/// embora no catálogo ela seja uma fonte separada, para poder ser `sob_demanda`.
const FONTE_RAW: &str = "tse";
/// Pasta dos arquivos de prestação de contas no repositório de dados do TSE.
const BASE: &str = "https://cdn.tse.jus.br/estatistica/sead/odsele/prestacao_contas";
/// Primeira eleição com prestação de contas publicada em dados abertos.
const PRIMEIRA_ELEICAO: u16 = 2002;

/// Última eleição com arquivo publicado: o maior ano par até hoje. Eleições são de dois em
/// dois anos, então um ano ímpar cai na eleição anterior.
fn ultima_eleicao() -> u16 {
    let ano = Utc::now().year() as u16;
    if ano.is_multiple_of(2) { ano } else { ano - 1 }
}

fn eleicoes_ate(ultima: u16) -> impl Iterator<Item = u16> {
    (PRIMEIRA_ELEICAO..=ultima).step_by(2)
}

/// Nome do arquivo consolidado de prestação de contas de cada eleição.
///
/// O TSE mudou a convenção de nome ao longo dos anos — até 2010 um ZIP único, 2012 e 2016
/// com o sufixo `_final`, 2014 como `prestacao_final`, e de 2018 em diante o arquivo
/// específico de candidatos. Eleições futuras seguem a convenção de 2018.
fn nome_do_arquivo(ano: u16) -> String {
    match ano {
        ..=2010 => format!("prestacao_contas_{ano}.zip"),
        2012 | 2016 => format!("prestacao_contas_final_{ano}.zip"),
        2014 => format!("prestacao_final_{ano}.zip"),
        _ => format!("prestacao_de_contas_eleitorais_candidatos_{ano}.zip"),
    }
}

fn tarefas_ate(ultima: u16) -> Vec<TarefaPlanejada> {
    eleicoes_ate(ultima)
        .map(|ano| {
            let mut tarefa = TarefaDeArquivo::nova(
                format!("contas_{ano}"),
                FONTE_RAW,
                format!("prestacao-contas/ano={ano}"),
                format!("{BASE}/{}", nome_do_arquivo(ano)),
            );
            if ano == ultima {
                // A eleição corrente pode ainda não ter o arquivo publicado no começo do ano.
                tarefa = tarefa.aceitar_ausencia();
            }
            TarefaPlanejada::nova(tarefa).com_etiquetas([("ano", ano.to_string().as_str())])
        })
        .collect()
}

/// Prestação de contas eleitorais (TSE): receitas e despesas de **campanha** dos candidatos.
///
/// Universo diferente da cota de mandato (CEAP/CEAPS): aqui está o dinheiro de campanha —
/// quem doou, para quem, e no que o candidato gastou. Um arquivo ZIP por eleição (de dois
/// em dois anos, desde 2002), nacional, baixado como arquivo grande e guardado opaco na Raw
/// ([ADR 0014]). O filtro é por `ano` da eleição; o recorte por UF e a separação
/// receitas/despesas vêm das colunas dos CSVs, então ficam para o refino ([ADR 0015]).
/// Fonte separada da `tse` (votação por seção) para que cada uma rode sozinha. Ver issue #28.
///
/// Dados abertos: <https://dadosabertos.tse.jus.br/dataset/?groups=prestacao-de-contas>
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteTsePrestacaoContas;

impl FonteDeDados for FonteTsePrestacaoContas {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "TSE - prestação de contas de campanha (receitas e despesas), um arquivo por eleição desde 2002"
    }

    /// Arquivos grandes por eleição: não devem entrar em `--todas` por acidente.
    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        tarefas_ate(ultima_eleicao())
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    /// Anos cobertos pelas tarefas, lidos da etiqueta `ano`.
    fn anos_ate(ultima: u16) -> BTreeSet<u16> {
        tarefas_ate(ultima)
            .iter()
            .map(|t| t.etiquetas["ano"].parse().expect("ano numérico"))
            .collect()
    }

    #[test]
    fn cobre_exatamente_cada_eleicao_de_dois_em_dois_anos() {
        let esperado: BTreeSet<u16> = (PRIMEIRA_ELEICAO..=2026).step_by(2).collect();
        assert_eq!(anos_ate(2026), esperado);
    }

    #[test]
    fn nao_inclui_anos_sem_eleicao() {
        let anos = anos_ate(2026);
        assert!(anos.contains(&2022));
        assert!(!anos.contains(&2023), "2023 não é ano de eleição");
    }

    #[test]
    fn a_ultima_eleicao_cai_no_ano_par_anterior_quando_o_ano_e_impar() {
        // Rodar em 2027 (ímpar) ainda aponta para a eleição de 2026.
        assert_eq!(*anos_ate(2027).iter().max().unwrap(), 2026);
    }

    #[test]
    fn o_nome_do_arquivo_segue_a_convencao_de_cada_era() {
        assert_eq!(nome_do_arquivo(2002), "prestacao_contas_2002.zip");
        assert_eq!(nome_do_arquivo(2010), "prestacao_contas_2010.zip");
        assert_eq!(nome_do_arquivo(2012), "prestacao_contas_final_2012.zip");
        assert_eq!(nome_do_arquivo(2014), "prestacao_final_2014.zip");
        assert_eq!(nome_do_arquivo(2016), "prestacao_contas_final_2016.zip");
        assert_eq!(
            nome_do_arquivo(2018),
            "prestacao_de_contas_eleitorais_candidatos_2018.zip"
        );
        // Eleições futuras seguem a convenção de 2018 em diante.
        assert_eq!(
            nome_do_arquivo(2028),
            "prestacao_de_contas_eleitorais_candidatos_2028.zip"
        );
    }

    #[test]
    fn a_etiqueta_ano_bate_com_o_nome_da_tarefa() {
        for tarefa in FonteTsePrestacaoContas.criar_tarefas() {
            let ano = tarefa
                .tarefa
                .nome()
                .strip_prefix("contas_")
                .expect("nome no formato contas_<ano>");
            assert_eq!(
                tarefa.etiquetas.get("ano").map(String::as_str),
                Some(ano),
                "etiqueta `ano` diverge do nome em {}",
                tarefa.tarefa.nome()
            );
        }
    }

    #[test]
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteTsePrestacaoContas.sob_demanda());
    }
}
