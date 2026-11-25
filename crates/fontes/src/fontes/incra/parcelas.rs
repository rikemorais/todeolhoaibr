use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "incra";

const BASE_WFS: &str = "https://geoportal.incra.gov.br/geoserver/ows";

const BASE_CSV: &str = "https://www.gov.br/incra/pt-br/assuntos/reforma-agraria/assentamentosgeral.csv/@@download/file";

const UFS: [(&str, u8); 27] = [
    ("AC", 12),
    ("AL", 27),
    ("AM", 13),
    ("AP", 16),
    ("BA", 29),
    ("CE", 23),
    ("DF", 53),
    ("ES", 32),
    ("GO", 52),
    ("MA", 21),
    ("MG", 31),
    ("MS", 50),
    ("MT", 51),
    ("PA", 15),
    ("PB", 25),
    ("PE", 26),
    ("PI", 22),
    ("PR", 41),
    ("RJ", 33),
    ("RN", 24),
    ("RO", 11),
    ("RR", 14),
    ("RS", 43),
    ("SC", 42),
    ("SE", 28),
    ("SP", 35),
    ("TO", 17),
];

const NATUREZA_SIGEF_PUBLICO: &str = "Gleba Pública";
const NATUREZA_SIGEF_PRIVADO: &str = "Particular";
const NATUREZA_SIGEF_ASSENTAMENTO: &str = "Assentamento Parcela";

fn url_sigef(uf_id: u8, natureza: &str) -> String {
    let mut url = url::Url::parse(BASE_WFS).expect("URL do Geoportal do INCRA é válida");
    let filtro = format!("uf_id={uf_id} AND natureza='{natureza}'");
    url.query_pairs_mut()
        .append_pair("service", "WFS")
        .append_pair("version", "2.0.0")
        .append_pair("request", "GetFeature")
        .append_pair("typeNames", "geonode:sigef_geo")
        .append_pair("outputFormat", "shape-zip")
        .append_pair("CQL_FILTER", &filtro);
    url.into()
}

/// INCRA — SIGEF e projetos de assentamento da reforma agrária.
///
/// Baixa do Geoportal público do INCRA as parcelas SIGEF por UF e natureza,
/// além do CSV tabular de projetos de assentamento (SIPRA). As camadas SNCI e
/// os perímetros geográficos de assentamentos deixaram de ser ingeridos porque
/// o Acervo Fundiário passou a exigir autenticação gov.br.
pub struct FonteIncra;

impl FonteDeDados for FonteIncra {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "INCRA — parcelas SIGEF e projetos de assentamento"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "geoportal.incra.gov.br",
                requisicoes: 2,
                intervalo: std::time::Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Bloquear,
            aceitar_certificado_invalido: false,
        }
    }

    fn tempo_sem_dados(&self) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(300))
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas = Vec::with_capacity(82);

        for (uf, uf_id) in UFS {
            let uf_lower = uf.to_ascii_lowercase();

            tarefas.push(
                TarefaPlanejada::nova(
                    TarefaDeArquivo::nova(
                        format!("sigef-publico-{uf_lower}"),
                        NOME,
                        format!("sigef-publico/uf={uf}"),
                        url_sigef(uf_id, NATUREZA_SIGEF_PUBLICO),
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
                        url_sigef(uf_id, NATUREZA_SIGEF_PRIVADO),
                    )
                    .aceitar_ausencia(),
                )
                .com_etiquetas([("camada", "sigef-privado"), ("uf", uf)]),
            );

            tarefas.push(
                TarefaPlanejada::nova(
                    TarefaDeArquivo::nova(
                        format!("sigef-assentamento-{uf_lower}"),
                        NOME,
                        format!("sigef-assentamento/uf={uf}"),
                        url_sigef(uf_id, NATUREZA_SIGEF_ASSENTAMENTO),
                    )
                    .aceitar_ausencia(),
                )
                .com_etiquetas([("camada", "sigef-assentamento"), ("uf", uf)]),
            );
        }

        tarefas.push(
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                "assentamentos-csv".into(),
                NOME,
                "assentamentos-csv/projetos".into(),
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
    fn gera_82_tarefas() {
        let tarefas = FonteIncra.criar_tarefas();
        assert_eq!(tarefas.len(), 82);
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
    fn nao_inclui_camadas_legadas_sem_download_publico() {
        let tarefas = FonteIncra.criar_tarefas();
        assert!(!tarefas.iter().any(|t| t.tarefa.nome().starts_with("snci-")));
        assert!(
            !tarefas
                .iter()
                .any(|t| t.tarefa.nome() == "assentamentos-geo")
        );
    }

    #[test]
    fn tem_parcelas_sigef_de_assentamento_e_csv() {
        let tarefas = FonteIncra.criar_tarefas();
        assert!(
            tarefas
                .iter()
                .any(|t| t.tarefa.nome() == "sigef-assentamento-ac")
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
    fn valida_certificado_tls() {
        assert!(!FonteIncra.perfil_http().aceitar_certificado_invalido);
    }

    #[test]
    fn tempo_sem_dados_ampliado() {
        let tempo = FonteIncra.tempo_sem_dados().expect("INCRA precisa de timeout estendido");
        assert!(tempo >= std::time::Duration::from_secs(180));
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
        let sigef_assentamento = tarefas
            .iter()
            .filter(|t| {
                t.etiquetas
                    .get("camada")
                    .is_some_and(|v| v == "sigef-assentamento")
            })
            .count();
        assert_eq!(sigef_pub, 27);
        assert_eq!(sigef_priv, 27);
        assert_eq!(sigef_assentamento, 27);
    }
}
