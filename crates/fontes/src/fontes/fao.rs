//! FAOSTAT — dados globais de agropecuária da FAO.
//!
//! Bulk downloads em ZIP (CSV dentro), um por domínio. Cada ZIP contém todos
//! os países e anos do domínio. Os arquivos são servidos por
//! `fenixservices.fao.org` e redirecionados para `bulks-faostat.fao.org`.

use std::time::Duration;
use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "fao";
const BASE: &str = "https://fenixservices.fao.org/faostat/static/bulkdownloads";
const HOST: &str = "fenixservices.fao.org";

struct Dominio {
    slug: &'static str,
    arquivo: &'static str,
}

const DOMINIOS: &[Dominio] = &[
    Dominio {
        slug: "producao",
        arquivo: "Production_Crops_Livestock_E_All_Data",
    },
    Dominio {
        slug: "valor-producao",
        arquivo: "Value_of_Production_E_All_Data",
    },
    Dominio {
        slug: "indices-producao",
        arquivo: "Production_Indices_E_All_Data",
    },
    Dominio {
        slug: "comercio",
        arquivo: "Trade_CropsLivestock_E_All_Data",
    },
    Dominio {
        slug: "balanco-alimentar",
        arquivo: "FoodBalanceSheets_E_All_Data",
    },
    Dominio {
        slug: "balanco-alimentar-historico",
        arquivo: "FoodBalanceSheetsHistoric_E_All_Data",
    },
    Dominio {
        slug: "uso-do-solo",
        arquivo: "Inputs_LandUse_E_All_Data",
    },
    Dominio {
        slug: "fertilizantes",
        arquivo: "Inputs_FertilizersNutrient_E_All_Data",
    },
    Dominio {
        slug: "precos",
        arquivo: "Prices_E_All_Data",
    },
    Dominio {
        slug: "emissoes",
        arquivo: "Emissions_Totals_E_All_Data",
    },
    Dominio {
        slug: "populacao",
        arquivo: "Population_E_All_Data",
    },
];

fn url_do_dominio(dominio: &Dominio) -> String {
    format!("{BASE}/{}.zip", dominio.arquivo)
}

/// Dados globais de agropecuária da FAO (FAOSTAT).
pub struct FonteFao;

impl FonteDeDados for FonteFao {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "FAOSTAT - produção, comércio, preços e emissões agrícolas por país (bulk CSV/ZIP)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: None,
            limite: Some(LimiteHttp {
                host: HOST,
                requisicoes: 1,
                intervalo: Duration::from_secs(2),
            }),
            agente: None,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        DOMINIOS
            .iter()
            .map(|dominio| {
                let tarefa = TarefaDeArquivo::nova(
                    dominio.slug.to_owned(),
                    NOME,
                    format!("dominio={}", dominio.slug),
                    url_do_dominio(dominio),
                );
                TarefaPlanejada::nova(tarefa)
                    .com_etiquetas([("dominio", dominio.slug), ("formato", "zip")])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use todeolho_nucleo::FiltroDeTarefas;

    #[test]
    fn total_de_tarefas() {
        assert_eq!(FonteFao.criar_tarefas().len(), DOMINIOS.len());
        assert_eq!(DOMINIOS.len(), 11);
    }

    #[test]
    fn nomes_sao_unicos() {
        let tarefas = FonteFao.criar_tarefas();
        let nomes: std::collections::HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), tarefas.len());
    }

    #[test]
    fn url_do_dominio_segue_padrao() {
        let url = url_do_dominio(&DOMINIOS[0]);
        assert!(url.starts_with("https://fenixservices.fao.org/faostat/static/bulkdownloads/"));
        assert!(url.ends_with(".zip"));
        assert!(url.contains("Production_Crops_Livestock"));
    }

    #[test]
    fn filtra_por_dominio() {
        let tarefas = FonteFao.criar_tarefas();
        let filtro = FiltroDeTarefas::a_partir_de(&["dominio=comercio".to_owned()]).unwrap();
        let filtradas = filtro.aplicar(tarefas).unwrap();
        assert_eq!(filtradas.len(), 1);
        assert_eq!(filtradas[0].tarefa.nome(), "comercio");
    }

    #[test]
    fn etiquetas_presentes() {
        let tarefas = FonteFao.criar_tarefas();
        for tarefa in &tarefas {
            assert!(tarefa.etiquetas.contains_key("dominio"));
            assert_eq!(tarefa.etiquetas.get("formato").unwrap(), "zip");
        }
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteFao.sob_demanda());
    }

    #[test]
    fn declara_limite_e_redirecionamento() {
        let perfil = FonteFao.perfil_http();
        assert_eq!(perfil.limite.as_ref().unwrap().host, HOST);
        assert!(matches!(
            perfil.redirecionamento,
            todeolho_nucleo::RedirecionamentoHttp::Seguir
        ));
        assert!(perfil.credencial.is_none());
    }
}
