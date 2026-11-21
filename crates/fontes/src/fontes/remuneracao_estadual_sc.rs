//! CSVs mensais de remuneração do Executivo estadual de Santa Catarina.
//!
//! O catálogo CKAN publica arquivos separados para servidores ativos e inativos.
//! A fonte escolhe o par mais recente e preserva os bytes opacos na Raw.

use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use todeolho_nucleo::{
    ContextoDeExecucao, ErroDeIngestao, FonteDeDados, LimiteHttp, PerfilHttp, ResumoDaTarefa,
    Tarefa, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "remuneracao-estadual-sc";
const FONTE_RAW: &str = "remuneracao-estadual";
const CATALOGO: &str =
    "https://dados.sc.gov.br/api/3/action/package_show?id=remuneracao-servidores";
const PREFIXO_DOWNLOAD: &str = "https://dados.sc.gov.br/dataset/";
const HOST: &str = "dados.sc.gov.br";
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
    categoria: &'static str,
    url: String,
}

fn extrair_categoria_e_data(nome: &str) -> Option<(&'static str, &str)> {
    nome.strip_prefix("servidores-ativos-")
        .map(|d| ("ativos", d))
        .or_else(|| {
            nome.strip_prefix("servidores-inativos-")
                .map(|d| ("inativos", d))
        })
}

fn competencia_do_url(url: &str) -> Option<(&'static str, i32, u32, String)> {
    let nome = url.rsplit('/').next()?;
    let nome = nome.strip_suffix(".csv")?;
    let (categoria, data) = extrair_categoria_e_data(nome)?;
    let (ano_str, mes_str) = data.split_once('-')?;
    let ano: i32 = ano_str.parse().ok()?;
    let mes: u32 = mes_str.parse().ok()?;
    if !(1..=12).contains(&mes) || !(2000..=2099).contains(&ano) {
        return None;
    }
    Some((categoria, ano, mes, format!("{ano}-{mes:02}")))
}

fn validar_url(url: &str) -> bool {
    url.starts_with(PREFIXO_DOWNLOAD)
        && !url.contains(['?', '#', '@'])
        && competencia_do_url(url).is_some()
}

fn selecionar_arquivos(pacote: Pacote) -> Result<Vec<ArquivoMensal>, ErroDeIngestao> {
    let mut candidatos: Vec<_> = pacote
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
            competencia_do_url(&recurso.url).map(|(categoria, ano, mes, competencia)| {
                (
                    (ano, mes),
                    ArquivoMensal {
                        competencia,
                        categoria,
                        url: recurso.url,
                    },
                )
            })
        })
        .collect();

    candidatos.sort_by_key(|((ano, mes), _)| std::cmp::Reverse((*ano, *mes)));

    let competencia_mais_recente = candidatos
        .first()
        .map(|((ano, mes), _)| (*ano, *mes))
        .ok_or_else(|| ErroDeIngestao::Http {
            url: CATALOGO.to_owned(),
            detalhe: "o catálogo não contém CSV mensal recente com URL e tamanho válidos".into(),
        })?;

    let arquivos: Vec<_> = candidatos
        .into_iter()
        .filter(|((ano, mes), _)| (*ano, *mes) == competencia_mais_recente)
        .map(|(_, arquivo)| arquivo)
        .collect();

    Ok(arquivos)
}

async fn arquivos_atuais(
    contexto: &ContextoDeExecucao,
) -> Result<Vec<ArquivoMensal>, ErroDeIngestao> {
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
    selecionar_arquivos(catalogo.result.ok_or_else(|| ErroDeIngestao::Http {
        url: CATALOGO.to_owned(),
        detalhe: "resposta CKAN sem o conjunto solicitado".into(),
    })?)
}

struct TarefaRemuneracaoSc {
    categoria: &'static str,
}

fn tarefa_de_arquivo(arquivo: ArquivoMensal) -> TarefaDeArquivo {
    TarefaDeArquivo::nova(
        format!("{}-ultima-competencia", arquivo.categoria),
        FONTE_RAW,
        format!(
            "uf=SC/categoria={}/competencia={}",
            arquivo.categoria, arquivo.competencia
        ),
        arquivo.url,
    )
}

