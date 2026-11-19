//! Arquivos mensais de inativos, pensionistas civis, pensão militar e reserva/reforma
//! do Portal da Transparência.

use std::time::Duration;

use chrono::{Datelike, NaiveDate, Utc};
use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "transparencia-inativos";
const FONTE_RAW: &str = "transparencia";
const BASE: &str = "https://portaldatransparencia.gov.br/download-de-dados/servidores";
const PRIMEIRA_COMPETENCIA: (i32, u32) = (2020, 1);

const AGENTE: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";

#[derive(Debug, Clone, Copy)]
enum Tipo {
    AposentadosSiape,
    AposentadosBacen,
    PensionistasSiape,
    PensionistasBacen,
    PensionistasDefesa,
    ReservaReformaMilitares,
}

impl Tipo {
    fn slug(self) -> &'static str {
        match self {
            Self::AposentadosSiape => "aposentados-siape",
            Self::AposentadosBacen => "aposentados-bacen",
            Self::PensionistasSiape => "pensionistas-siape",
            Self::PensionistasBacen => "pensionistas-bacen",
            Self::PensionistasDefesa => "pensionistas-defesa",
            Self::ReservaReformaMilitares => "reserva-reforma-militares",
        }
    }

    fn arquivo(self) -> &'static str {
        match self {
            Self::AposentadosSiape => "Aposentados_SIAPE",
            Self::AposentadosBacen => "Aposentados_BACEN",
            Self::PensionistasSiape => "Pensionistas_SIAPE",
            Self::PensionistasBacen => "Pensionistas_BACEN",
            Self::PensionistasDefesa => "Pensionistas_DEFESA",
            Self::ReservaReformaMilitares => "Reserva_Reforma_Militares",
        }
    }

    fn categoria(self) -> &'static str {
        match self {
            Self::AposentadosSiape | Self::AposentadosBacen => "aposentados",
            Self::PensionistasSiape | Self::PensionistasBacen => "pensionistas-civis",
            Self::PensionistasDefesa => "pensionistas-defesa",
            Self::ReservaReformaMilitares => "reserva-reforma",
        }
    }
}

const TIPOS: [Tipo; 6] = [
    Tipo::AposentadosSiape,
    Tipo::AposentadosBacen,
    Tipo::PensionistasSiape,
    Tipo::PensionistasBacen,
    Tipo::PensionistasDefesa,
    Tipo::ReservaReformaMilitares,
];

fn url(tipo: Tipo, competencia: &str) -> String {
    format!("{BASE}/{}_{}", competencia.replace('-', ""), tipo.arquivo())
}

fn ultima_competencia_publicada() -> (i32, u32) {
    let mes = Utc::now().date_naive().with_day(1).expect("dia válido");
    let mes = mes
        .checked_sub_months(chrono::Months::new(2))
        .unwrap_or(mes);
    (mes.year(), mes.month())
}

fn competencias(inicio: (i32, u32), fim: (i32, u32)) -> Vec<String> {
    let mut atual = NaiveDate::from_ymd_opt(inicio.0, inicio.1, 1).expect("competência válida");
    let limite = NaiveDate::from_ymd_opt(fim.0, fim.1, 1).expect("competência válida");
    let mut resultado = Vec::new();
    while atual <= limite {
        resultado.push(format!("{}-{:02}", atual.year(), atual.month()));
        let Some(proximo) = atual.checked_add_months(chrono::Months::new(1)) else {
            break;
        };
        atual = proximo;
    }
    resultado
}

fn tarefas_ate(fim: (i32, u32)) -> Vec<TarefaPlanejada> {
    TIPOS
        .into_iter()
        .flat_map(|tipo| {
            competencias(PRIMEIRA_COMPETENCIA, fim)
                .into_iter()
                .map(move |competencia| {
                    let tarefa = TarefaDeArquivo::nova(
                        format!("{}_{}", tipo.slug(), competencia),
                        FONTE_RAW,
                        format!("inativos/tipo={}/competencia={competencia}", tipo.slug()),
                        url(tipo, &competencia),
                    );
                    TarefaPlanejada::nova(tarefa).com_etiquetas([
                        ("tipo", tipo.slug()),
                        ("categoria", tipo.categoria()),
                        ("competencia", competencia.as_str()),
                    ])
                })
        })
        .collect()
}

/// Inativos, pensionistas civis e militares do Portal da Transparência.
///
/// Baixa 6 tipos de planilha mensal: aposentados (SIAPE/BACEN), pensionistas
/// civis (SIAPE/BACEN), pensionistas militares (Defesa) e reserva/reforma.
/// Série desde jan/2020. Inclui dados pessoais públicos por lei.
pub struct FonteTransparenciaInativos;

