use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "camara-despesas";

/// A Raw guarda as despesas sob a mesma fonte `camara` (`raw/camara/despesas/...`),
/// embora no catálogo elas sejam uma fonte separada, para poderem ser `sob_demanda`.
const FONTE_RAW: &str = "camara";
/// Arquivos anuais da CEAP (Cota para o Exercício da Atividade Parlamentar).
const BASE_CEAP: &str = "https://www.camara.leg.br/cotas";
/// Primeiro ano com arquivo anual da CEAP publicado.
const PRIMEIRO_ANO_CEAP: u16 = 2008;
/// Último ano disponível. Atualizar a cada ano.
const ULTIMO_ANO_CEAP: u16 = 2026;

/// Despesas da CEAP (Cota para o Exercício da Atividade Parlamentar) dos deputados.
///
/// Um arquivo CSV (zipado) por ano, nacional, baixado como arquivo grande. O arquivo é
/// guardado opaco na Raw ([ADR 0014]); a UF é coluna do CSV, então o filtro aqui é só por
/// ano, e a deduplicação por chave composta (a CEAP não tem ID único) fica para o refino
/// ([ADR 0015]). Fonte separada da `camara` para ser `sob_demanda` e não entrar em `--todas`.
/// Ver issue #25.
///
/// Documentação dos campos: <https://dadosabertos.camara.leg.br/howtouse/2023-12-26-dados-ceap.html>
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteCamaraDespesas;

impl FonteDeDados for FonteCamaraDespesas {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Câmara dos Deputados - despesas da CEAP, um arquivo por ano desde 2008"
    }

    /// São ~19 arquivos anuais em lote: não devem entrar em `--todas` por acidente.
    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        (PRIMEIRO_ANO_CEAP..=ULTIMO_ANO_CEAP)
            .map(|ano| {
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    format!("despesas_{ano}"),
                    FONTE_RAW,
                    format!("despesas/ano={ano}"),
                    format!("{BASE_CEAP}/Ano-{ano}.csv.zip"),
                ))
                .com_etiquetas([("ano", ano.to_string().as_str())])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    /// Anos cobertos pelas tarefas, lidos do nome `despesas_<ano>`.
    fn anos() -> BTreeSet<u16> {
        FonteCamaraDespesas
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
        let esperado: BTreeSet<u16> = (PRIMEIRO_ANO_CEAP..=ULTIMO_ANO_CEAP).collect();
        assert_eq!(anos(), esperado);
    }

    #[test]
    fn a_etiqueta_ano_bate_com_o_ano_do_nome() {
        for tarefa in FonteCamaraDespesas.criar_tarefas() {
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
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteCamaraDespesas.sob_demanda());
    }
}
