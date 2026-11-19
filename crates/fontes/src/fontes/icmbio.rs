use std::time::Duration;

use todeolho_nucleo::{
    FonteDeDados, LimiteHttp, PerfilHttp, RedirecionamentoHttp, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "icmbio";

const BASE: &str = "https://dados.mma.gov.br/dataset/44b6dc8a-dc82-4a84-8d95-1b0da7c85dac/resource";

struct Recurso {
    nome: &'static str,
    id: &'static str,
    arquivo: &'static str,
    formato: &'static str,
    periodo: &'static str,
}

const RECURSOS: &[Recurso] = &[
    // --- CSVs tabulares (lista de UCs ativas) ---
    Recurso {
        nome: "cnuc/2018",
        id: "3562d093-14ed-434b-9fa3-411b664d6836",
        arquivo: "unidade-de-conservacao.csv",
        formato: "csv",
        periodo: "2018",
    },
    Recurso {
        nome: "cnuc/2019-01",
        id: "49a0fd37-9fe6-4f9b-91fe-5b9611436c74",
        arquivo: "unidades-de-conservacao_janeiro-2019.csv",
        formato: "csv",
        periodo: "2019-01",
    },
    Recurso {
        nome: "cnuc/2019-07",
        id: "9c661f5d-400e-4188-a67f-0a6b09105408",
        arquivo: "unidades-de-conservacao-2019-2-semestre.csv",
        formato: "csv",
        periodo: "2019-07",
    },
    Recurso {
        nome: "cnuc/2020-01",
        id: "5ffc83b3-2dee-4ed1-86a8-3a70a18094c5",
        arquivo: "cnuc_2020_1-semestre.csv",
        formato: "csv",
        periodo: "2020-01",
    },
    Recurso {
        nome: "cnuc/2020-07",
        id: "c0babb3e-ec4e-4db5-a2b6-b79477260b0f",
        arquivo: "cnuc_2020_2-semestre.csv",
        formato: "csv",
        periodo: "2020-07",
    },
    Recurso {
        nome: "cnuc/2021-01",
        id: "4ad8200c-c102-42b9-bfc6-bae6c2a16ab9",
        arquivo: "cnuc_2021_1-semestre.csv",
        formato: "csv",
        periodo: "2021-01",
    },
    Recurso {
        nome: "cnuc/2021-07",
        id: "baf25448-5064-4ece-9a0e-d778b0eca542",
        arquivo: "cnuc_2021_2-semestre.csv",
        formato: "csv",
        periodo: "2021-07",
    },
    Recurso {
        nome: "cnuc/2022-01",
        id: "fed217ff-fe75-4744-9663-0a06157d9c3b",
        arquivo: "cnuc_2022_01sem.csv",
        formato: "csv",
        periodo: "2022-01",
    },
    Recurso {
        nome: "cnuc/2022-07",
        id: "15ddba57-ec86-43bb-b970-faac4db8497b",
        arquivo: "cnuc_2022_07.csv",
        formato: "csv",
        periodo: "2022-07",
    },
    Recurso {
        nome: "cnuc/2023-07",
        id: "2450dbde-43d8-4693-942a-0456c661059d",
        arquivo: "cnuc_2023_07_v3.xlsx",
        formato: "xlsx",
        periodo: "2023-07",
    },
    Recurso {
        nome: "cnuc/2024-02",
        id: "10dc7e74-3c78-4c6d-b7af-b2ec1fe88eeb",
        arquivo: "cnuc_2024_02-2.csv",
        formato: "csv",
        periodo: "2024-02",
    },
    Recurso {
        nome: "cnuc/2024-10",
        id: "83498949-96ac-45b9-8be5-dcf9db4300eb",
        arquivo: "cnuc_2024_10.csv",
        formato: "csv",
        periodo: "2024-10",
    },
    Recurso {
        nome: "cnuc/2025-03",
        id: "f6bf9940-cf30-4ef2-927d-2bd278e4c8af",
        arquivo: "cnuc_2025_03.csv",
        formato: "csv",
        periodo: "2025-03",
    },
    Recurso {
        nome: "cnuc/2025-08",
        id: "a6e0d1ca-b589-499f-ad48-0c3ee9fb7de1",
        arquivo: "cnuc_2025_08.csv",
        formato: "csv",
        periodo: "2025-08",
    },
    Recurso {
        nome: "cnuc/2026-03",
        id: "bab6d474-d38d-457c-9092-755f23ebdc76",
        arquivo: "cnuc_2026_03_atualizado.csv",
        formato: "csv",
        periodo: "2026-03",
    },
    Recurso {
        nome: "cnuc/2026-07",
        id: "72dd3d2d-3cca-4b97-b382-a1c90531e379",
        arquivo: "cnuc_2026_07.csv",
        formato: "csv",
        periodo: "2026-07",
    },
    // --- Shapefiles (polígonos dos limites) ---
    Recurso {
        nome: "poligono/2024-02",
        id: "9ec98f66-44ad-4397-8583-a1d9cc3a9835",
        arquivo: "shp_cnuc_2024_02.zip",
        formato: "shapefile",
        periodo: "2024-02",
    },
    Recurso {
        nome: "poligono/2024-10",
        id: "7a142cc0-dae9-4a0b-8180-3016994d2932",
        arquivo: "shp_cnuc_2024_10.zip",
        formato: "shapefile",
        periodo: "2024-10",
    },
    Recurso {
        nome: "poligono/2025-03",
        id: "20327e02-d4fe-4a1b-bd12-e381ab461d97",
        arquivo: "shp_cnuc_2025_03.zip",
        formato: "shapefile",
        periodo: "2025-03",
    },
    Recurso {
        nome: "poligono/2025-08",
        id: "6ba9a557-87e8-4882-acb7-b3e0f0ea192d",
        arquivo: "shp_cnuc_2025_08.zip",
        formato: "shapefile",
        periodo: "2025-08",
    },
    // Dicionários de dados
    Recurso {
        nome: "dicionario/tabular",
        id: "1f50ca6b-f045-4c79-b172-33c58a59e667",
        arquivo: "dicionario-de-dados-unidades-de-conservacao.pdf",
        formato: "pdf",
        periodo: "referencia",
    },
    Recurso {
        nome: "dicionario/shapefile",
        id: "71dc3c85-019f-4176-a43e-5aa41b3ec434",
        arquivo: "dicionario-de-dados-shapefile.pdf",
        formato: "pdf",
        periodo: "referencia",
    },
];

fn url(r: &Recurso) -> String {
    format!(
        "{BASE}/{id}/download/{arquivo}",
        id = r.id,
        arquivo = r.arquivo
    )
}

/// Unidades de Conservação do SNUC — ICMBio/CNUC via portal de dados do MMA.
///
/// Baixa CSVs semestrais (lista de UCs ativas com atributos) e SHPs dos
/// polígonos dos limites, publicados em `dados.mma.gov.br`.
///
/// Série: 2018 a 2026, semestral. Inclui UCs federais, estaduais, municipais
/// e RPPNs (particulares).
pub struct FonteIcmbio;

impl FonteDeDados for FonteIcmbio {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Unidades de Conservação do SNUC — CNUC/MMA (atributos e polígonos)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "dados.mma.gov.br",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        RECURSOS
            .iter()
            .map(|r| {
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    r.nome.to_owned(),
                    NOME,
                    r.nome.to_owned(),
                    url(r),
                ))
                .com_etiquetas([("periodo", r.periodo), ("formato", r.formato)])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;
    use todeolho_nucleo::FiltroDeTarefas;

    fn tarefas() -> Vec<TarefaPlanejada> {
        FonteIcmbio.criar_tarefas()
    }

    #[test]
    fn total_22_tarefas() {
        assert_eq!(tarefas().len(), 22);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn filtra_por_periodo() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["periodo=2025-08".to_owned()])
            .unwrap()
            .aplicar(tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 2);
        let nomes: HashSet<_> = filtradas.iter().map(|t| t.tarefa.nome()).collect();
        assert!(nomes.contains("cnuc/2025-08"));
        assert!(nomes.contains("poligono/2025-08"));
    }

    #[test]
    fn filtra_por_formato_shapefile() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["formato=shapefile".to_owned()])
            .unwrap()
            .aplicar(tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 4);
    }

    #[test]
    fn url_do_csv_2026_07() {
        let r = &RECURSOS[15];
        assert_eq!(r.nome, "cnuc/2026-07");
        assert!(url(r).contains("cnuc_2026_07.csv"));
        assert!(url(r).starts_with(BASE));
    }

    #[test]
    fn url_do_shp_2025_08() {
        let r = &RECURSOS[19];
        assert_eq!(r.nome, "poligono/2025-08");
        assert!(url(r).contains("shp_cnuc_2025_08.zip"));
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            assert!(
                t.etiquetas.contains_key("periodo"),
                "{} sem etiqueta periodo",
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
        assert!(FonteIcmbio.sob_demanda());
    }

    #[test]
    fn declara_limite_do_host() {
        let limite = FonteIcmbio
            .perfil_http()
            .limite
            .expect("limite do host MMA");
        assert_eq!(limite.host, "dados.mma.gov.br");
    }

    #[test]
    fn nao_precisa_credencial() {
        assert!(FonteIcmbio.perfil_http().credencial.is_none());
    }
}
