use todeolho_nucleo::TarefaPlanejada;

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "cnpj";
const BASE: &str = "https://arquivos.receitafederal.gov.br/public.php/dav/files/YggdBLfdninEJX9";

/// Competências (meses) publicadas pela Receita Federal.
/// Acrescente aqui cada novo snapshot mensal quando disponibilizado.
const COMPETENCIAS: [&str; 1] = ["2026-10"];

/// Tabelas divididas em 10 partes (0–9).
const TABELAS_DIVIDIDAS: [(&str, u8); 3] =
    [("Empresas", 10), ("Estabelecimentos", 10), ("Socios", 10)];

/// Tabelas publicadas em arquivo único.
const TABELAS_UNICAS: [&str; 7] = [
    "Simples",
    "Cnaes",
    "Municipios",
    "Naturezas",
    "Motivos",
    "Qualificacoes",
    "Paises",
];

fn slug(nome_arquivo: &str) -> String {
    nome_arquivo.to_ascii_lowercase()
}

pub(crate) fn criar_tarefas() -> Vec<TarefaPlanejada> {
    let mut tarefas = Vec::new();

    for competencia in COMPETENCIAS {
        for &(tabela, partes) in &TABELAS_DIVIDIDAS {
            let tabela_slug = slug(tabela);
            for parte in 0..partes {
                tarefas.push(
                    TarefaPlanejada::nova(TarefaDeArquivo::nova(
                        format!("{competencia}/{tabela_slug}-{parte}"),
                        NOME,
                        format!("{competencia}/{tabela_slug}/{parte}"),
                        format!("{BASE}/{competencia}/{tabela}{parte}.zip"),
                    ))
                    .com_etiquetas([
                        ("competencia", competencia),
                        ("tabela", &tabela_slug),
                        ("parte", &parte.to_string()),
                    ]),
                );
            }
        }

        for tabela in TABELAS_UNICAS {
            let tabela_slug = slug(tabela);
            tarefas.push(
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    format!("{competencia}/{tabela_slug}"),
                    NOME,
                    format!("{competencia}/{tabela_slug}"),
                    format!("{BASE}/{competencia}/{tabela}.zip"),
                ))
                .com_etiquetas([("competencia", competencia), ("tabela", &tabela_slug)]),
            );
        }
    }

    tarefas
}

#[cfg(test)]
mod testes {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn cada_competencia_gera_37_tarefas() {
        let tarefas = criar_tarefas();
        assert_eq!(tarefas.len(), COMPETENCIAS.len() * 37);
    }

    #[test]
    fn nomes_sao_unicos() {
        let tarefas = criar_tarefas();
        let nomes: HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), tarefas.len());
    }

    #[test]
    fn recursos_sao_unicos() {
        let tarefas = criar_tarefas();
        let recursos: HashSet<String> = tarefas
            .iter()
            .map(|t| format!("{}/{}", t.tarefa.nome(), "recurso"))
            .collect();
        assert_eq!(recursos.len(), tarefas.len());
    }

    #[test]
    fn urls_seguem_o_padrao() {
        let tarefas = criar_tarefas();
        let primeira = &tarefas[0];
        assert!(
            primeira.tarefa.nome().starts_with("2026-10/"),
            "nome: {}",
            primeira.tarefa.nome()
        );

        let empresas: Vec<_> = tarefas
            .iter()
            .filter(|t| t.etiquetas.get("tabela").is_some_and(|v| v == "empresas"))
            .collect();
        assert_eq!(empresas.len(), 10);

        let simples: Vec<_> = tarefas
            .iter()
            .filter(|t| t.etiquetas.get("tabela").is_some_and(|v| v == "simples"))
            .collect();
        assert_eq!(simples.len(), 1);
    }

    #[test]
    fn etiquetas_presentes() {
        let tarefas = criar_tarefas();
        for tarefa in &tarefas {
            assert!(
                tarefa.etiquetas.contains_key("competencia"),
                "{}",
                tarefa.tarefa.nome()
            );
            assert!(
                tarefa.etiquetas.contains_key("tabela"),
                "{}",
                tarefa.tarefa.nome()
            );
        }
    }
}
