use std::error::Error;
use std::io;
use std::time::Duration;

use async_trait::async_trait;
use futures::TryStreamExt;
use reqwest::{RequestBuilder, Response, StatusCode, header};
use todeolho_nucleo::{
    ClienteHttp, DownloadHttp, ErroDeIngestao, InformacoesDoArquivo, RespostaHttp,
};
use tokio_util::io::StreamReader;
use tracing::warn;

const AGENTE: &str = concat!(
    "todeolhoai/",
    env!("CARGO_PKG_VERSION"),
    " (ingestão de dados públicos)"
);
const TEMPO_DE_CONEXAO: Duration = Duration::from_secs(30);
/// Sem receber nenhum byte por este tempo, a conexão é considerada perdida.
const TEMPO_SEM_DADOS: Duration = Duration::from_secs(120);
const TEMPO_LIMITE: Duration = Duration::from_secs(90);
/// Generoso para o maior arquivo (cerca de 860 MB) em uma conexão lenta.
const TEMPO_LIMITE_DE_DOWNLOAD: Duration = Duration::from_secs(6 * 60 * 60);

#[derive(Debug, Clone, Copy)]
pub struct PoliticaDeRetentativas {
    pub tentativas: u32,
    pub espera_inicial: Duration,
}

impl Default for PoliticaDeRetentativas {
    fn default() -> Self {
        Self {
            tentativas: 5,
            espera_inicial: Duration::from_secs(2),
        }
    }
}

impl PoliticaDeRetentativas {
    /// Espera antes da nova tentativa: dobra a cada falha.
    fn espera(&self, falhas: u32) -> Duration {
        self.espera_inicial
            .saturating_mul(2u32.saturating_pow(falhas.saturating_sub(1)))
    }
}

/// APIs públicas oscilam (a da Câmara responde 504 com frequência), então falhas
/// transitórias são repetidas com espera crescente em vez de derrubar a tarefa.
pub struct ClienteHttpComRetentativas {
    cliente: reqwest::Client,
    politica: PoliticaDeRetentativas,
}

impl ClienteHttpComRetentativas {
    pub fn novo(politica: PoliticaDeRetentativas) -> Result<Self, ErroDeIngestao> {
        let cliente = reqwest::Client::builder()
            .user_agent(AGENTE)
            .connect_timeout(TEMPO_DE_CONEXAO)
            .read_timeout(TEMPO_SEM_DADOS)
            .build()
            .map_err(|erro| ErroDeIngestao::Http {
                url: String::new(),
                detalhe: erro.to_string(),
            })?;
        Ok(Self { cliente, politica })
    }

    async fn tentar_obter(&self, url: &str, aceita: &str) -> Result<RespostaHttp, Falha> {
        let resposta = enviar(
            self.cliente
                .get(url)
                .timeout(TEMPO_LIMITE)
                .header(header::ACCEPT, aceita),
        )
        .await?;

        let tipo_de_conteudo = tipo_de_conteudo(&resposta);
        let corpo = resposta
            .bytes()
            .await
            .map_err(|erro| Falha::transitoria(descrever(erro)))?;

        Ok(RespostaHttp {
            tipo_de_conteudo,
            corpo: corpo.to_vec(),
        })
    }

    /// Repete a operação enquanto a falha for transitória.
    async fn com_retentativas<T, F>(
        &self,
        url: &str,
        mut operacao: impl FnMut() -> F,
    ) -> Result<T, ErroDeIngestao>
    where
        F: Future<Output = Result<T, Falha>>,
    {
        let mut falhas = 0;
        loop {
            let falha = match operacao().await {
                Ok(resultado) => return Ok(resultado),
                Err(falha) => falha,
            };

            falhas += 1;
            if !falha.transitoria || falhas >= self.politica.tentativas {
                return Err(ErroDeIngestao::Http {
                    url: url.to_owned(),
                    detalhe: format!("{} (após {falhas} tentativa(s))", falha.detalhe),
                });
            }

            let espera = self.politica.espera(falhas);
            warn!(%url, detalhe = %falha.detalhe, ?espera, "falha transitória; tentando novamente");
            tokio::time::sleep(espera).await;
        }
    }
}

