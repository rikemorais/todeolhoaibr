use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;
use crate::ingestao_http::TarefaDeIngestaoHttp;
use crate::paginacao::LinksDePaginacao;

const NOME: &str = "camara";
const BASE: &str = "https://dadosabertos.camara.leg.br/api/v2";

/// Arquivos anuais da CEAP (Cota para o Exercício da Atividade Parlamentar).
const BASE_CEAP: &str = "https://www.camara.leg.br/cotas";
/// Primeiro ano com arquivo anual da CEAP publicado.
const PRIMEIRO_ANO_CEAP: u16 = 2008;
/// Último ano disponível. Atualizar a cada ano.
const ULTIMO_ANO_CEAP: u16 = 2026;

/// Dados Abertos da Câmara dos Deputados: <https://dadosabertos.camara.leg.br/swagger/api.html>
pub struct FonteCamara;

impl FonteDeDados for FonteCamara {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Câmara dos Deputados - legislaturas, partidos, deputados em exercício e despesas (CEAP)"
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let api = |recurso: &str, tamanho_tipico: u64| {
            TarefaDeIngestaoHttp::paginada(
                NOME,
                recurso,
                format!("{BASE}/{recurso}?itens=100"),
                LinksDePaginacao,
            )
            .com_tamanho_tipico(tamanho_tipico)
        };

        let mut tarefas = vec![
            TarefaPlanejada::nova(api("legislaturas", 8_000)),
            TarefaPlanejada::nova(api("partidos", 3_000)),
            TarefaPlanejada::nova(api("deputados", 190_000))
                .depois_de(["legislaturas", "partidos"]),
        ];

        // CEAP: um arquivo CSV (zipado) por ano, nacional, baixado como arquivo grande.
        // As despesas não têm identificador único e a Raw guarda o arquivo como vem
        // ([ADR 0014]); a deduplicação por chave composta fica para o refino ([ADR 0015]).
        // Ver issue #25. A UF é uma coluna do CSV, então o filtro aqui é só por ano.
        tarefas.extend((PRIMEIRO_ANO_CEAP..=ULTIMO_ANO_CEAP).map(|ano| {
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                format!("despesas_{ano}"),
                NOME,
                format!("despesas/ano={ano}"),
                format!("{BASE_CEAP}/Ano-{ano}.csv.zip"),
            ))
            .com_etiquetas([("ano", ano.to_string().as_str())])
        }));

        tarefas
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    /// Anos cobertos pelas tarefas de despesas, lidos do nome `despesas_<ano>`.
    fn anos_de_despesas() -> BTreeSet<u16> {
        FonteCamara
            .criar_tarefas()
            .iter()
            .filter_map(|t| {
                t.tarefa
                    .nome()
                    .strip_prefix("despesas_")
                    .map(|ano| ano.parse().expect("ano numérico no nome da tarefa"))
            })
            .collect()
    }

    #[test]
    fn cobre_exatamente_um_arquivo_por_ano_de_ceap() {
        // Conjunto exato (não só a contagem e os extremos): pega ano faltando no meio.
        let esperado: BTreeSet<u16> = (PRIMEIRO_ANO_CEAP..=ULTIMO_ANO_CEAP).collect();
        assert_eq!(anos_de_despesas(), esperado);
    }

    #[test]
    fn a_etiqueta_ano_bate_com_o_ano_do_nome() {
        for tarefa in FonteCamara.criar_tarefas() {
            let Some(ano) = tarefa.tarefa.nome().strip_prefix("despesas_") else {
                continue;
            };
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
    fn mantem_os_recursos_de_api() {
        let nomes: Vec<String> = FonteCamara
            .criar_tarefas()
            .iter()
            .map(|t| t.tarefa.nome().to_owned())
            .collect();
        for recurso in ["legislaturas", "partidos", "deputados"] {
            assert!(nomes.iter().any(|n| n == recurso), "faltou {recurso}");
        }
    }
}
