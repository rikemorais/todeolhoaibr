use std::time::Duration;

use todeolho_nucleo::{
    FonteDeDados, LimiteHttp, PerfilHttp, RedirecionamentoHttp, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "gfw";

const BASE: &str = "https://ourworldindata.org/grapher";

/// Desmatamento e incêndios globais — Global Forest Watch via Our World in Data.
///
/// Baixa CSVs consolidados (todos os países, 2001–2024) do mirror OWID:
///
/// - **Tree Cover Loss**: perda de cobertura florestal por país, ano e driver
///   (agricultura, mineração, incêndio, etc.).
/// - **Tree Cover Loss from Wildfires**: subconjunto da perda causada por
///   incêndios florestais.
///
/// Fonte original: Hansen et al. (2013) + Sims et al. (2025), processado pelo
/// Our World in Data. Dados do Global Forest Watch / World Resources Institute.
pub struct FonteGfw;

impl FonteDeDados for FonteGfw {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Desmatamento e incêndios globais — Global Forest Watch via OWID"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "ourworldindata.org",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        vec![
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                "tree_cover_loss".into(),
                NOME,
                "tree-cover-loss".into(),
                format!(
                    "{BASE}/tree-cover-loss.csv?v=1&csvType=full&useColumnShortNames=false"
                ),
            ))
            .com_etiquetas([("registro", "tree-cover-loss"), ("formato", "csv")]),
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                "tree_cover_loss_wildfires".into(),
                NOME,
                "tree-cover-loss-wildfires".into(),
                format!(
                    "{BASE}/tree-cover-loss-from-wildfires.csv?v=1&csvType=full&useColumnShortNames=false"
                ),
            ))
            .com_etiquetas([
                ("registro", "tree-cover-loss-wildfires"),
                ("formato", "csv"),
            ]),
        ]
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteGfw.criar_tarefas()
    }

    #[test]
    fn total_2_tarefas() {
        assert_eq!(tarefas().len(), 2);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn tree_cover_loss_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "tree_cover_loss")
        );
    }

    #[test]
    fn tree_cover_loss_wildfires_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "tree_cover_loss_wildfires")
        );
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            assert!(
                t.etiquetas.contains_key("registro"),
                "{} sem etiqueta registro",
                t.tarefa.nome()
            );
            assert!(
                t.etiquetas.contains_key("formato"),
                "{} sem etiqueta formato",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteGfw.sob_demanda());
    }

    #[test]
    fn declara_limite_do_host() {
        let limite = FonteGfw.perfil_http().limite.expect("limite do host OWID");
        assert_eq!(limite.host, "ourworldindata.org");
    }

    #[test]
    fn nao_precisa_credencial() {
        assert!(FonteGfw.perfil_http().credencial.is_none());
    }
}
