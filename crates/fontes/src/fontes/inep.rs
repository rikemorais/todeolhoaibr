use std::time::Duration;

use todeolho_nucleo::{
    FonteDeDados, LimiteHttp, PerfilHttp, RedirecionamentoHttp, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "inep";

const BASE: &str = "https://download.inep.gov.br";

const PRIMEIRO_ANO_CENSO: u16 = 1995;
const ULTIMO_ANO_CENSO: u16 = 2025;

const PRIMEIRO_ANO_ENEM: u16 = 1998;
const ULTIMO_ANO_ENEM: u16 = 2025;

const PRIMEIRO_ANO_REDACAO: u16 = 2019;
const ULTIMO_ANO_REDACAO: u16 = 2024;

const EDICOES_IDEB: [u16; 5] = [2017, 2019, 2021, 2023, 2025];

const STAGES_IDEB: [&str; 3] = ["anos_iniciais", "anos_finais", "ensino_medio"];

/// Indicadores de educação do INEP — Censo Escolar, ENEM e IDEB.
///
/// - **Censo Escolar** (1995–2025): microdados ZIP com CSV de escolas,
///   matrículas, turmas e docentes.
/// - **ENEM** (1998–2025): microdados ZIP; complemento de competências
///   da redação (2019–2024).
/// - **IDEB** (bienal, 2017–2025): resultados por escola, município,
///   UF e Brasil em XLSX/ZIP.
pub struct FonteInep;

impl FonteDeDados for FonteInep {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Indicadores de educação do INEP — Censo Escolar, ENEM e IDEB"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "download.inep.gov.br",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas = Vec::with_capacity(110);
        tarefas.extend(tarefas_censo_escolar());
        tarefas.extend(tarefas_enem());
        tarefas.extend(tarefas_enem_redacao());
        tarefas.extend(tarefas_ideb());
        tarefas
    }
}

fn tarefas_censo_escolar() -> Vec<TarefaPlanejada> {
    (PRIMEIRO_ANO_CENSO..=ULTIMO_ANO_CENSO)
        .map(|ano| {
            let sufixo = if ano == 2025 {
                format!("microdados_censo_escolar_{ano}_.zip")
            } else {
                format!("microdados_censo_escolar_{ano}.zip")
            };
            let url = format!("{BASE}/dados_abertos/{sufixo}");
            let ano_s = ano.to_string();
            let mut tarefa = TarefaDeArquivo::nova(
                format!("censo_escolar_{ano}"),
                NOME,
                format!("censo-escolar/ano={ano}"),
                url,
            );
            if ano == ULTIMO_ANO_CENSO {
                tarefa = tarefa.aceitar_ausencia();
            }
            TarefaPlanejada::nova(tarefa)
                .com_etiquetas([("registro", "censo-escolar"), ("ano", &ano_s)])
        })
        .collect()
}

fn tarefas_enem() -> Vec<TarefaPlanejada> {
    (PRIMEIRO_ANO_ENEM..=ULTIMO_ANO_ENEM)
        .map(|ano| {
            let url = format!("{BASE}/microdados/microdados_enem_{ano}.zip");
            let ano_s = ano.to_string();
            let mut tarefa =
                TarefaDeArquivo::nova(format!("enem_{ano}"), NOME, format!("enem/ano={ano}"), url);
            if ano == ULTIMO_ANO_ENEM {
                tarefa = tarefa.aceitar_ausencia();
            }
            TarefaPlanejada::nova(tarefa).com_etiquetas([("registro", "enem"), ("ano", &ano_s)])
        })
        .collect()
}

fn tarefas_enem_redacao() -> Vec<TarefaPlanejada> {
    (PRIMEIRO_ANO_REDACAO..=ULTIMO_ANO_REDACAO)
        .map(|ano| {
            let url = format!("{BASE}/microdados/competencias_redacao_enem_{ano}.zip");
            let ano_s = ano.to_string();
            let mut tarefa = TarefaDeArquivo::nova(
                format!("enem_redacao_{ano}"),
                NOME,
                format!("enem-redacao/ano={ano}"),
                url,
            );
            if ano == ULTIMO_ANO_REDACAO {
                tarefa = tarefa.aceitar_ausencia();
            }
            TarefaPlanejada::nova(tarefa)
                .com_etiquetas([("registro", "enem-redacao"), ("ano", &ano_s)])
        })
        .collect()
}

fn ext_ideb(ano: u16) -> &'static str {
    if ano >= 2025 { ".zip" } else { ".xlsx" }
}

