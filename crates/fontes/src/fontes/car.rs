use std::time::Duration;

use todeolho_nucleo::{
    FonteDeDados, LimiteHttp, PerfilHttp, RedirecionamentoHttp, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "car";

const BASE_WFS: &str = "https://geoserver.car.gov.br/geoserver/sicar/wfs";

const UFS: [&str; 27] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "DF", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE",
    "PI", "PR", "RJ", "RN", "RO", "RR", "RS", "SC", "SE", "SP", "TO",
];

fn url_wfs(uf: &str) -> String {
    format!(
        "{BASE_WFS}?service=WFS&version=2.0.0&request=GetFeature\
         &typeNames=sicar:sicar_imoveis_{uf}&outputFormat=SHAPE-ZIP",
        uf = uf.to_lowercase()
    )
}

/// Cadastro Ambiental Rural (CAR) — perímetros de imóveis rurais por UF.
///
/// Baixa SHAPE-ZIP via WFS público do GeoServer do SICAR, sem captcha.
/// Cada UF é uma layer (`sicar:sicar_imoveis_{uf}`) com geometria e atributos
/// do imóvel: código, status, área, município, código IBGE e módulo fiscal.
///
/// Total: ~8,5 milhões de imóveis em 27 UFs.
pub struct FonteCar;

impl FonteDeDados for FonteCar {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Cadastro Ambiental Rural — perímetros de imóveis rurais por UF (SICAR/WFS)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "geoserver.car.gov.br",
                requisicoes: 1,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        UFS.iter()
            .map(|&uf| {
                TarefaPlanejada::nova(TarefaDeArquivo::nova(
                    format!("imoveis/{uf}"),
                    NOME,
                    format!("imoveis/{uf}"),
                    url_wfs(uf),
                ))
                .com_etiquetas([("uf", uf), ("formato", "shapefile")])
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
        FonteCar.criar_tarefas()
    }

    #[test]
    fn total_27_tarefas() {
        assert_eq!(tarefas().len(), 27);
    }

    #[test]
    fn nomes_sao_unicos() {
        let t = tarefas();
        let nomes: HashSet<_> = t.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), t.len());
    }

    #[test]
    fn filtra_por_uf() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["uf=DF".to_owned()])
            .unwrap()
            .aplicar(tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 1);
        assert_eq!(filtradas[0].tarefa.nome(), "imoveis/DF");
    }

    #[test]
    fn url_do_df_usa_wfs() {
        let url = url_wfs("DF");
        assert!(url.starts_with(BASE_WFS));
        assert!(url.contains("typeNames=sicar:sicar_imoveis_df"));
        assert!(url.contains("outputFormat=SHAPE-ZIP"));
    }

    #[test]
    fn etiquetas_presentes() {
        for t in &tarefas() {
            assert!(
                t.etiquetas.contains_key("uf"),
                "{} sem etiqueta uf",
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
        assert!(FonteCar.sob_demanda());
    }

    #[test]
    fn declara_limite_do_host() {
        let limite = FonteCar
            .perfil_http()
            .limite
            .expect("limite do host geoserver");
        assert_eq!(limite.host, "geoserver.car.gov.br");
        assert_eq!(limite.requisicoes, 1);
    }

    #[test]
    fn nao_precisa_credencial() {
        assert!(FonteCar.perfil_http().credencial.is_none());
    }
}
