//! Painel web das execuções de ingestão (ver `docs/adr/0008`).

#![warn(missing_docs)]

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Json, Response};
use axum::routing::{get, post};
use axum::{Router, middleware};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use todeolho_nucleo::{
    CatalogoDeFontes, ConsultaDeExecucoes, RegistroDeExecucao, SinalizadorDeExecucoes,
    SituacaoDaExecucao,
};
use tracing::{error, info, warn};

const LIMITE_DE_EXECUCOES: usize = 200;
const POR_PAGINA_PADRAO: usize = 20;
const POR_PAGINA_MAXIMO: usize = 100;
const PAGINA: &str = include_str!("pagina.html");
/// A página não carrega nada externo e só conversa com o próprio painel.
const POLITICA_DE_CONTEUDO: &str = "default-src 'none'; style-src 'unsafe-inline'; \
     script-src 'unsafe-inline'; connect-src 'self'; base-uri 'none'; form-action 'none'";

#[derive(Serialize)]
struct ExecucaoVisivel {
    situacao: SituacaoDaExecucao,
    #[serde(flatten)]
    registro: RegistroDeExecucao,
}

#[derive(Serialize)]
struct RespostaPaginada {
    dados: Vec<ExecucaoVisivel>,
    total: usize,
    pagina: usize,
    por_pagina: usize,
    total_paginas: usize,
    ha_andamento: bool,
}

#[derive(Deserialize)]
struct FiltroDeExecucoes {
    situacao: Option<String>,
    pagina: Option<usize>,
    por_pagina: Option<usize>,
}

#[derive(Serialize)]
struct RespostaDeErro {
    erro: &'static str,
}

#[derive(Clone)]
struct EstadoDoPainel {
    consulta: Arc<dyn ConsultaDeExecucoes>,
    sinalizador: Option<Arc<dyn SinalizadorDeExecucoes>>,
    catalogo: Option<Arc<dyn CatalogoDeFontes>>,
}

/// Monta as rotas do painel.
pub fn rotas(consulta: Arc<dyn ConsultaDeExecucoes>) -> Router {
    rotas_completas(consulta, None, None)
}

/// Monta as rotas do painel com suporte a interrupção de execuções.
pub fn rotas_com_sinalizador(
    consulta: Arc<dyn ConsultaDeExecucoes>,
    sinalizador: Option<Arc<dyn SinalizadorDeExecucoes>>,
) -> Router {
    rotas_completas(consulta, sinalizador, None)
}

/// Monta as rotas do painel com todas as capacidades opcionais.
pub fn rotas_completas(
    consulta: Arc<dyn ConsultaDeExecucoes>,
    sinalizador: Option<Arc<dyn SinalizadorDeExecucoes>>,
    catalogo: Option<Arc<dyn CatalogoDeFontes>>,
) -> Router {
    let estado = EstadoDoPainel {
        consulta,
        sinalizador,
        catalogo,
    };
    Router::new()
        .route("/", get(pagina))
        .route("/api/execucoes", get(execucoes))
        .route("/api/execucoes/{id}/interromper", post(interromper))
        .route("/api/fontes", get(listar_fontes))
        .layer(middleware::map_response(cabecalhos_de_seguranca))
        .with_state(estado)
}

/// Serve o painel até o processo ser encerrado.
pub async fn servir(
    endereco: SocketAddr,
    consulta: Arc<dyn ConsultaDeExecucoes>,
) -> std::io::Result<()> {
    servir_completo(endereco, consulta, None, None).await
}

/// Serve o painel com suporte a interrupção.
pub async fn servir_com_sinalizador(
    endereco: SocketAddr,
    consulta: Arc<dyn ConsultaDeExecucoes>,
    sinalizador: Option<Arc<dyn SinalizadorDeExecucoes>>,
) -> std::io::Result<()> {
    servir_completo(endereco, consulta, sinalizador, None).await
}

/// Serve o painel com todas as capacidades opcionais.
pub async fn servir_completo(
    endereco: SocketAddr,
    consulta: Arc<dyn ConsultaDeExecucoes>,
    sinalizador: Option<Arc<dyn SinalizadorDeExecucoes>>,
    catalogo: Option<Arc<dyn CatalogoDeFontes>>,
) -> std::io::Result<()> {
    let ouvinte = tokio::net::TcpListener::bind(endereco).await?;
    info!(%endereco, "painel disponível");
    axum::serve(ouvinte, rotas_completas(consulta, sinalizador, catalogo)).await
}

async fn pagina() -> Html<&'static str> {
    Html(PAGINA)
}

