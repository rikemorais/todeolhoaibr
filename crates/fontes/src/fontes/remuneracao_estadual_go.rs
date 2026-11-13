//! CSV mensal de remuneração do Executivo estadual de Goiás.
//!
//! A fonte consulta somente metadados CKAN, escolhe o CSV mensal mais recente e
//! preserva os bytes em fluxo na Raw, sem interpretar registros nominais.

use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use todeolho_nucleo::{
    ContextoDeExecucao, ErroDeIngestao, FonteDeDados, LimiteHttp, PerfilHttp, PrecisaoDaEstimativa,
    ResumoDaTarefa, Tarefa, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "remuneracao-estadual-go";
const FONTE_RAW: &str = "remuneracao-estadual";
const CATALOGO: &str =
    "https://dadosabertos.go.gov.br/api/3/action/package_show?id=folha-de-pagamento";
const PREFIXO_DOWNLOAD: &str = "https://dadosabertos.go.gov.br/dataset/";
const HOST: &str = "dadosabertos.go.gov.br";
const TAMANHO_MAXIMO: u64 = 1_073_741_824;
const AGENTE: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";

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
    let data = nome.strip_prefix("folhapagamento_")?.strip_suffix(".csv")?;
    if data.len() != 6 || !data.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let ano: i32 = data[..4].parse().ok()?;
    let mes: u32 = data[4..].parse().ok()?;
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

struct TarefaRemuneracaoGo;

fn tarefa_de_arquivo(arquivo: ArquivoMensal) -> TarefaDeArquivo {
    TarefaDeArquivo::nova(
        "ultima-competencia".to_owned(),
        FONTE_RAW,
        format!("uf=GO/competencia={}", arquivo.competencia),
        arquivo.url,
    )
}

#[async_trait]
impl Tarefa for TarefaRemuneracaoGo {
    fn nome(&self) -> &str {
        "ultima-competencia"
    }

    async fn bytes_a_baixar(
        &self,
        contexto: &ContextoDeExecucao,
        precisao: PrecisaoDaEstimativa,
    ) -> Result<Option<u64>, ErroDeIngestao> {
        if precisao == PrecisaoDaEstimativa::Rapida {
            return Ok(None);
        }
        let arquivo = arquivo_atual(contexto).await?;
        tarefa_de_arquivo(arquivo)
            .bytes_a_baixar(contexto, precisao)
            .await
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let arquivo = arquivo_atual(contexto).await?;
        tarefa_de_arquivo(arquivo).executar(contexto).await
    }
}

/// Ingestão sob demanda do CSV mensal mais recente de remuneração do Executivo de GO.
pub struct FonteRemuneracaoEstadualGo;

impl FonteDeDados for FonteRemuneracaoEstadualGo {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Dados Abertos Goiás - folha mensal de remuneração do Executivo estadual"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: HOST,
                requisicoes: 2,
                intervalo: Duration::from_secs(1),
            }),
            agente: Some(AGENTE),
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        vec![TarefaPlanejada::nova(TarefaRemuneracaoGo)]
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn seleciona_o_csv_mensal_mais_recente_e_ignora_o_zip_anual() {
        let pacote: Pacote = serde_json::from_str(
            r#"{"resources":[
                {"format":"CSV","url":"https://dadosabertos.go.gov.br/dataset/a/resource/b/download/folhapagamento_202601.csv","size":100},
                {"format":"CSV","url":"https://dadosabertos.go.gov.br/dataset/a/resource/c/download/folhapagamento_202608.csv","size":200},
                {"format":"ZIP","url":"https://dadosabertos.go.gov.br/dataset/a/resource/d/download/folhapagamento_ano_2025.zip","size":300}
            ]}"#,
        )
        .unwrap();

        let atual = selecionar_arquivo(pacote).unwrap();
        assert_eq!(atual.competencia, "2026-08");
        assert!(atual.url.ends_with("folhapagamento_202608.csv"));
    }

    #[test]
    fn rejeita_host_externo_competencia_invalida_e_arquivo_sem_tamanho() {
        let pacote: Pacote = serde_json::from_str(
            r#"{"resources":[
                {"format":"CSV","url":"https://example.org/folhapagamento_202609.csv","size":10},
                {"format":"CSV","url":"https://dadosabertos.go.gov.br/dataset/a/resource/b/download/folhapagamento_202613.csv","size":10},
                {"format":"CSV","url":"https://dadosabertos.go.gov.br/dataset/a/resource/c/download/folhapagamento_202608.csv","size":null}
            ]}"#,
        )
        .unwrap();

        assert!(selecionar_arquivo(pacote).is_err());
    }

    #[test]
    fn fonte_e_sob_demanda_e_declara_limite_de_requisicoes() {
        assert!(FonteRemuneracaoEstadualGo.sob_demanda());
        assert_eq!(FonteRemuneracaoEstadualGo.criar_tarefas().len(), 1);
        assert_eq!(
            FonteRemuneracaoEstadualGo
                .perfil_http()
                .limite
                .expect("fonte declara limite")
                .host,
            HOST
        );
    }
}
