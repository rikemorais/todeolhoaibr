use chrono::{Datelike, Utc};
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "tse-candidaturas";

/// A Raw guarda os dados sob a fonte `tse` (`raw/tse/...`), embora no catálogo sejam
/// uma fonte separada, para poderem ser `sob_demanda`.
const FONTE_RAW: &str = "tse";
const BASE: &str = "https://cdn.tse.jus.br/estatistica/sead/odsele";

/// Primeira eleição com o registro de candidaturas publicado em dados abertos.
const PRIMEIRA_ELEICAO_CANDIDATURAS: u16 = 1994;
/// Primeira eleição com a declaração de bens dos candidatos publicada.
const PRIMEIRA_ELEICAO_BENS: u16 = 2006;

/// Última eleição com arquivo publicado: o maior ano par até hoje (eleições são de dois
/// em dois anos), então um ano ímpar cai na eleição anterior.
fn ultima_eleicao() -> u16 {
    let ano = Utc::now().year() as u16;
    if ano.is_multiple_of(2) { ano } else { ano - 1 }
}

/// Monta a tarefa de um ano de um conjunto. `tipo` vira etiqueta; `pasta`/`prefixo` montam
/// a URL (`{BASE}/{pasta}/{prefixo}_{ano}.zip`).
fn tarefa(tipo: &str, pasta: &str, prefixo: &str, ano: u16, ultima: u16) -> TarefaPlanejada {
    let mut arquivo = TarefaDeArquivo::nova(
        format!("{tipo}_{ano}"),
        FONTE_RAW,
        format!("{tipo}/ano={ano}"),
        format!("{BASE}/{pasta}/{prefixo}_{ano}.zip"),
    );
    if ano == ultima {
        // A eleição corrente pode ainda não ter o arquivo publicado no começo do ano.
        arquivo = arquivo.aceitar_ausencia();
    }
    TarefaPlanejada::nova(arquivo)
        .com_etiquetas([("ano", ano.to_string().as_str()), ("tipo", tipo)])
}

fn tarefas_ate(ultima: u16) -> Vec<TarefaPlanejada> {
    let candidaturas = (PRIMEIRA_ELEICAO_CANDIDATURAS..=ultima)
        .step_by(2)
        .map(|ano| {
            tarefa(
                "candidaturas",
                "consulta_cand",
                "consulta_cand",
                ano,
                ultima,
            )
        });
    let bens = (PRIMEIRA_ELEICAO_BENS..=ultima)
        .step_by(2)
        .map(|ano| tarefa("bens", "bem_candidato", "bem_candidato", ano, ultima));
    candidaturas.chain(bens).collect()
}

/// Candidaturas e bens declarados dos candidatos (TSE).
///
/// Dois conjuntos por eleição (de dois em dois anos), nacionais, baixados como arquivos
/// grandes e guardados opacos na Raw ([ADR 0014]): o **registro de candidaturas** (cargo,
/// partido, UF, município — desde 1994) e os **bens declarados** de cada candidato (tipo e
/// valor — desde 2006). O filtro é por `ano` da eleição e `tipo`; o recorte por UF e a
/// evolução patrimonial vêm das colunas dos CSVs, então ficam para o refino ([ADR 0015]).
/// Fonte separada da `tse` (votação por seção) para rodar sozinha. Ver issue #58.
///
/// Dados abertos: <https://dadosabertos.tse.jus.br/dataset/?groups=candidatos>
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteTseCandidaturas;

impl FonteDeDados for FonteTseCandidaturas {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "TSE - candidaturas (desde 1994) e bens declarados dos candidatos (desde 2006), por eleição"
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

    fn anos_do_tipo(ultima: u16, tipo: &str) -> BTreeSet<u16> {
        tarefas_ate(ultima)
            .iter()
            .filter(|t| t.etiquetas.get("tipo").map(String::as_str) == Some(tipo))
            .map(|t| t.etiquetas["ano"].parse().expect("ano numérico"))
            .collect()
    }

    #[test]
    fn candidaturas_cobrem_de_1994_ate_a_ultima_eleicao() {
        assert_eq!(
            anos_do_tipo(2026, "candidaturas"),
            (PRIMEIRA_ELEICAO_CANDIDATURAS..=2026).step_by(2).collect()
        );
    }

    #[test]
    fn bens_cobrem_de_2006_ate_a_ultima_eleicao() {
        assert_eq!(
            anos_do_tipo(2026, "bens"),
            (PRIMEIRA_ELEICAO_BENS..=2026).step_by(2).collect()
        );
    }

    #[test]
    fn nao_inclui_anos_sem_eleicao() {
        let anos = anos_do_tipo(2026, "candidaturas");
        assert!(anos.contains(&2022));
        assert!(!anos.contains(&2023), "2023 não é ano de eleição");
    }

    #[test]
    fn ano_impar_cai_na_eleicao_par_anterior() {
        assert_eq!(
            *anos_do_tipo(2027, "candidaturas").iter().max().unwrap(),
            2026
        );
    }

    #[test]
    fn cada_tarefa_tem_ano_e_tipo_coerentes_com_o_nome() {
        for tarefa in FonteTseCandidaturas.criar_tarefas() {
            let nome = tarefa.tarefa.nome();
            let ano = tarefa.etiquetas.get("ano").expect("etiqueta ano");
            let tipo = tarefa.etiquetas.get("tipo").expect("etiqueta tipo");
            assert_eq!(
                nome,
                format!("{tipo}_{ano}"),
                "nome incoerente com ano/tipo"
            );
        }
    }

    #[test]
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteTseCandidaturas.sob_demanda());
    }
}
