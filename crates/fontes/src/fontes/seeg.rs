use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "seeg";

struct Arquivo {
    slug: &'static str,
    url: &'static str,
}

const ARQUIVOS: [Arquivo; 2] = [
    Arquivo {
        slug: "dados-nacionais",
        url: "https://seeg.obass.haus/wp-content/uploads/2026/11/Dados-nacionais-14.0.xlsx",
    },
    Arquivo {
        slug: "dados-municipais",
        url: "https://seeg.obass.haus/wp-content/uploads/2026/11/Dados-municipais-resumido-CO2e-GWP-AR5-14.0.xlsx",
    },
];

/// SEEG — emissões de gases de efeito estufa do Observatório do Clima.
///
/// Dois arquivos XLSX da edição 14.0: dados nacionais (BR + UF) e dados
/// municipais resumidos em CO2 equivalente (GWP-AR5). Cobertura 1990–2025,
/// cinco setores (Agropecuária, Energia, Mudança de Uso da Terra, Processos
/// Industriais, Resíduos).
pub struct FonteSeeg;

impl FonteDeDados for FonteSeeg {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "SEEG — emissões de gases de efeito estufa (Observatório do Clima)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "seeg.obass.haus",
                requisicoes: 1,
                intervalo: Duration::from_secs(2),
            }),
            agente: None,
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Bloquear,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        ARQUIVOS
            .iter()
            .map(|a| {
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    a.slug.to_owned(),
                    NOME,
                    a.slug.to_owned(),
                    a.url.to_owned(),
                ))
                .com_etiquetas([("produto", "emissoes"), ("edicao", "14.0")])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteSeeg.criar_tarefas()
    }

    #[test]
    fn gera_2_tarefas() {
        assert_eq!(tarefas().len(), 2);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn tem_dados_nacionais() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "dados-nacionais")
        );
    }

    #[test]
    fn tem_dados_municipais() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "dados-municipais")
        );
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteSeeg.sob_demanda());
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
                etiq.contains_key("edicao"),
                "{} sem etiqueta edicao",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn declara_limite_do_host() {
        let limite = FonteSeeg.perfil_http().limite.expect("limite do host");
        assert_eq!(limite.host, "seeg.obass.haus");
    }

    #[test]
    fn bloqueia_redirects() {
        assert!(matches!(
            FonteSeeg.perfil_http().redirecionamento,
            todeolho_nucleo::RedirecionamentoHttp::Bloquear
        ));
    }
}