fn parse_situacao(valor: &str) -> Option<SituacaoDaExecucao> {
    match valor {
        "sucesso" => Some(SituacaoDaExecucao::Sucesso),
        "com_falhas" => Some(SituacaoDaExecucao::ComFalhas),
        "em_andamento" => Some(SituacaoDaExecucao::EmAndamento),
        "interrompida" => Some(SituacaoDaExecucao::Interrompida),
        "cancelada" => Some(SituacaoDaExecucao::Cancelada),
        _ => None,
    }
}

async fn execucoes(
    State(estado): State<EstadoDoPainel>,
    Query(filtro): Query<FiltroDeExecucoes>,
) -> Response {
    match estado.consulta.recentes(LIMITE_DE_EXECUCOES).await {
        Ok(registros) => {
            let agora = Utc::now();
            let mut visiveis: Vec<ExecucaoVisivel> = registros
                .into_iter()
                .map(|registro| ExecucaoVisivel {
                    situacao: registro.situacao(agora),
                    registro,
                })
                .collect();

            let ha_andamento = visiveis
                .iter()
                .any(|e| e.situacao == SituacaoDaExecucao::EmAndamento);

            if let Some(ref valor) = filtro.situacao {
                match parse_situacao(valor) {
                    Some(situacao) => visiveis.retain(|e| e.situacao == situacao),
                    None => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(RespostaDeErro {
                                erro: "Valor de situação inválido.",
                            }),
                        )
                            .into_response();
                    }
                }
            }

            let total = visiveis.len();
            let por_pagina = filtro
                .por_pagina
                .unwrap_or(POR_PAGINA_PADRAO)
                .clamp(1, POR_PAGINA_MAXIMO);
            let total_paginas = total.div_ceil(por_pagina).max(1);
            let pagina_num = filtro.pagina.unwrap_or(1).clamp(1, total_paginas);
            let inicio = (pagina_num - 1) * por_pagina;
            let dados: Vec<ExecucaoVisivel> =
                visiveis.into_iter().skip(inicio).take(por_pagina).collect();

            Json(RespostaPaginada {
                dados,
                total,
                pagina: pagina_num,
                por_pagina,
                total_paginas,
                ha_andamento,
            })
            .into_response()
        }
        Err(erro) => {
            error!(%erro, "não foi possível consultar o histórico");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(RespostaDeErro {
                    erro: "Não foi possível consultar o armazenamento.",
                }),
            )
                .into_response()
        }
    }
}

async fn interromper(
    State(estado): State<EstadoDoPainel>,
    Path(id): Path<String>,
) -> Response {
    let Some(sinalizador) = &estado.sinalizador else {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(RespostaDeErro {
                erro: "Interrupção não disponível nesta instância.",
            }),
        )
            .into_response();
    };

    match sinalizador.sinalizar_interrupcao(&id).await {
        Ok(()) => {
            info!(execucao = %id, "sinal de interrupção enviado");
            Json(serde_json::json!({"ok": true})).into_response()
        }
        Err(erro) => {
            warn!(execucao = %id, %erro, "falha ao sinalizar interrupção");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(RespostaDeErro {
                    erro: "Não foi possível sinalizar a interrupção.",
                }),
            )
                .into_response()
        }
    }
}

async fn listar_fontes(State(estado): State<EstadoDoPainel>) -> Response {
    let Some(catalogo) = &estado.catalogo else {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(RespostaDeErro {
                erro: "Catálogo de fontes não disponível nesta instância.",
            }),
        )
            .into_response();
    };

    Json(catalogo.fontes()).into_response()
}

async fn cabecalhos_de_seguranca(mut resposta: Response) -> Response {
    let cabecalhos = resposta.headers_mut();
    cabecalhos.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(POLITICA_DE_CONTEUDO),
    );
    cabecalhos.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    cabecalhos.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    resposta
}

#[cfg(test)]
mod testes {
    use async_trait::async_trait;
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use chrono::TimeDelta;
    use todeolho_nucleo::ErroDeIngestao;
    use tower::ServiceExt;

    use super::*;

    struct ConsultaFalsa(Result<Vec<RegistroDeExecucao>, ()>);

    #[async_trait]
    impl ConsultaDeExecucoes for ConsultaFalsa {
        async fn recentes(&self, _: usize) -> Result<Vec<RegistroDeExecucao>, ErroDeIngestao> {
            self.0.clone().map_err(|()| ErroDeIngestao::Armazenamento {
                chave: "execucoes".into(),
                detalhe: "segredo interno".into(),
            })
        }
    }