fn tarefas_ideb() -> Vec<TarefaPlanejada> {
    let mut tarefas = Vec::with_capacity(42);

    for &ano in &EDICOES_IDEB {
        let ano_s = ano.to_string();
        let ext = ext_ideb(ano);

        for nivel in ["escolas", "municipios"] {
            let nivel_slug = if nivel == "escolas" {
                "escola"
            } else {
                "municipio"
            };
            for &stage in &STAGES_IDEB {
                let url = format!("{BASE}/ideb/resultados/divulgacao_{stage}_{nivel}_{ano}{ext}");
                let mut tarefa = TarefaDeArquivo::nova(
                    format!("ideb_{nivel_slug}_{stage}_{ano}"),
                    NOME,
                    format!("ideb/{nivel_slug}/{stage}/edicao={ano}"),
                    url,
                );
                if ano == *EDICOES_IDEB.last().unwrap() {
                    tarefa = tarefa.aceitar_ausencia();
                }
                tarefas.push(TarefaPlanejada::nova(tarefa).com_etiquetas([
                    ("registro", "ideb"),
                    ("nivel", nivel_slug),
                    ("stage", stage),
                    ("ano", &ano_s),
                ]));
            }
        }

        let url_brasil = format!("{BASE}/ideb/resultados/divulgacao_brasil_ideb_{ano}{ext}");
        let mut tarefa_br = TarefaDeArquivo::nova(
            format!("ideb_brasil_{ano}"),
            NOME,
            format!("ideb/brasil/edicao={ano}"),
            url_brasil,
        );
        if ano == *EDICOES_IDEB.last().unwrap() {
            tarefa_br = tarefa_br.aceitar_ausencia();
        }
        tarefas.push(TarefaPlanejada::nova(tarefa_br).com_etiquetas([
            ("registro", "ideb"),
            ("nivel", "brasil"),
            ("ano", &ano_s),
        ]));

        let url_regioes = format!("{BASE}/ideb/resultados/divulgacao_regioes_ufs_ideb_{ano}{ext}");
        let mut tarefa_reg = TarefaDeArquivo::nova(
            format!("ideb_regioes_ufs_{ano}"),
            NOME,
            format!("ideb/regioes-ufs/edicao={ano}"),
            url_regioes,
        );
        if ano == *EDICOES_IDEB.last().unwrap() {
            tarefa_reg = tarefa_reg.aceitar_ausencia();
        }
        tarefas.push(TarefaPlanejada::nova(tarefa_reg).com_etiquetas([
            ("registro", "ideb"),
            ("nivel", "regioes-ufs"),
            ("ano", &ano_s),
        ]));
    }

    for nivel in ["brasil", "ufs"] {
        let url = format!(
            "{BASE}/ideb/resultados/divulgacao_{nivel}_Ensino_Medio_integrado_ideb_2025.xlsx"
        );
        let nivel_slug = if nivel == "ufs" {
            "regioes-ufs"
        } else {
            "brasil"
        };
        tarefas.push(
            TarefaPlanejada::nova(
                TarefaDeArquivo::nova(
                    format!("ideb_emi_{nivel}_2025"),
                    NOME,
                    format!("ideb/{nivel_slug}/ensino_medio_integrado/edicao=2025"),
                    url,
                )
                .aceitar_ausencia(),
            )
            .com_etiquetas([
                ("registro", "ideb"),
                ("nivel", nivel_slug),
                ("stage", "ensino_medio_integrado"),
                ("ano", "2025"),
            ]),
        );
    }

    tarefas
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteInep.criar_tarefas()
    }

    fn contagem(registro: &str) -> usize {
        tarefas()
            .iter()
            .filter(|t| t.etiquetas.get("registro").map(String::as_str) == Some(registro))
            .count()
    }

    #[test]
    fn contagem_fixa_por_registro() {
        assert_eq!(contagem("censo-escolar"), 31);
        assert_eq!(contagem("enem"), 28);
        assert_eq!(contagem("enem-redacao"), 6);
        assert_eq!(contagem("ideb"), 42);
    }

    #[test]
    fn total_107_tarefas() {
        assert_eq!(tarefas().len(), 107);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn censo_2024_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "censo_escolar_2024")
        );
    }

    #[test]
    fn censo_2025_url_tem_underscore_extra() {
        let url = format!("{BASE}/dados_abertos/microdados_censo_escolar_2025_.zip");
        assert!(url.contains("_2025_.zip"));
    }

    #[test]
    fn enem_2023_existe() {
        assert!(tarefas().iter().any(|t| t.tarefa.nome() == "enem_2023"));
    }

    #[test]
    fn enem_redacao_2022_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "enem_redacao_2022")
        );
    }

    #[test]
    fn ideb_escola_anos_iniciais_2023_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "ideb_escola_anos_iniciais_2023")
        );
    }

    #[test]
    fn ideb_2025_usa_zip() {
        assert_eq!(ext_ideb(2025), ".zip");
        assert_eq!(ext_ideb(2023), ".xlsx");
    }

    #[test]
    fn ideb_emi_brasil_2025_existe() {
        assert!(
            tarefas()
                .iter()
                .any(|t| t.tarefa.nome() == "ideb_emi_brasil_2025")
        );
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            let etiq = &t.etiquetas;
            assert!(
                etiq.contains_key("registro"),
                "{} sem etiqueta registro",
                t.tarefa.nome()
            );
            assert!(
                etiq.contains_key("ano"),
                "{} sem etiqueta ano",
                t.tarefa.nome()
            );
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteInep.sob_demanda());
    }

    #[test]
    fn declara_limite_do_host() {
        let limite = FonteInep.perfil_http().limite.expect("limite do host INEP");
        assert_eq!(limite.host, "download.inep.gov.br");
    }
}
