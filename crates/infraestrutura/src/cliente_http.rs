use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use async_trait::async_trait;
use futures::TryStreamExt;
use reqwest::{RequestBuilder, Response, StatusCode, header};
use todeolho_nucleo::{
    ClienteHttp, DownloadHttp, ErroDeIngestao, InformacoesDoArquivo, PerfilHttp,
    RespostaCondicional, RespostaHttp,
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

/// Quantas vezes tentar uma requisição transitória e quanto esperar no início.
#[derive(Debug, Clone, Copy)]
pub struct PoliticaDeRetentativas {
    /// Número **total** de tentativas, já contando a primeira (ex.: `5` = a
    /// requisição inicial mais até 4 repetições).
    pub tentativas: u32,
    /// Espera antes da primeira retentativa; dobra a cada falha.
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
    host_da_credencial: Option<String>,
}

struct EstadoDoHost {
    proxima: tokio::time::Instant,
    intervalo_entre_requisicoes: Duration,
}

static LIMITES_POR_HOST: OnceLock<Mutex<HashMap<String, EstadoDoHost>>> = OnceLock::new();

impl ClienteHttpComRetentativas {
    /// Cria o cliente com a política de retentativas dada.
    pub fn novo(politica: PoliticaDeRetentativas) -> Result<Self, ErroDeIngestao> {
        Self::com_perfil(politica, PerfilHttp::default(), None)
    }

    /// Configura uma fonte. `segredo` vem do ambiente e nunca é incluído em erros ou logs.
    pub fn com_perfil(
        politica: PoliticaDeRetentativas,
        perfil: PerfilHttp,
        segredo: Option<&str>,
    ) -> Result<Self, ErroDeIngestao> {
        let mut construtor = reqwest::Client::builder()
            .user_agent(AGENTE)
            .connect_timeout(TEMPO_DE_CONEXAO)
            .read_timeout(TEMPO_SEM_DADOS);

        let host_da_credencial = if let Some(credencial) = perfil.credencial {
            let valor = segredo
                .filter(|valor| !valor.trim().is_empty())
                .ok_or_else(|| {
                    erro_de_configuracao(format!(
                        "variável de ambiente ausente ou vazia: {}",
                        credencial.variavel
                    ))
                })?;
            let nome = header::HeaderName::from_bytes(credencial.cabecalho.as_bytes())
                .map_err(|_| erro_de_configuracao("nome de cabeçalho HTTP inválido".into()))?;
            let mut valor =
                header::HeaderValue::from_str(&format!("{}{}", credencial.prefixo, valor))
                    .map_err(|_| {
                        erro_de_configuracao("valor de credencial HTTP inválido".into())
                    })?;
            valor.set_sensitive(true);
            let mut cabecalhos = header::HeaderMap::new();
            cabecalhos.insert(nome, valor);
            // Mesmo um redirect válido nunca transporta o segredo para outro host.
            construtor = construtor
                .default_headers(cabecalhos)
                .redirect(reqwest::redirect::Policy::none());
            Some(credencial.host.to_ascii_lowercase())
        } else {
            None
        };

        let limite = perfil
            .limite
            .map(|limite| {
                if limite.requisicoes == 0 || limite.intervalo.is_zero() || limite.host.is_empty() {
                    return Err(erro_de_configuracao("limite HTTP inválido".into()));
                }
                Ok((
                    limite.host.to_ascii_lowercase(),
                    (limite.intervalo / limite.requisicoes).max(Duration::from_nanos(1)),
                ))
            })
            .transpose()?;

        let cliente = construtor.build().map_err(|erro| ErroDeIngestao::Http {
            url: String::new(),
            detalhe: erro.to_string(),
        })?;
        if let Some((host, intervalo)) = limite {
            registrar_limite(&host, intervalo);
        }
        Ok(Self {
            cliente,
            politica,
            host_da_credencial,
        })
    }

    async fn enviar(&self, requisicao: RequestBuilder) -> Result<Response, Falha> {
        classificar_resposta(self.enviar_bruto(requisicao).await)
    }

    async fn enviar_bruto(&self, requisicao: RequestBuilder) -> Result<Response, Falha> {
        let requisicao = requisicao
            .build()
            .map_err(|erro| Falha::definitiva(descrever(erro)))?;
        let host = requisicao
            .url()
            .host_str()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if let Some(autorizado) = &self.host_da_credencial
            && &host != autorizado
        {
            return Err(Falha::definitiva(format!(
                "host HTTP não autorizado para a credencial: {host}"
            )));
        }
        aguardar_limite(&host).await;
        self.cliente
            .execute(requisicao)
            .await
            .map_err(|erro| Falha::transitoria(descrever(erro)))
    }

    async fn tentar_obter(&self, url: &str, aceita: &str) -> Result<RespostaHttp, Falha> {
        let resposta = self
            .enviar(
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

    async fn tentar_condicional(
        &self,
        url: &str,
        aceita: &str,
        validador: Option<&str>,
    ) -> Result<RespostaCondicional, Falha> {
        let mut requisicao = self
            .cliente
            .get(url)
            .timeout(TEMPO_LIMITE)
            .header(header::ACCEPT, aceita);
        if let Some(validador) = validador {
            requisicao = requisicao.header(header::IF_NONE_MATCH, validador);
        }

        let resposta = self.enviar_bruto(requisicao).await?;

        let status = resposta.status();
        if status == StatusCode::NOT_MODIFIED {
            return Ok(RespostaCondicional::NaoModificado);
        }
        verificar_status(status)?;

        let tipo_de_conteudo = tipo_de_conteudo(&resposta);
        let versao = etag(&resposta);
        let corpo = resposta
            .bytes()
            .await
            .map_err(|erro| Falha::transitoria(descrever(erro)))?;

        Ok(RespostaCondicional::Modificado {
            tipo_de_conteudo,
            versao,
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
            if let Some(status) = falha.status_de_autenticacao {
                return Err(ErroDeIngestao::Autenticacao {
                    url: url.to_owned(),
                    status,
                });
            }
            if falha.nao_encontrado {
                return Err(ErroDeIngestao::NaoEncontrado {
                    url: url.to_owned(),
                });
            }
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

    async fn obter_condicional(
        &self,
        url: &str,
        aceita: &str,
        validador: Option<&str>,
    ) -> Result<RespostaCondicional, ErroDeIngestao> {
        self.com_retentativas(url, || self.tentar_condicional(url, aceita, validador))
            .await
    }

    async fn informacoes(&self, url: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
        let resposta = self
            .com_retentativas(url, || {
                self.enviar(self.cliente.head(url).timeout(TEMPO_LIMITE))
            })
            .await;

        // Sem essas informações, a estimativa fica desconhecida e o arquivo é baixado de novo.
        Ok(match resposta {
            Ok(resposta) => InformacoesDoArquivo {
                versao: etag(&resposta),
                tamanho: cabecalho(&resposta, header::CONTENT_LENGTH)
                    .and_then(|tamanho| tamanho.parse().ok()),
            },
            Err(erro @ ErroDeIngestao::Autenticacao { .. }) => return Err(erro),
            Err(erro) => {
                warn!(%erro, "não foi possível consultar as informações do arquivo");
                InformacoesDoArquivo::default()
            }
        })
    }

    async fn baixar(&self, url: &str) -> Result<DownloadHttp, ErroDeIngestao> {
        self.baixar_a_partir_de(url, 0, None).await
    }

    async fn baixar_a_partir_de(
        &self,
        url: &str,
        inicio: u64,
        versao: Option<&str>,
    ) -> Result<DownloadHttp, ErroDeIngestao> {
        let resposta = self
            .com_retentativas(url, || {
                let mut requisicao = self.cliente.get(url).timeout(TEMPO_LIMITE_DE_DOWNLOAD);
                if inicio > 0 {
                    // Retomada: pede do offset em diante e só aceita se o arquivo
                    // não mudou (If-Range com o ETag da primeira resposta). If-Range
                    // só aceita validador FORTE; um ETag fraco (`W/"..."`) faria a
                    // origem devolver o arquivo inteiro (200), impedindo a retomada.
                    requisicao = requisicao.header(header::RANGE, format!("bytes={inicio}-"));
                    if let Some(validador) = versao
                        && !validador.trim_start().starts_with("W/")
                    {
                        requisicao = requisicao.header(header::IF_RANGE, validador);
                    }
                }
                self.enviar(requisicao)
            })
            .await?;

        // `206` traz o início e o total no Content-Range; `200` (inclusive quando
        // a origem ignora o Range) começa do zero, com o total no Content-Length.
        let (inicio_real, tamanho_total) = if resposta.status() == StatusCode::PARTIAL_CONTENT {
            faixa_de_conteudo(&resposta).unwrap_or((0, None))
        } else {
            (
                0,
                cabecalho(&resposta, header::CONTENT_LENGTH).and_then(|total| total.parse().ok()),
            )
        };

        Ok(DownloadHttp {
            tipo_de_conteudo: tipo_de_conteudo(&resposta),
            versao: etag(&resposta),
            inicio: inicio_real,
            tamanho_total,
            fluxo: Box::new(StreamReader::new(
                resposta
                    .bytes_stream()
                    .map_err(|erro| io::Error::other(descrever(erro))),
            )),
        })
    }
}

/// Lê o `Content-Range: bytes INICIO-FIM/TOTAL` de uma resposta `206` e devolve
/// `(inicio, total)`. `total` é `None` quando a origem manda `*`.
fn faixa_de_conteudo(resposta: &Response) -> Option<(u64, Option<u64>)> {
    let valor = cabecalho(resposta, header::CONTENT_RANGE)?;
    let resto = valor.strip_prefix("bytes ")?;
    let (faixa, total) = resto.split_once('/')?;
    let inicio = faixa.split('-').next()?.trim().parse().ok()?;
    Some((inicio, total.trim().parse().ok()))
}

fn classificar_resposta(resposta: Result<Response, Falha>) -> Result<Response, Falha> {
    let resposta = resposta?;
    verificar_status(resposta.status())?;
    Ok(resposta)
}

fn verificar_status(status: StatusCode) -> Result<(), Falha> {
    if status.is_success() {
        return Ok(());
    }
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(Falha::autenticacao(status.as_u16()));
    }
    if status == StatusCode::NOT_FOUND {
        return Err(Falha::nao_encontrado());
    }
    let detalhe = format!("status {status}");
    Err(if eh_transitorio(status) {
        Falha::transitoria(detalhe)
    } else {
        Falha::definitiva(detalhe)
    })
}

fn erro_de_configuracao(detalhe: String) -> ErroDeIngestao {
    ErroDeIngestao::Http {
        url: String::new(),
        detalhe,
    }
}

async fn aguardar_limite(host: &str) {
    if let Some(instante) = reservar_limite(host) {
        tokio::time::sleep_until(instante).await;
    }
}

fn registrar_limite(host: &str, intervalo: Duration) {
    let mut limites = LIMITES_POR_HOST
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let agora = tokio::time::Instant::now();
    let estado = limites.entry(host.to_owned()).or_insert(EstadoDoHost {
        proxima: agora,
        intervalo_entre_requisicoes: intervalo,
    });
    estado.intervalo_entre_requisicoes = estado.intervalo_entre_requisicoes.max(intervalo);
}

fn reservar_limite(host: &str) -> Option<tokio::time::Instant> {
    let mut limites = LIMITES_POR_HOST
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let estado = limites.get_mut(host)?;
    let instante = estado.proxima.max(tokio::time::Instant::now());
    estado.proxima = instante + estado.intervalo_entre_requisicoes;
    Some(instante)
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
    status_de_autenticacao: Option<u16>,
    nao_encontrado: bool,
}

impl Falha {
    fn transitoria(detalhe: String) -> Self {
        Self {
            detalhe,
            transitoria: true,
            status_de_autenticacao: None,
            nao_encontrado: false,
        }
    }

    fn definitiva(detalhe: String) -> Self {
        Self {
            detalhe,
            transitoria: false,
            status_de_autenticacao: None,
            nao_encontrado: false,
        }
    }

    fn autenticacao(status: u16) -> Self {
        Self {
            detalhe: format!("status {status}"),
            transitoria: false,
            status_de_autenticacao: Some(status),
            nao_encontrado: false,
        }
    }

    fn nao_encontrado() -> Self {
        Self {
            detalhe: "status 404 Not Found".into(),
            transitoria: false,
            status_de_autenticacao: None,
            nao_encontrado: true,
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
    use todeolho_nucleo::{CredencialHttp, LimiteHttp};

    const PERFIL_DE_TESTE: PerfilHttp = PerfilHttp {
        credencial: Some(CredencialHttp {
            host: "127.0.0.1",
            variavel: "TESTE_TOKEN",
            cabecalho: "chave-api-dados",
            prefixo: "",
        }),
        limite: None,
    };

    #[test]
    fn credencial_ausente_ou_invalida_nao_expoe_segredo() {
        let politica = PoliticaDeRetentativas::default();
        let erro = ClienteHttpComRetentativas::com_perfil(politica, PERFIL_DE_TESTE, None)
            .err()
            .unwrap()
            .to_string();
        assert!(erro.contains("TESTE_TOKEN"));

        let erro = ClienteHttpComRetentativas::com_perfil(
            politica,
            PERFIL_DE_TESTE,
            Some("segredo\nquebrado"),
        )
        .err()
        .unwrap()
        .to_string();
        assert!(!erro.contains("segredo"));
    }

    #[tokio::test]
    async fn host_diferente_e_recusado_antes_de_enviar() {
        let cliente = ClienteHttpComRetentativas::com_perfil(
            PoliticaDeRetentativas::default(),
            PERFIL_DE_TESTE,
            Some("segredo"),
        )
        .unwrap();
        let erro = cliente
            .obter("https://example.org/recurso", "application/json")
            .await
            .unwrap_err();
        assert!(erro.to_string().contains("host HTTP não autorizado"));
        assert!(!erro.to_string().contains("segredo"));
    }

    #[tokio::test]
    async fn content_length_maior_que_o_corpo_falha_no_adapter() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let ouvinte = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://127.0.0.1:{}/recurso",
            ouvinte.local_addr().unwrap().port()
        );
        let servidor = std::thread::spawn(move || {
            let (mut conexao, _) = ouvinte.accept().unwrap();
            let mut buf = [0u8; 2048];
            assert!(conexao.read(&mut buf).unwrap() > 0);
            conexao
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                      Content-Length: 100\r\nConnection: close\r\n\r\n{\"dados\":[1]}",
                )
                .unwrap();
        });
        let cliente = ClienteHttpComRetentativas::novo(PoliticaDeRetentativas {
            tentativas: 1,
            espera_inicial: Duration::ZERO,
        })
        .unwrap();

        let erro = cliente.obter(&url, "application/json").await.unwrap_err();

        assert!(erro.to_string().contains(&url));
        servidor.join().unwrap();
    }

    #[test]
    fn limite_e_compartilhado_por_host_e_respeita_o_mais_restritivo() {
        let host = "teste-limite-compartilhado.example";
        registrar_limite(host, Duration::from_millis(100));
        let primeira = reservar_limite(host).unwrap();
        registrar_limite(host, Duration::from_millis(200));
        let segunda = reservar_limite(host).unwrap();
        let terceira = reservar_limite(host).unwrap();
        assert!(segunda.duration_since(primeira) >= Duration::from_millis(100));
        assert!(terceira.duration_since(segunda) >= Duration::from_millis(200));
    }

    #[test]
    fn perfil_com_limite_invalido_e_recusado() {
        let perfil = PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "example.org",
                requisicoes: 0,
                intervalo: Duration::from_secs(1),
            }),
        };
        assert!(
            ClienteHttpComRetentativas::com_perfil(PoliticaDeRetentativas::default(), perfil, None)
                .is_err()
        );
    }

    #[tokio::test]
    async fn head_com_credencial_e_401_falha_sem_vazar_o_segredo() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let ouvinte = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://127.0.0.1:{}/arquivo",
            ouvinte.local_addr().unwrap().port()
        );
        let servidor = std::thread::spawn(move || {
            let (mut conexao, _) = ouvinte.accept().unwrap();
            let mut buf = [0u8; 2048];
            let lidos = conexao.read(&mut buf).unwrap();
            conexao
                .write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
            String::from_utf8_lossy(&buf[..lidos]).into_owned()
        });
        let cliente = ClienteHttpComRetentativas::com_perfil(
            PoliticaDeRetentativas::default(),
            PERFIL_DE_TESTE,
            Some("token-secreto"),
        )
        .unwrap();
        let erro = cliente.informacoes(&url).await.unwrap_err();
        assert!(matches!(
            erro,
            ErroDeIngestao::Autenticacao { status: 401, .. }
        ));
        assert!(!erro.to_string().contains("token-secreto"));
        let requisicao = servidor.join().unwrap().to_ascii_lowercase();
        assert!(requisicao.starts_with("head /arquivo "));
        assert!(requisicao.contains("chave-api-dados: token-secreto"));
    }

    #[tokio::test]
    async fn download_404_devolve_ausencia_tipificada() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let ouvinte = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://127.0.0.1:{}/arquivo",
            ouvinte.local_addr().unwrap().port()
        );
        let servidor = std::thread::spawn(move || {
            let (mut conexao, _) = ouvinte.accept().unwrap();
            let mut buf = [0u8; 2048];
            let lidos = conexao.read(&mut buf).unwrap();
            assert!(lidos > 0);
            conexao
                .write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
        });
        let cliente = ClienteHttpComRetentativas::novo(PoliticaDeRetentativas::default()).unwrap();
        assert!(matches!(
            cliente.baixar(&url).await,
            Err(ErroDeIngestao::NaoEncontrado { url: ausente }) if ausente == url
        ));
        servidor.join().unwrap();
    }

    /// Exercita o adaptador real: pede uma retomada e confere que ele envia
    /// `Range`/`If-Range` e lê o `Content-Range` de um `206`, entregando o resto.
    #[tokio::test]
    async fn baixar_a_partir_de_envia_range_e_le_content_range() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        use tokio::io::AsyncReadExt;

        const CORPO: &[u8] = b"0123456789abcdefghij"; // 20 bytes
        let total = CORPO.len();
        let inicio = 5usize;

        let ouvinte = TcpListener::bind("127.0.0.1:0").unwrap();
        let porta = ouvinte.local_addr().unwrap().port();
        let (enviar_requisicao, receber_requisicao) = std::sync::mpsc::channel::<String>();

        let servidor = std::thread::spawn(move || {
            let (mut conexao, _) = ouvinte.accept().unwrap();
            let mut buf = [0u8; 2048];
            let lidos = conexao.read(&mut buf).unwrap();
            enviar_requisicao
                .send(String::from_utf8_lossy(&buf[..lidos]).into_owned())
                .unwrap();

            let resto = &CORPO[inicio..];
            let cabecalho = format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Type: application/zip\r\n\
                 ETag: \"etag\"\r\nContent-Range: bytes {inicio}-{fim}/{total}\r\n\
                 Content-Length: {len}\r\nConnection: close\r\n\r\n",
                fim = total - 1,
                len = resto.len(),
            );
            conexao.write_all(cabecalho.as_bytes()).unwrap();
            conexao.write_all(resto).unwrap();
            conexao.flush().unwrap();
        });

        let cliente = ClienteHttpComRetentativas::novo(PoliticaDeRetentativas::default()).unwrap();
        let url = format!("http://127.0.0.1:{porta}/arquivo.zip");
        let download = cliente
            .baixar_a_partir_de(&url, inicio as u64, Some("\"etag\""))
            .await
            .unwrap();

        // O adaptador leu o Content-Range: começa em `inicio`, com o total certo.
        assert_eq!(download.inicio, inicio as u64);
        assert_eq!(download.tamanho_total, Some(total as u64));
        assert_eq!(download.versao.as_deref(), Some("\"etag\""));

        let mut corpo = Vec::new();
        let mut fluxo = download.fluxo;
        fluxo.read_to_end(&mut corpo).await.unwrap();
        assert_eq!(corpo, &CORPO[inicio..]);

        // E enviou os cabeçalhos de retomada (nome do cabeçalho é case-insensitive).
        let requisicao = receber_requisicao.recv().unwrap().to_lowercase();
        assert!(requisicao.contains("range: bytes=5-"), "{requisicao}");
        assert!(requisicao.contains("if-range: \"etag\""), "{requisicao}");

        servidor.join().unwrap();
    }

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
    fn status_403_e_falha_de_autenticacao_definitiva() {
        let falha = verificar_status(StatusCode::FORBIDDEN).err().unwrap();
        assert_eq!(falha.status_de_autenticacao, Some(403));
        assert!(!falha.transitoria);
    }

    #[test]
    fn status_404_e_ausencia_definitiva() {
        let falha = verificar_status(StatusCode::NOT_FOUND).err().unwrap();
        assert!(falha.nao_encontrado);
        assert!(!falha.transitoria);
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