#[async_trait]
impl ClienteHttp for ClienteHttpComRetentativas {
    async fn obter(&self, url: &str, aceita: &str) -> Result<RespostaHttp, ErroDeIngestao> {
        self.com_retentativas(url, || self.tentar_obter(url, aceita))
            .await
    }

    async fn informacoes(&self, url: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
        let resposta = self
            .com_retentativas(url, || enviar(self.cliente.head(url).timeout(TEMPO_LIMITE)))
            .await;

        // Sem essas informações, a estimativa fica desconhecida e o arquivo é baixado de novo.
        Ok(match resposta {
            Ok(resposta) => InformacoesDoArquivo {
                versao: etag(&resposta),
                tamanho: cabecalho(&resposta, header::CONTENT_LENGTH)
                    .and_then(|tamanho| tamanho.parse().ok()),
            },
            Err(erro) => {
                warn!(%erro, "não foi possível consultar as informações do arquivo");
                InformacoesDoArquivo::default()
            }
        })
    }

    async fn baixar(&self, url: &str) -> Result<DownloadHttp, ErroDeIngestao> {
        let resposta = self
            .com_retentativas(url, || {
                enviar(self.cliente.get(url).timeout(TEMPO_LIMITE_DE_DOWNLOAD))
            })
            .await?;

        Ok(DownloadHttp {
            tipo_de_conteudo: tipo_de_conteudo(&resposta),
            versao: etag(&resposta),
            fluxo: Box::new(StreamReader::new(
                resposta
                    .bytes_stream()
                    .map_err(|erro| io::Error::other(descrever(erro))),
            )),
        })
    }
}

async fn enviar(requisicao: RequestBuilder) -> Result<Response, Falha> {
    let resposta = requisicao
        .send()
        .await
        .map_err(|erro| Falha::transitoria(descrever(erro)))?;

    let status = resposta.status();
    if status.is_success() {
        return Ok(resposta);
    }

    let detalhe = format!("status {status}");
    Err(if eh_transitorio(status) {
        Falha::transitoria(detalhe)
    } else {
        Falha::definitiva(detalhe)
    })
}

fn tipo_de_conteudo(resposta: &Response) -> String {
    cabecalho(resposta, header::CONTENT_TYPE).unwrap_or_else(|| "application/octet-stream".into())
}

fn etag(resposta: &Response) -> Option<String> {
    cabecalho(resposta, header::ETAG)
}

fn cabecalho(resposta: &Response, nome: header::HeaderName) -> Option<String> {
    resposta
        .headers()
        .get(nome)
        .and_then(|valor| valor.to_str().ok())
        .map(str::to_owned)
}

struct Falha {
    detalhe: String,
    transitoria: bool,
}

impl Falha {
    fn transitoria(detalhe: String) -> Self {
        Self {
            detalhe,
            transitoria: true,
        }
    }

    fn definitiva(detalhe: String) -> Self {
        Self {
            detalhe,
            transitoria: false,
        }
    }
}

/// Inclui a causa de origem, sem a URL, que já consta na mensagem final.
fn descrever(erro: reqwest::Error) -> String {
    let erro = erro.without_url();
    let mut descricao = erro.to_string();
    let mut causa = Error::source(&erro);
    while let Some(atual) = causa {
        descricao.push_str(": ");
        descricao.push_str(&atual.to_string());
        causa = atual.source();
    }
    descricao
}

fn eh_transitorio(status: StatusCode) -> bool {
    status.is_server_error()
        || status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::REQUEST_TIMEOUT
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn somente_erros_temporarios_sao_repetidos() {
        for status in [
            StatusCode::GATEWAY_TIMEOUT,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::REQUEST_TIMEOUT,
        ] {
            assert!(eh_transitorio(status), "{status}");
        }
        for status in [
            StatusCode::NOT_FOUND,
            StatusCode::BAD_REQUEST,
            StatusCode::FORBIDDEN,
        ] {
            assert!(!eh_transitorio(status), "{status}");
        }
    }

    #[test]
    fn espera_dobra_a_cada_falha() {
        let politica = PoliticaDeRetentativas::default();
        let esperas: Vec<_> = (1..=4)
            .map(|falhas| politica.espera(falhas).as_secs())
            .collect();
        assert_eq!(esperas, vec![2, 4, 8, 16]);
    }
}
