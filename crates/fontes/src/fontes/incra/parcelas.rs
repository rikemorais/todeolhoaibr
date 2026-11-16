use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "incra";

const BASE_SHP: &str = "https://certificacao.incra.gov.br/csv_shp/zip";

const BASE_CSV: &str = "https://www.gov.br/incra/pt-br/assuntos/reforma-agraria/assentamentosgeral.csv/@@download/file";

const UFS: [&str; 27] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "DF", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE",
    "PI", "PR", "RJ", "RN", "RO", "RR", "RS", "SC", "SE", "SP", "TO",
];

const UFS_SEM_SNCI: [&str; 3] = ["AC", "DF", "RR"];

/// INCRA — SIGEF / SNCI / Assentamentos: parcelas certificadas de imóveis
/// rurais e projetos de assentamento da reforma agrária.
///
/// Publica shapefiles por UF (SIGEF público e privado, SNCI) e um nacional
/// (assentamentos geoespaciais), além do CSV tabular de projetos de
/// assentamento (SIPRA). Os shapefiles usam EPSG:4674 (SIRGAS 2000).
///
/// O endpoint de shapefiles (`certificacao.incra.gov.br`) tem indisponibilidade
/// intermitente — todas as tarefas de shapefile aceitam ausência.
pub struct FonteIncra;

impl FonteDeDados for FonteIncra {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "INCRA — imóveis rurais certificados (SIGEF/SNCI) e assentamentos"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "certificacao.incra.gov.br",
                requisicoes: 2,
                intervalo: std::time::Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: true,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas = Vec::with_capacity(80);

        for uf in UFS {
            let uf_lower = uf.to_ascii_lowercase();

            tarefas.push(
                TarefaPlanejada::nova(
                    TarefaDeArquivo::nova(
                        format!("sigef-publico-{uf_lower}"),
                        NOME,
                        format!("sigef-publico/uf={uf}"),
                        format!("{BASE_SHP}/Sigef%20P%C3%BAblico_{uf}.zip"),
                    )
                    .aceitar_ausencia(),
                )
                .com_etiquetas([("camada", "sigef-publico"), ("uf", uf)]),
            );

            tarefas.push(
                TarefaPlanejada::nova(
                    TarefaDeArquivo::nova(
                        format!("sigef-privado-{uf_lower}"),
                        NOME,
                        format!("sigef-privado/uf={uf}"),
                        format!("{BASE_SHP}/Sigef%20Privado_{uf}.zip"),
                    )
                    .aceitar_ausencia(),
                )
                .com_etiquetas([("camada", "sigef-privado"), ("uf", uf)]),
            );

            if !UFS_SEM_SNCI.contains(&uf) {
                tarefas.push(
                    TarefaPlanejada::nova(
                        TarefaDeArquivo::nova(
                            format!("snci-{uf_lower}"),
                            NOME,
                            format!("snci/uf={uf}"),
                            format!("{BASE_SHP}/SNCI%20Brasil_{uf}.zip"),
                        )
                        .aceitar_ausencia(),
                    )
                    .com_etiquetas([("camada", "snci"), ("uf", uf)]),
                );
            }
        }

        tarefas.push(
            TarefaPlanejada::nova(
                TarefaDeArquivo::nova(
                    "assentamentos-geo".into(),
                    NOME,
                    "assentamentos-geo".into(),
                    format!("{BASE_SHP}/Assentamento%20Brasil.zip"),
                )
                .aceitar_ausencia(),
            )
            .com_etiquetas([("camada", "assentamentos-geo")]),
        );

        tarefas.push(
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                "assentamentos-csv".into(),
                NOME,
                "assentamentos-csv".into(),
                BASE_CSV.into(),
            ))
            .com_etiquetas([("camada", "assentamentos-csv")]),
        );

        tarefas
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn gera_80_tarefas() {
        let tarefas = FonteIncra.criar_tarefas();
        assert_eq!(tarefas.len(), 80);
    }

    #[test]
    fn nomes_sao_unicos() {
        let tarefas = FonteIncra.criar_tarefas();
        let nomes: std::collections::HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), tarefas.len());
    }

    #[test]
    fn tem_sigef_publico_sp() {
        let tarefas = FonteIncra.criar_tarefas();
        assert!(
            tarefas
                .iter()
                .any(|t| t.tarefa.nome() == "sigef-publico-sp")
        );
    }

    #[test]
    fn tem_snci_mg_e_nao_tem_snci_df() {
        let tarefas = FonteIncra.criar_tarefas();
        assert!(tarefas.iter().any(|t| t.tarefa.nome() == "snci-mg"));
        assert!(!tarefas.iter().any(|t| t.tarefa.nome() == "snci-df"));
        assert!(!tarefas.iter().any(|t| t.tarefa.nome() == "snci-ac"));
        assert!(!tarefas.iter().any(|t| t.tarefa.nome() == "snci-rr"));
    }

    #[test]
    fn tem_assentamentos_geo_e_csv() {
        let tarefas = FonteIncra.criar_tarefas();
        assert!(
            tarefas
                .iter()
                .any(|t| t.tarefa.nome() == "assentamentos-geo")
        );
        assert!(
            tarefas
                .iter()
                .any(|t| t.tarefa.nome() == "assentamentos-csv")
        );
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteIncra.sob_demanda());
    }

    #[test]
    fn aceita_certificado_invalido() {
        assert!(FonteIncra.perfil_http().aceitar_certificado_invalido);
    }

    #[test]
    fn etiquetas_presentes() {
        let tarefas = FonteIncra.criar_tarefas();
        for tarefa in &tarefas {
            assert!(
                tarefa.etiquetas.contains_key("camada"),
                "{}",
                tarefa.tarefa.nome()
            );
        }
    }

    #[test]
    fn contagem_por_camada() {
        let tarefas = FonteIncra.criar_tarefas();
        let sigef_pub = tarefas
            .iter()
            .filter(|t| {
                t.etiquetas
                    .get("camada")
                    .is_some_and(|v| v == "sigef-publico")
            })
            .count();
        let sigef_priv = tarefas
            .iter()
            .filter(|t| {
                t.etiquetas
                    .get("camada")
                    .is_some_and(|v| v == "sigef-privado")
            })
            .count();
        let snci = tarefas
            .iter()
            .filter(|t| t.etiquetas.get("camada").is_some_and(|v| v == "snci"))
            .count();
        assert_eq!(sigef_pub, 27);
        assert_eq!(sigef_priv, 27);
        assert_eq!(snci, 24);
    }
}