    async fn obter(consulta: ConsultaFalsa, caminho: &str) -> (StatusCode, String, Option<String>) {
        let resposta = rotas(Arc::new(consulta))
            .oneshot(Request::get(caminho).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = resposta.status();
        let politica = resposta
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .map(|valor| valor.to_str().unwrap().to_owned());
        let corpo = to_bytes(resposta.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(corpo.to_vec()).unwrap(), politica)
    }

    fn registro(id: &str, atraso: TimeDelta) -> RegistroDeExecucao {
        let instante = Utc::now() - atraso;
        RegistroDeExecucao {
            id: id.into(),
            camada: todeolho_nucleo::Camada::Raw,
            iniciada_em: instante,
            atualizada_em: instante,
            concluida_em: None,
            fontes: Vec::new(),
        }
    }

    fn registro_concluido(id: &str, atraso: TimeDelta) -> RegistroDeExecucao {
        let instante = Utc::now() - atraso;
        RegistroDeExecucao {
            id: id.into(),
            camada: todeolho_nucleo::Camada::Raw,
            iniciada_em: instante,
            atualizada_em: instante,
            concluida_em: Some(instante + TimeDelta::seconds(10)),
            fontes: Vec::new(),
        }
    }

    #[tokio::test]
    async fn entrega_a_pagina_com_politica_de_conteudo() {
        let (status, corpo, politica) = obter(ConsultaFalsa(Ok(Vec::new())), "/").await;

        assert_eq!(status, StatusCode::OK);
        assert!(corpo.contains("Tô De Olho Aí"));
        assert!(politica.unwrap().contains("default-src 'none'"));
    }

    #[tokio::test]
    async fn api_retorna_envelope_paginado() {
        let consulta = ConsultaFalsa(Ok(vec![
            registro("viva", TimeDelta::seconds(5)),
            registro("parada", TimeDelta::minutes(10)),
        ]));

        let (status, corpo, _) = obter(consulta, "/api/execucoes").await;

        assert_eq!(status, StatusCode::OK);
        let resposta: serde_json::Value = serde_json::from_str(&corpo).unwrap();
        assert_eq!(resposta["total"], 2);
        assert_eq!(resposta["pagina"], 1);
        assert_eq!(resposta["dados"][0]["id"], "viva");
        assert_eq!(resposta["dados"][0]["situacao"], "em_andamento");
        assert_eq!(resposta["dados"][1]["situacao"], "interrompida");
        assert_eq!(resposta["ha_andamento"], true);
    }

    #[tokio::test]
    async fn api_filtra_por_situacao() {
        let consulta = ConsultaFalsa(Ok(vec![
            registro("viva", TimeDelta::seconds(5)),
            registro_concluido("pronta", TimeDelta::minutes(30)),
        ]));

        let (status, corpo, _) = obter(consulta, "/api/execucoes?situacao=sucesso").await;

        assert_eq!(status, StatusCode::OK);
        let resposta: serde_json::Value = serde_json::from_str(&corpo).unwrap();
        assert_eq!(resposta["total"], 1);
        assert_eq!(resposta["dados"][0]["id"], "pronta");
        assert_eq!(resposta["ha_andamento"], true);
    }

    #[tokio::test]
    async fn api_pagina_resultados() {
        let consulta = ConsultaFalsa(Ok(vec![
            registro_concluido("a", TimeDelta::seconds(10)),
            registro_concluido("b", TimeDelta::seconds(20)),
            registro_concluido("c", TimeDelta::seconds(30)),
        ]));

        let (_, corpo, _) = obter(consulta, "/api/execucoes?por_pagina=2&pagina=2").await;

        let resposta: serde_json::Value = serde_json::from_str(&corpo).unwrap();
        assert_eq!(resposta["total"], 3);
        assert_eq!(resposta["pagina"], 2);
        assert_eq!(resposta["total_paginas"], 2);
        assert_eq!(resposta["dados"].as_array().unwrap().len(), 1);
        assert_eq!(resposta["dados"][0]["id"], "c");
        assert_eq!(resposta["ha_andamento"], false);
    }

    #[tokio::test]
    async fn api_rejeita_situacao_invalida() {
        let consulta = ConsultaFalsa(Ok(vec![registro_concluido("a", TimeDelta::seconds(10))]));

        let (status, corpo, _) = obter(consulta, "/api/execucoes?situacao=inexistente").await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(corpo.contains("inválido"));
    }

    #[tokio::test]
    async fn falha_no_armazenamento_nao_expoe_detalhes_internos() {
        let (status, corpo, _) = obter(ConsultaFalsa(Err(())), "/api/execucoes").await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(!corpo.contains("segredo"));
    }
}
