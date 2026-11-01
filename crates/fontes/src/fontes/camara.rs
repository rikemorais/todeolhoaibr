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
    use super::*;

    fn tarefas_de_despesas() -> Vec<String> {
        FonteCamara
            .criar_tarefas()
            .iter()
            .filter(|t| t.tarefa.nome().starts_with("despesas_"))
            .map(|t| t.tarefa.nome().to_owned())
            .collect()
    }

    #[test]
    fn cobre_um_arquivo_de_ceap_por_ano() {
        let esperado = usize::from(ULTIMO_ANO_CEAP - PRIMEIRO_ANO_CEAP + 1);
        assert_eq!(tarefas_de_despesas().len(), esperado);
        assert!(tarefas_de_despesas().contains(&format!("despesas_{PRIMEIRO_ANO_CEAP}")));
        assert!(tarefas_de_despesas().contains(&format!("despesas_{ULTIMO_ANO_CEAP}")));
    }

    #[test]
    fn as_despesas_sao_filtraveis_por_ano() {
        for tarefa in FonteCamara.criar_tarefas() {
            if tarefa.tarefa.nome().starts_with("despesas_") {
                assert!(
                    tarefa.etiquetas.contains_key("ano"),
                    "{}",
                    tarefa.tarefa.nome()
                );
            }
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