#[async_trait]
impl Tarefa for TarefaRemuneracaoSc {
    fn nome(&self) -> &str {
        match self.categoria {
            "ativos" => "ativos-ultima-competencia",
            _ => "inativos-ultima-competencia",
        }
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let arquivos = arquivos_atuais(contexto).await?;
        let arquivo = arquivos
            .into_iter()
            .find(|a| a.categoria == self.categoria)
            .ok_or_else(|| ErroDeIngestao::Http {
                url: CATALOGO.to_owned(),
                detalhe: format!(
                    "o catálogo não contém CSV de {} na competência mais recente",
                    self.categoria
                ),
            })?;
        tarefa_de_arquivo(arquivo).executar(contexto).await
    }
}

/// Ingestão sob demanda dos CSVs mensais mais recentes de remuneração do Executivo de SC.
pub struct FonteRemuneracaoEstadualSc;

impl FonteDeDados for FonteRemuneracaoEstadualSc {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Dados Abertos SC - remuneração do Executivo estadual (ativos e inativos) por competência"
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
        vec![
            TarefaPlanejada::nova(TarefaRemuneracaoSc {
                categoria: "ativos",
            }),
            TarefaPlanejada::nova(TarefaRemuneracaoSc {
                categoria: "inativos",
            }),
        ]
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn seleciona_par_mais_recente() {
        let pacote: Pacote = serde_json::from_str(
            r#"{"resources":[
                {"format":"CSV","url":"https://dados.sc.gov.br/dataset/x/resource/a/download/servidores-ativos-2026-09.csv","size":22130280},
                {"format":"CSV","url":"https://dados.sc.gov.br/dataset/x/resource/b/download/servidores-inativos-2026-09.csv","size":14460933},
                {"format":"CSV","url":"https://dados.sc.gov.br/dataset/x/resource/c/download/servidores-ativos-2026-10.csv","size":22254421},
                {"format":"CSV","url":"https://dados.sc.gov.br/dataset/x/resource/d/download/servidores-inativos-2026-10.csv","size":14487977}
            ]}"#,
        )
        .unwrap();

        let arquivos = selecionar_arquivos(pacote).unwrap();
        assert_eq!(arquivos.len(), 2);
        assert!(arquivos.iter().all(|a| a.competencia == "2026-10"));
        assert!(arquivos.iter().any(|a| a.categoria == "ativos"));
        assert!(arquivos.iter().any(|a| a.categoria == "inativos"));
    }

    #[test]
    fn rejeita_host_externo_e_competencia_invalida() {
        let pacote: Pacote = serde_json::from_str(
            r#"{"resources":[
                {"format":"CSV","url":"https://example.org/dataset/x/resource/a/download/servidores-ativos-2026-10.csv","size":10},
                {"format":"CSV","url":"https://dados.sc.gov.br/dataset/x/resource/b/download/servidores-ativos-2026-13.csv","size":10}
            ]}"#,
        )
        .unwrap();

        assert!(selecionar_arquivos(pacote).is_err());
    }

    #[test]
    fn rejeita_arquivo_sem_tamanho() {
        let pacote: Pacote = serde_json::from_str(
            r#"{"resources":[
                {"format":"CSV","url":"https://dados.sc.gov.br/dataset/x/resource/a/download/servidores-ativos-2026-10.csv","size":null}
            ]}"#,
        )
        .unwrap();

        let result = selecionar_arquivos(pacote);
        assert!(result.is_err());
    }

    #[test]
    fn fonte_e_sob_demanda_com_duas_tarefas() {
        assert!(FonteRemuneracaoEstadualSc.sob_demanda());
        assert_eq!(FonteRemuneracaoEstadualSc.criar_tarefas().len(), 2);
    }

    #[test]
    fn declara_limite_de_requisicoes() {
        assert_eq!(
            FonteRemuneracaoEstadualSc
                .perfil_http()
                .limite
                .expect("fonte declara limite")
                .host,
            HOST
        );
    }

    #[test]
    fn competencia_do_url_extrai_categoria_mes_e_ano() {
        let url =
            "https://dados.sc.gov.br/dataset/x/resource/y/download/servidores-ativos-2026-10.csv";
        let (cat, ano, mes, comp) = competencia_do_url(url).unwrap();
        assert_eq!(cat, "ativos");
        assert_eq!(ano, 2026);
        assert_eq!(mes, 10);
        assert_eq!(comp, "2026-10");

        let url2 =
            "https://dados.sc.gov.br/dataset/x/resource/y/download/servidores-inativos-2026-09.csv";
        let (cat2, _, _, _) = competencia_do_url(url2).unwrap();
        assert_eq!(cat2, "inativos");
    }
}