impl FonteDeDados for FonteTransparenciaInativos {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Portal da Transparência - inativos, pensionistas civis e militares, reserva e reforma"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            redirecionamento: todeolho_nucleo::RedirecionamentoHttp::Seguir,
            credencial: None,
            limite: Some(LimiteHttp {
                host: "portaldatransparencia.gov.br",
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: Some(AGENTE),
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        tarefas_ate(ultima_competencia_publicada())
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;
    use todeolho_nucleo::FiltroDeTarefas;

    #[test]
    fn gera_tarefas_por_tipo_e_competencia() {
        let tarefas = tarefas_ate((2020, 2));
        assert_eq!(tarefas.len(), 12);
        assert_eq!(tarefas[0].tarefa.nome(), "aposentados-siape_2020-01");
        assert_eq!(tarefas[0].etiquetas["tipo"], "aposentados-siape");
        assert_eq!(tarefas[0].etiquetas["categoria"], "aposentados");
        assert_eq!(tarefas[0].etiquetas["competencia"], "2020-01");
    }

    #[test]
    fn seis_tipos_por_competencia() {
        let tarefas = tarefas_ate((2020, 1));
        assert_eq!(tarefas.len(), 6);
        let nomes: HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome().to_owned()).collect();
        assert!(nomes.contains("aposentados-siape_2020-01"));
        assert!(nomes.contains("aposentados-bacen_2020-01"));
        assert!(nomes.contains("pensionistas-siape_2020-01"));
        assert!(nomes.contains("pensionistas-bacen_2020-01"));
        assert!(nomes.contains("pensionistas-defesa_2020-01"));
        assert!(nomes.contains("reserva-reforma-militares_2020-01"));
    }

    #[test]
    fn urls_seguem_padrao_do_portal() {
        assert_eq!(
            url(Tipo::AposentadosSiape, "2020-01"),
            "https://portaldatransparencia.gov.br/download-de-dados/servidores/202001_Aposentados_SIAPE"
        );
        assert_eq!(
            url(Tipo::PensionistasDefesa, "2024-06"),
            "https://portaldatransparencia.gov.br/download-de-dados/servidores/202406_Pensionistas_DEFESA"
        );
        assert_eq!(
            url(Tipo::ReservaReformaMilitares, "2024-01"),
            "https://portaldatransparencia.gov.br/download-de-dados/servidores/202401_Reserva_Reforma_Militares"
        );
    }

    #[test]
    fn filtra_por_categoria() {
        let tarefas = tarefas_ate((2020, 1));
        let filtradas = FiltroDeTarefas::a_partir_de(&["categoria=pensionistas-defesa".to_owned()])
            .unwrap()
            .aplicar(tarefas)
            .unwrap();
        assert_eq!(filtradas.len(), 1);
        assert_eq!(filtradas[0].tarefa.nome(), "pensionistas-defesa_2020-01");
    }

    #[test]
    fn filtra_por_tipo() {
        let tarefas = tarefas_ate((2020, 2));
        let filtradas =
            FiltroDeTarefas::a_partir_de(&["tipo=reserva-reforma-militares".to_owned()])
                .unwrap()
                .aplicar(tarefas)
                .unwrap();
        assert_eq!(filtradas.len(), 2);
    }

    #[test]
    fn competencias_avancam_de_ano_sem_lacunas() {
        assert_eq!(
            competencias((2020, 11), (2021, 2)),
            ["2020-11", "2020-12", "2021-01", "2021-02"]
        );
    }

    #[test]
    fn nomes_sao_unicos() {
        let tarefas = tarefas_ate((2020, 3));
        let nomes: HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), tarefas.len());
    }

    #[test]
    fn e_sob_demanda() {
        assert!(FonteTransparenciaInativos.sob_demanda());
    }

    #[test]
    fn usa_agente_de_navegador_para_evitar_waf() {
        let perfil = FonteTransparenciaInativos.perfil_http();
        assert!(perfil.agente.is_some());
        assert!(perfil.agente.unwrap().contains("Mozilla"));
    }

    #[test]
    fn limita_requisicoes_ao_portal() {
        let limite = FonteTransparenciaInativos
            .perfil_http()
            .limite
            .expect("limite do portal");
        assert_eq!(limite.host, "portaldatransparencia.gov.br");
    }

    #[test]
    fn nao_precisa_credencial() {
        assert!(
            FonteTransparenciaInativos
                .perfil_http()
                .credencial
                .is_none()
        );
    }

    #[test]
    fn recurso_raw_inclui_tipo_e_competencia() {
        let tarefas = tarefas_ate((2020, 1));
        for t in &tarefas {
            let nome = t.tarefa.nome();
            let tipo = &t.etiquetas["tipo"];
            let comp = &t.etiquetas["competencia"];
            assert!(
                nome.contains(tipo),
                "nome {nome} deveria conter tipo {tipo}"
            );
            assert!(
                nome.contains(comp),
                "nome {nome} deveria conter competencia {comp}"
            );
        }
    }
}
