//! CSV mensal de remuneração do Executivo estadual de Mato Grosso do Sul.
//!
//! A fonte consulta somente metadados CKAN, escolhe o CSV mensal mais recente e
//! preserva os bytes em fluxo na Raw, sem interpretar registros nominais.

use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use todeolho_nucleo::{
    ContextoDeExecucao, ErroDeIngestao, FonteDeDados, LimiteHttp, PerfilHttp, ResumoDaTarefa,
    Tarefa, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "remuneracao-estadual-ms";
const FONTE_RAW: &str = "remuneracao-estadual";
const CATALOGO: &str =
    "https://www.dados.ms.gov.br/api/3/action/package_show?id=folha-de-pagamento";
const PREFIXO_DOWNLOAD: &str = "https://www.dados.ms.gov.br/dataset/";
const HOST: &str = "www.dados.ms.gov.br";
const TAMANHO_MAXIMO: u64 = 1_073_741_824;

#[derive(Debug, Deserialize)]
struct RespostaCatalogo {
    success: bool,
    result: Option<Pacote>,
}

#[derive(Debug, Deserialize)]
struct Pacote {
    #[serde(default)]
    resources: Vec<Recurso>,
}

#[derive(Debug, Deserialize)]
struct Recurso {
    format: String,
    url: String,
    size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArquivoMensal {
    competencia: String,
    url: String,
}

fn competencia_do_url(url: &str) -> Option<(i32, u32, String)> {
    let nome = url.rsplit('/').next()?;
    let data = nome.strip_prefix("folha-")?.strip_suffix(".csv")?;
    let (mes_str, ano_str) = data.split_once('_')?;
    let mes: u32 = mes_str.parse().ok()?;
    let ano: i32 = ano_str.parse().ok()?;
    if !(1..=12).contains(&mes) || !(2000..=2099).contains(&ano) {
        return None;
    }
    Some((ano, mes, format!("{ano}-{mes:02}")))
}

fn validar_url(url: &str) -> bool {
    url.starts_with(PREFIXO_DOWNLOAD)
        && !url.contains(['?', '#', '@'])
        && competencia_do_url(url).is_some()
}

fn selecionar_arquivo(pacote: Pacote) -> Result<ArquivoMensal, ErroDeIngestao> {
    pacote
        .resources
        .into_iter()
        .filter(|recurso| recurso.format.eq_ignore_ascii_case("csv"))
        .filter(|recurso| validar_url(&recurso.url))
        .filter(|recurso| {
            recurso
                .size
                .is_some_and(|tamanho| tamanho > 0 && tamanho <= TAMANHO_MAXIMO)
        })
        .filter_map(|recurso| {
            competencia_do_url(&recurso.url).map(|(ano, mes, competencia)| {
                (
                    (ano, mes),
                    ArquivoMensal {
                        competencia,
                        url: recurso.url,
                    },
                )
            })
        })
        .max_by_key(|(data, _)| *data)
        .map(|(_, arquivo)| arquivo)
        .ok_or_else(|| ErroDeIngestao::Http {
            url: CATALOGO.to_owned(),
            detalhe: "o catálogo não contém CSV mensal recente com URL e tamanho válidos".into(),
        })
}

async fn arquivo_atual(contexto: &ContextoDeExecucao) -> Result<ArquivoMensal, ErroDeIngestao> {
    let resposta = contexto.http.obter(CATALOGO, "application/json").await?;
    let catalogo: RespostaCatalogo =
        serde_json::from_slice(&resposta.corpo).map_err(|erro| ErroDeIngestao::Http {
            url: CATALOGO.to_owned(),
            detalhe: format!("resposta CKAN inválida: {erro}"),
        })?;
    if !catalogo.success {
        return Err(ErroDeIngestao::Http {
            url: CATALOGO.to_owned(),
            detalhe: "o catálogo CKAN informou falha ao consultar o conjunto".into(),
        });
    }
    selecionar_arquivo(catalogo.result.ok_or_else(|| ErroDeIngestao::Http {
        url: CATALOGO.to_owned(),
        detalhe: "resposta CKAN sem o conjunto solicitado".into(),
    })?)
}

struct TarefaRemuneracaoMs;

fn tarefa_de_arquivo(arquivo: ArquivoMensal) -> TarefaDeArquivo {
    TarefaDeArquivo::nova(
        "ultima-competencia".to_owned(),
        FONTE_RAW,
        format!("uf=MS/competencia={}", arquivo.competencia),
        arquivo.url,
    )
}

#[async_trait]
impl Tarefa for TarefaRemuneracaoMs {
    fn nome(&self) -> &str {
        "ultima-competencia"
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let arquivo = arquivo_atual(contexto).await?;
        tarefa_de_arquivo(arquivo).executar(contexto).await
    }
}

/// Ingestão sob demanda do CSV mensal mais recente de remuneração do Executivo de MS.
pub struct FonteRemuneracaoEstadualMs;

impl FonteDeDados for FonteRemuneracaoEstadualMs {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Dados Abertos MS - remuneração do Executivo estadual por competência"
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
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        vec![TarefaPlanejada::nova(TarefaRemuneracaoMs)]
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn seleciona_o_csv_mensal_mais_recente() {
        let pacote: Pacote = serde_json::from_str(
            r#"{"resources":[
                {"format":"CSV","url":"https://www.dados.ms.gov.br/dataset/be6c9e9f/resource/aaa/download/folha-08_2026.csv","size":14856152},
                {"format":"CSV","url":"https://www.dados.ms.gov.br/dataset/be6c9e9f/resource/bbb/download/folha-10_2026.csv","size":14904348},
                {"format":"CSV","url":"https://www.dados.ms.gov.br/dataset/be6c9e9f/resource/ccc/download/folha-09_2026.csv","size":14901116}
            ]}"#,
        )
        .unwrap();

        let atual = selecionar_arquivo(pacote).unwrap();
        assert_eq!(atual.competencia, "2026-10");
        assert!(atual.url.contains("folha-10_2026.csv"));
    }

    #[test]
    fn rejeita_host_externo_competencia_invalida_e_arquivo_sem_tamanho() {
        let pacote: Pacote = serde_json::from_str(
            r#"{"resources":[
                {"format":"CSV","url":"https://example.org/dataset/x/resource/y/download/folha-10_2026.csv","size":10},
                {"format":"CSV","url":"https://www.dados.ms.gov.br/dataset/x/resource/y/download/folha-13_2026.csv","size":10},
                {"format":"CSV","url":"https://www.dados.ms.gov.br/dataset/x/resource/y/download/folha-10_2026.csv","size":null}
            ]}"#,
        )
        .unwrap();

        assert!(selecionar_arquivo(pacote).is_err());
    }

    #[test]
    fn ignora_meses_13_do_catalogo() {
        let pacote: Pacote = serde_json::from_str(
            r#"{"resources":[
                {"format":"CSV","url":"https://www.dados.ms.gov.br/dataset/x/resource/y/download/folha-13_2018.csv","size":9747617},
                {"format":"CSV","url":"https://www.dados.ms.gov.br/dataset/x/resource/y/download/folha-12_2018.csv","size":11320313}
            ]}"#,
        )
        .unwrap();

        let atual = selecionar_arquivo(pacote).unwrap();
        assert_eq!(atual.competencia, "2018-12");
    }

    #[test]
    fn fonte_e_sob_demanda_e_declara_limite_de_requisicoes() {
        assert!(FonteRemuneracaoEstadualMs.sob_demanda());
        assert_eq!(FonteRemuneracaoEstadualMs.criar_tarefas().len(), 1);
        assert_eq!(
            FonteRemuneracaoEstadualMs
                .perfil_http()
                .limite
                .expect("fonte declara limite")
                .host,
            HOST
        );
    }

    #[test]
    fn competencia_do_url_extrai_mes_e_ano() {
        let url = "https://www.dados.ms.gov.br/dataset/x/resource/y/download/folha-10_2026.csv";
        let (ano, mes, comp) = competencia_do_url(url).unwrap();
        assert_eq!(ano, 2026);
        assert_eq!(mes, 10);
        assert_eq!(comp, "2026-10");
    }
}
