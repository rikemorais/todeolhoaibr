//! Arquivo mensal de remuneração do Executivo estadual de Minas Gerais.
//!
//! A fonte consulta somente os metadados do catálogo CKAN e preserva o CSV mais
//! recente em fluxo na Raw. O conteúdo nominal não é interpretado aqui.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;
use todeolho_nucleo::{
    ContextoDeExecucao, ErroDeIngestao, FonteDeDados, LimiteHttp, PerfilHttp, PrecisaoDaEstimativa,
    ResumoDaTarefa, Tarefa, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "remuneracao-estadual-mg";
const FONTE_RAW: &str = "remuneracao-estadual";
const CATALOGO: &str =
    "https://www.dados.mg.gov.br/api/3/action/package_show?id=remuneracao-servidores-ativos";
const PREFIXOS_DOWNLOAD: [&str; 2] = [
    "https://dados.mg.gov.br/dataset/",
    "https://www.dados.mg.gov.br/dataset/",
];
const TAMANHO_MAXIMO: u64 = 1_073_741_824;
const HOST: &str = "www.dados.mg.gov.br";
// O portal CKAN de MG bloqueia User-Agents de bot com 403.
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
    let data = nome.strip_suffix(".csv")?;
    let (ano, mes) = if let Some(data) = data.strip_prefix("servidores-") {
        let (ano, mes) = data.split_once('-')?;
        (ano.parse().ok()?, mes.parse().ok()?)
    } else if let Some(data) = data.strip_prefix("servidores_") {
        if data.len() != 6 || !data.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        (data[2..].parse().ok()?, data[..2].parse().ok()?)
    } else {
        return None;
    };
    if !(1..=12).contains(&mes) || !(2012..=2099).contains(&ano) {
        return None;
    }
    Some((ano, mes, format!("{ano}-{mes:02}")))
}

fn validar_url(url: &str) -> bool {
    PREFIXOS_DOWNLOAD
        .iter()
        .any(|prefixo| url.starts_with(prefixo))
        && !url.contains(['?', '#', '@'])
        && competencia_do_url(url).is_some()
}

fn selecionar_arquivo(pacote: Pacote) -> Result<ArquivoMensal, ErroDeIngestao> {
    let escolhido = pacote
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
        .map(|(_, arquivo)| arquivo);

    escolhido.ok_or_else(|| ErroDeIngestao::Http {
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

struct TarefaRemuneracaoMg;

fn tarefa_de_arquivo(arquivo: ArquivoMensal) -> TarefaDeArquivo {
    TarefaDeArquivo::nova(
        "ultima-competencia".to_owned(),
        FONTE_RAW,
        format!("uf=MG/competencia={}", arquivo.competencia),
        arquivo.url,
    )
}

#[async_trait]
impl Tarefa for TarefaRemuneracaoMg {
    fn nome(&self) -> &str {
        "ultima-competencia"
    }

    async fn bytes_a_baixar(
        &self,
        contexto: &ContextoDeExecucao,
        precisao: PrecisaoDaEstimativa,
    ) -> Result<Option<u64>, ErroDeIngestao> {
        // Evita consultar o catálogo e fazer HEAD do arquivo no menu de seleção.
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

/// Ingestão sob demanda do CSV mensal mais recente de remuneração do Executivo de MG.
pub struct FonteRemuneracaoEstadualMg;

impl FonteDeDados for FonteRemuneracaoEstadualMg {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Catálogo de Dados Abertos de MG - remuneração do Executivo estadual por competência"
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
        vec![TarefaPlanejada::nova(TarefaRemuneracaoMg)]
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn seleciona_csv_valido_mais_recente_sem_inspecionar_registros() {
        let pacote: Pacote = serde_json::from_str(
            r#"{"resources":[
                {"format":"CSV","url":"https://www.dados.mg.gov.br/dataset/abc/resource/123/download/servidores_082026.csv","size":100},
                {"format":"CSV","url":"https://dados.mg.gov.br/dataset/abc/resource/456/download/servidores-2026-08.csv","size":200},
                {"format":"CSV","url":"https://dados.mg.gov.br/dataset/abc/resource/789/download/servidores-2026-09.csv","size":300},
                {"format":"PDF","url":"https://dados.mg.gov.br/dataset/abc/resource/789/download/servidores-2026-10.csv","size":400}
            ]}"#,
        )
        .unwrap();

        let atual = selecionar_arquivo(pacote).unwrap();
        assert_eq!(atual.competencia, "2026-09");
        assert!(atual.url.ends_with("servidores-2026-09.csv"));
    }

    #[test]
    fn rejeita_url_externa_competencia_invalida_ou_arquivo_sem_tamanho() {
        let pacote: Pacote = serde_json::from_str(
            r#"{"resources":[
                {"format":"CSV","url":"https://example.org/servidores_102026.csv","size":10},
                {"format":"CSV","url":"https://dados.mg.gov.br/dataset/a/resource/b/download/servidores-2026-13.csv","size":10},
                {"format":"CSV","url":"https://dados.mg.gov.br/dataset/a/resource/c/download/servidores-2026-08.csv","size":null}
            ]}"#,
        )
        .unwrap();

        assert!(selecionar_arquivo(pacote).is_err());
    }

    #[test]
    fn fonte_e_sob_demanda() {
        assert!(FonteRemuneracaoEstadualMg.sob_demanda());
        assert_eq!(FonteRemuneracaoEstadualMg.criar_tarefas().len(), 1);
    }
}
