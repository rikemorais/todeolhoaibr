//! Painel web, somente leitura, das execuções de ingestão (ver `docs/adr/0008`).

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{Html, IntoResponse, Json, Response};
use axum::routing::get;
use axum::{Router, middleware};
use chrono::Utc;
use serde::Serialize;
use todeolho_nucleo::{ConsultaDeExecucoes, RegistroDeExecucao, SituacaoDaExecucao};
use tracing::{error, info};

const LIMITE_DE_EXECUCOES: usize = 50;
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
struct RespostaDeErro {
    erro: &'static str,
}

/// Monta as rotas do painel.
pub fn rotas(consulta: Arc<dyn ConsultaDeExecucoes>) -> Router {
    Router::new()
        .route("/", get(pagina))
        .route("/api/execucoes", get(execucoes))
        .layer(middleware::map_response(cabecalhos_de_seguranca))
        .with_state(consulta)
}

/// Serve o painel até o processo ser encerrado.
pub async fn servir(
    endereco: SocketAddr,
    consulta: Arc<dyn ConsultaDeExecucoes>,
) -> std::io::Result<()> {
    let ouvinte = tokio::net::TcpListener::bind(endereco).await?;
    info!(%endereco, "painel disponível");
    axum::serve(ouvinte, rotas(consulta)).await
}

async fn pagina() -> Html<&'static str> {
    Html(PAGINA)
}

async fn execucoes(State(consulta): State<Arc<dyn ConsultaDeExecucoes>>) -> Response {
    match consulta.recentes(LIMITE_DE_EXECUCOES).await {
        Ok(registros) => {
            let agora = Utc::now();
            let visiveis: Vec<ExecucaoVisivel> = registros
                .into_iter()
                .map(|registro| ExecucaoVisivel {
                    situacao: registro.situacao(agora),
                    registro,
                })
                .collect();
            Json(visiveis).into_response()
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
            iniciada_em: instante,
            atualizada_em: instante,
            concluida_em: None,
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
    async fn api_calcula_a_situacao_de_cada_execucao() {
        let consulta = ConsultaFalsa(Ok(vec![
            registro("viva", TimeDelta::seconds(5)),
            registro("parada", TimeDelta::minutes(10)),
        ]));

        let (status, corpo, _) = obter(consulta, "/api/execucoes").await;

        assert_eq!(status, StatusCode::OK);
        let execucoes: serde_json::Value = serde_json::from_str(&corpo).unwrap();
        assert_eq!(execucoes[0]["id"], "viva");
        assert_eq!(execucoes[0]["situacao"], "em_andamento");
        assert_eq!(execucoes[1]["situacao"], "interrompida");
    }

    #[tokio::test]
    async fn falha_no_armazenamento_nao_expoe_detalhes_internos() {
        let (status, corpo, _) = obter(ConsultaFalsa(Err(())), "/api/execucoes").await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(!corpo.contains("segredo"));
    }
}
