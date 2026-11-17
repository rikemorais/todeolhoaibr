use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "inpe";

const BASE_WFS: &str = "https://terrabrasilis.dpi.inpe.br/geoserver";

const BASE_QUEIMADAS: &str =
    "https://dataserver-coids.inpe.br/queimadas/queimadas/focos/csv/anual/Brasil_sat_ref";

const ANO_INICIO_QUEIMADAS: u16 = 2003;
const ANO_FIM_QUEIMADAS: u16 = 2025;

struct BiomaProdes {
    slug: &'static str,
    workspace: &'static str,
    layer: &'static str,
}

const BIOMAS_PRODES: [BiomaProdes; 7] = [
    BiomaProdes {
        slug: "amazonia",
        workspace: "prodes-amazon-nb",
        layer: "yearly_deforestation_biome",
    },
    BiomaProdes {
        slug: "amazonia-legal",
        workspace: "prodes-legal-amz",
        layer: "yearly_deforestation",
    },
    BiomaProdes {
        slug: "cerrado",
        workspace: "prodes-cerrado-nb",
        layer: "yearly_deforestation",
    },
    BiomaProdes {
        slug: "caatinga",
        workspace: "prodes-caatinga-nb",
        layer: "yearly_deforestation",
    },
    BiomaProdes {
        slug: "mata-atlantica",
        workspace: "prodes-mata-atlantica-nb",
        layer: "yearly_deforestation",
    },
    BiomaProdes {
        slug: "pampa",
        workspace: "prodes-pampa-nb",
        layer: "yearly_deforestation",
    },
    BiomaProdes {
        slug: "pantanal",
        workspace: "prodes-pantanal-nb",
        layer: "yearly_deforestation",
    },
];

struct BiomaDeter {
    slug: &'static str,
    workspace: &'static str,
    layer: &'static str,
}

const BIOMAS_DETER: [BiomaDeter; 2] = [
    BiomaDeter {
        slug: "amazonia",
        workspace: "deter-amz",
        layer: "deter_amz",
    },
    BiomaDeter {
        slug: "cerrado",
        workspace: "deter-cerrado-nb",
        layer: "deter_cerrado",
    },
];

fn url_wfs(workspace: &str, layer: &str) -> String {
    format!(
        "{BASE_WFS}/{workspace}/wfs\
         ?service=WFS&version=2.0.0&request=GetFeature\
         &typeName={workspace}:{layer}\
         &outputFormat=SHAPE-ZIP&srsName=EPSG:4674"
    )
}

/// INPE — PRODES / DETER / Queimadas: desmatamento anual, alertas quase em
/// tempo real e focos de calor por satélite.
///
/// Publica shapefiles por bioma (PRODES e DETER) via WFS do GeoServer
/// TerraBrasilis, e CSVs anuais de focos de calor (satélite de referência)
/// do data server do Programa Queimadas. CRS EPSG:4674 (SIRGAS 2000).
///
/// Ressalva: **não comparar DETER pré-2015 com pós-2015** (mudança
/// metodológica). PRODES é a medida oficial anual.
pub struct FonteInpe;

impl FonteDeDados for FonteInpe {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "INPE — desmatamento (PRODES/DETER) e focos de calor (Queimadas)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "terrabrasilis.dpi.inpe.br",
                requisicoes: 1,
                intervalo: std::time::Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Bloquear,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas = Vec::with_capacity(32);

        for b in &BIOMAS_PRODES {
            tarefas.push(
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    format!("prodes-{}", b.slug),
                    NOME,
                    format!("prodes/bioma={}", b.slug),
                    url_wfs(b.workspace, b.layer),
                ))
                .com_etiquetas([("produto", "prodes"), ("bioma", b.slug)]),
            );
        }

        for b in &BIOMAS_DETER {
            tarefas.push(
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    format!("deter-{}", b.slug),
                    NOME,
                    format!("deter/bioma={}", b.slug),
                    url_wfs(b.workspace, b.layer),
                ))
                .com_etiquetas([("produto", "deter"), ("bioma", b.slug)]),
            );
        }

        for ano in ANO_INICIO_QUEIMADAS..=ANO_FIM_QUEIMADAS {
            let mut tarefa = TarefaDeArquivo::nova(
                format!("queimadas-{ano}"),
                NOME,
                format!("queimadas/ano={ano}"),
                format!("{BASE_QUEIMADAS}/focos_br_ref_{ano}.zip"),
            );
            if ano == ANO_FIM_QUEIMADAS {
                tarefa = tarefa.aceitar_ausencia();
            }
            tarefas.push(
                TarefaPlanejada::nova(tarefa)
                    .com_etiquetas([("produto", "queimadas"), ("ano", &ano.to_string())]),
            );
        }

        tarefas
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteInpe.criar_tarefas()
    }

    #[test]
    fn gera_32_tarefas() {
        assert_eq!(tarefas().len(), 32);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn tem_prodes_cerrado() {
        let t = tarefas();
        assert!(t.iter().any(|t| t.tarefa.nome() == "prodes-cerrado"));
    }

    #[test]
    fn tem_deter_amazonia() {
        let t = tarefas();
        assert!(t.iter().any(|t| t.tarefa.nome() == "deter-amazonia"));
    }

    #[test]
    fn tem_queimadas_2024() {
        let t = tarefas();
        assert!(t.iter().any(|t| t.tarefa.nome() == "queimadas-2024"));
    }

    #[test]
    fn nao_tem_queimadas_2002() {
        let t = tarefas();
        assert!(!t.iter().any(|t| t.tarefa.nome() == "queimadas-2002"));
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteInpe.sob_demanda());
    }

    #[test]
    fn nao_aceita_certificado_invalido() {
        assert!(!FonteInpe.perfil_http().aceitar_certificado_invalido);
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
            let produto = &etiq["produto"];
            match produto.as_str() {
                "prodes" | "deter" => {
                    assert!(
                        etiq.contains_key("bioma"),
                        "{} sem etiqueta bioma",
                        t.tarefa.nome()
                    );
                }
                "queimadas" => {
                    assert!(
                        etiq.contains_key("ano"),
                        "{} sem etiqueta ano",
                        t.tarefa.nome()
                    );
                }
                _ => panic!("produto inesperado: {produto}"),
            }
        }
    }

    #[test]
    fn contagem_por_produto() {
        let t = tarefas();
        let prodes = t
            .iter()
            .filter(|t| t.etiquetas.get("produto").map(|v| v.as_str()) == Some("prodes"))
            .count();
        let deter = t
            .iter()
            .filter(|t| t.etiquetas.get("produto").map(|v| v.as_str()) == Some("deter"))
            .count();
        let queimadas = t
            .iter()
            .filter(|t| t.etiquetas.get("produto").map(|v| v.as_str()) == Some("queimadas"))
            .count();
        assert_eq!(prodes, 7);
        assert_eq!(deter, 2);
        assert_eq!(queimadas, 23);
    }
}
