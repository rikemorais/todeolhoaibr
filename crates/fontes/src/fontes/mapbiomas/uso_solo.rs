use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "mapbiomas";

const BASE: &str = "https://brasil.mapbiomas.org/wp-content/uploads/sites/3";

const COLECAO: &str = "11";

struct Produto {
    slug: &'static str,
    recorte: &'static str,
    caminho: &'static str,
}

const PRODUTOS: [Produto; 3] = [
    Produto {
        slug: "cobertura",
        recorte: "bioma-estado",
        caminho: "2026/08/MAPBIOMAS_BRAZIL-COL.11-BIOME_STATE.xlsx",
    },
    Produto {
        slug: "desmatamento",
        recorte: "municipio",
        caminho: "2026/10/MAPBIOMAS-BRAZIL-DEFORESTATION-ANNUAL-MUNICIPALITY-COLLECTION_11.xlsx",
    },
    Produto {
        slug: "vegetacao-secundaria",
        recorte: "municipio",
        caminho: "2026/10/MAPBIOMAS-BRAZIL-SECONDARY-VEGETATION-MUNICIPALITY-COLLECTION_11.xlsx",
    },
];

/// MapBiomas — uso e cobertura do solo (Collection 11, 1985–2025).
///
/// Estatísticas tabuladas (XLSX) de cobertura por bioma/estado,
/// desmatamento anual e vegetação secundária por município.
/// Fonte de terceiros, referência nacional, dado aberto.
pub struct FonteMapBiomas;

impl FonteDeDados for FonteMapBiomas {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "MapBiomas — uso e cobertura do solo, desmatamento e vegetação secundária"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "brasil.mapbiomas.org",
                requisicoes: 1,
                intervalo: std::time::Duration::from_secs(2),
            }),
            agente: None,
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Bloquear,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        PRODUTOS
            .iter()
            .map(|p| {
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    format!("{}-{}", p.slug, p.recorte),
                    NOME,
                    format!("{}/colecao={COLECAO}/recorte={}", p.slug, p.recorte),
                    format!("{BASE}/{}", p.caminho),
                ))
                .com_etiquetas([
                    ("produto", p.slug),
                    ("colecao", COLECAO),
                    ("recorte", p.recorte),
                ])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteMapBiomas.criar_tarefas()
    }

    #[test]
    fn gera_3_tarefas() {
        assert_eq!(tarefas().len(), 3);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn tem_cobertura_bioma_estado() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "cobertura-bioma-estado")
        );
    }

    #[test]
    fn tem_desmatamento_municipio() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "desmatamento-municipio")
        );
    }

    #[test]
    fn tem_vegetacao_secundaria() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "vegetacao-secundaria-municipio")
        );
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteMapBiomas.sob_demanda());
    }

    #[test]
    fn nao_aceita_certificado_invalido() {
        assert!(!FonteMapBiomas.perfil_http().aceitar_certificado_invalido);
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            let etiq = &t.etiquetas;
            assert!(
                etiq.contains_key("produto"),
                "{} sem etiqueta produto",
                t.tarefa.nome()
            );
            assert!(
                etiq.contains_key("colecao"),
                "{} sem etiqueta colecao",
                t.tarefa.nome()
            );
            assert!(
                etiq.contains_key("recorte"),
                "{} sem etiqueta recorte",
                t.tarefa.nome()
            );
            assert_eq!(etiq["colecao"], "11");
        }
    }

    #[test]
    fn bloqueia_redirecionamento() {
        assert!(matches!(
            FonteMapBiomas.perfil_http().redirecionamento,
            todeolho_nucleo::RedirecionamentoHttp::Bloquear
        ));
    }
}
