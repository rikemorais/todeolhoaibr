use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;

const NOME: &str = "ana";
const BASE: &str = "https://telemetriaws1.ana.gov.br/ServiceANA.asmx/HidroInventario";

struct TipoEstacao {
    slug: &'static str,
    codigo: u8,
    tamanho_tipico: u64,
}

const TIPOS: &[TipoEstacao] = &[
    TipoEstacao {
        slug: "estacoes-fluviometricas",
        codigo: 1,
        tamanho_tipico: 65_000_000,
    },
    TipoEstacao {
        slug: "estacoes-pluviometricas",
        codigo: 2,
        tamanho_tipico: 80_000_000,
    },
];

fn url_inventario(tipo: u8) -> String {
    format!(
        "{BASE}?codEstDE=&codEstATE=&tpEst={tipo}&nmEst=&nmRio=\
         &codSubBacia=&codBacia=&nmMunicipio=&nmEstado=&sgResp=\
         &sgOper=&telemetrica=&statusEstacao=&origin=&"
    )
}

/// Inventário de estações hidrológicas do SNIRH/ANA (ServiceANA).
///
/// Ingere o catálogo completo de estações fluviométricas (vazão/nível) e
/// pluviométricas (chuva) da Agência Nacional de Águas. Cada tipo é um XML
/// DataSet gravado opaco na Raw; o parse fica no refino. Ver issue #48.
///
/// API: <https://telemetriaws1.ana.gov.br/ServiceANA.asmx>
pub struct FonteAna;

impl FonteDeDados for FonteAna {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "ANA/SNIRH - inventário de estações hidrológicas (fluviométricas e pluviométricas)"
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        TIPOS
            .iter()
            .map(|tipo| {
                TarefaPlanejada::nova(
                    TarefaDeIngestaoHttp::sem_paginacao(
                        NOME,
                        tipo.slug,
                        url_inventario(tipo.codigo),
                    )
                    .com_tamanho_tipico(tipo.tamanho_tipico),
                )
                .com_etiquetas([("tipo-estacao", tipo.slug)])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn gera_duas_tarefas_uma_por_tipo_de_estacao() {
        let tarefas = FonteAna.criar_tarefas();
        assert_eq!(tarefas.len(), 2);
    }

    #[test]
    fn primeira_tarefa_e_fluviometrica() {
        let tarefas = FonteAna.criar_tarefas();
        assert_eq!(tarefas[0].tarefa.nome(), "estacoes-fluviometricas");
        assert_eq!(
            tarefas[0].etiquetas.get("tipo-estacao").map(String::as_str),
            Some("estacoes-fluviometricas")
        );
    }

    #[test]
    fn segunda_tarefa_e_pluviometrica() {
        let tarefas = FonteAna.criar_tarefas();
        assert_eq!(tarefas[1].tarefa.nome(), "estacoes-pluviometricas");
    }

    #[test]
    fn urls_apontam_para_service_ana_com_tipo_correto() {
        assert!(url_inventario(1).contains("tpEst=1"));
        assert!(url_inventario(2).contains("tpEst=2"));
        assert!(url_inventario(1).starts_with("https://telemetriaws1.ana.gov.br/"));
    }

    #[test]
    fn nao_exige_credencial() {
        assert!(FonteAna.perfil_http().credencial.is_none());
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteAna.sob_demanda());
    }

    #[test]
    fn host_do_limite_e_telemetria_ana() {
        let limite = FonteAna.perfil_http().limite.expect("limite da ANA");
        assert_eq!(limite.host, "telemetriaws1.ana.gov.br");
    }
}
