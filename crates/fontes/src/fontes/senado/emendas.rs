//! Snapshot de pagamentos por emenda e favorecido publicado no SIGA Brasil.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use futures::{SinkExt, StreamExt};
use reqwest::header::{COOKIE, LOCATION, SET_COOKIE, USER_AGENT};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use todeolho_nucleo::{
    ContextoDeExecucao, ErroDeIngestao, FonteDeDados, LocalizacaoRaw, ManifestoRaw, ObjetoRaw,
    ParteRaw, PrecisaoDaEstimativa, ResumoDaTarefa, Tarefa, TarefaPlanejada, UltimaVersao,
    manifesto_vigente, publicar_ponteiro,
};
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest, http::HeaderValue};
use url::Url;

const NOME_FONTE: &str = "emendas-siga-brasil";
const NOME_TAREFA: &str = "favorecidos-pagamento";
const RECURSO: &str = "favorecidos-pagamento";
const HOST: &str = "www9qs.senado.leg.br";
const APP_ID: &str = "19601816-8b83-44f2-af2f-88d75dbc9cd1";
const OBJETO_ID: &str = "ada071e1-1a38-436c-87a3-09a971b24b08";
const URL_PAINEL: &str = "https://www9qs.senado.leg.br/extensions/Siga_Brasil_Emendas/Siga_Brasil_Emendas.html?hidemenu=true";
const USER_AGENT_PROJETO: &str =
    "todeolhoaibr/0.97 (dados abertos; https://github.com/rikemorais/todeolhoaibr)";
const TIPO_XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const LIMITE_REDIRECIONAMENTOS: usize = 5;
const LIMITE_XLSX_BYTES: u64 = 128 * 1024 * 1024;
const TIMEOUT_ENGINE: Duration = Duration::from_secs(180);

/// Captura anônima da tabela de pagamentos por emenda e favorecido municipal do SIGA Brasil.
///
/// O Qlik emite uma sessão anônima temporária, exporta a tabela inteira em XLSX e mantém o
/// arquivo exportado por tempo limitado. A captura não precisa de token pessoal.
pub struct FonteSenadoEmendas;

impl FonteDeDados for FonteSenadoEmendas {
    fn nome(&self) -> &str {
        NOME_FONTE
    }

    fn descricao(&self) -> &str {
        "SIGA Brasil - pagamentos por emenda e favorecido (snapshot XLSX)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        vec![TarefaPlanejada::nova(TarefaSigaBrasil::nova())]
    }
}

struct TarefaSigaBrasil;

impl TarefaSigaBrasil {
    fn nova() -> Self {
        Self
    }
}

struct SessaoAnonima {
    cookies: BTreeMap<String, String>,
    url_painel: Url,
}

impl SessaoAnonima {
    fn cabecalho_cookie(&self) -> String {
        self.cookies
            .iter()
            .map(|(nome, valor)| format!("{nome}={valor}"))
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn guardar_cookies(&mut self, headers: &reqwest::header::HeaderMap) {
        for set_cookie in headers.get_all(SET_COOKIE) {
            let Ok(set_cookie) = set_cookie.to_str() else {
                continue;
            };
            let Some((nome, valor)) = set_cookie
                .split(';')
                .next()
                .and_then(|par| par.split_once('='))
            else {
                continue;
            };
            self.cookies
                .insert(nome.trim().to_owned(), valor.trim().to_owned());
        }
    }
}

fn erro_http(detalhe: impl Into<String>) -> ErroDeIngestao {
    ErroDeIngestao::Http {
        url: URL_PAINEL.to_owned(),
        detalhe: detalhe.into(),
    }
}

async fn abrir_sessao(cliente: &reqwest::Client) -> Result<SessaoAnonima, ErroDeIngestao> {
    let mut sessao = SessaoAnonima {
        cookies: BTreeMap::new(),
        url_painel: Url::parse(URL_PAINEL).expect("URL constante válida"),
    };

    for etapa in 0..=LIMITE_REDIRECIONAMENTOS {
        let mut requisicao = cliente.get(sessao.url_painel.clone());
        let cookie = sessao.cabecalho_cookie();
        if !cookie.is_empty() {
            requisicao = requisicao.header(COOKIE, cookie);
        }
        let resposta = requisicao
            .send()
            .await
            .map_err(|_| erro_http("falha ao abrir o painel público do SIGA Brasil"))?;
        sessao.guardar_cookies(resposta.headers());

        if resposta.status().is_redirection() {
            if etapa == LIMITE_REDIRECIONAMENTOS {
                return Err(erro_http("o painel excedeu o limite de redirecionamentos"));
            }
            let destino = resposta
                .headers()
                .get(LOCATION)
                .and_then(|valor| valor.to_str().ok())
                .ok_or_else(|| erro_http("redirecionamento do painel sem destino"))?;
            let destino = sessao
                .url_painel
                .join(destino)
                .map_err(|_| erro_http("destino de redirecionamento inválido"))?;
            if destino.scheme() != "https" || destino.host_str() != Some(HOST) {
                return Err(erro_http(
                    "redirecionamento do painel saiu do host público do Senado",
                ));
            }
            sessao.url_painel = destino;
            continue;
        }
        if !resposta.status().is_success() {
            return Err(erro_http(format!(
                "o painel recusou a sessão anônima (HTTP {})",
                resposta.status()
            )));
        }
        if sessao.cookies.is_empty() {
            return Err(erro_http("o painel não emitiu cookie de sessão anônima"));
        }
        return Ok(sessao);
    }

    Err(erro_http(
        "não foi possível abrir a sessão anônima do SIGA Brasil",
    ))
}

async fn obter_token_csrf(
    cliente: &reqwest::Client,
    sessao: &SessaoAnonima,
) -> Result<String, ErroDeIngestao> {
    let url = sessao
        .url_painel
        .join("/qps/csrftoken")
        .map_err(|_| erro_http("URL do endpoint CSRF inválida"))?;
    let resposta = cliente
        .get(url)
        .header(COOKIE, sessao.cabecalho_cookie())
        .send()
        .await
        .map_err(|_| erro_http("falha ao solicitar o token CSRF anônimo"))?;
    if resposta.status() != reqwest::StatusCode::NO_CONTENT {
        return Err(erro_http(format!(
            "endpoint CSRF respondeu HTTP {}",
            resposta.status()
        )));
    }
    resposta
        .headers()
        .get("qlik-csrf-token")
        .and_then(|valor| valor.to_str().ok())
        .filter(|valor| !valor.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| erro_http("endpoint CSRF não devolveu o cabeçalho qlik-csrf-token"))
}

async fn chamar_engine(
    socket: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    proximo_id: &mut u64,
    handle: i64,
    metodo: &str,
    parametros: Value,
) -> Result<Value, ErroDeIngestao> {
    *proximo_id += 1;
    let id = *proximo_id;
    let mensagem = json!({
        "jsonrpc": "2.0",
        "id": id,
        "handle": handle,
        "method": metodo,
        "params": parametros,
    });
    socket
        .send(Message::Text(mensagem.to_string().into()))
        .await
        .map_err(|_| erro_http("falha ao enviar uma chamada para o Engine Qlik"))?;

    loop {
        let mensagem = tokio::time::timeout(TIMEOUT_ENGINE, socket.next())
            .await
            .map_err(|_| erro_http("timeout aguardando resposta do Engine Qlik"))?;
        let Some(mensagem) = mensagem else {
            break;
        };
        match mensagem.map_err(|_| erro_http("conexão com o Engine Qlik interrompida"))? {
            Message::Text(texto) => {
                let resposta: Value = serde_json::from_str(texto.as_str())
                    .map_err(|_| erro_http("Engine Qlik devolveu JSON inválido"))?;
                if resposta.get("id").and_then(Value::as_u64) != Some(id) {
                    continue;
                }
                if resposta.get("error").is_some() {
                    return Err(erro_http(format!("Engine Qlik recusou o método {metodo}")));
                }
                return resposta.get("result").cloned().ok_or_else(|| {
                    erro_http(format!("resposta Qlik sem resultado para {metodo}"))
                });
            }
            Message::Ping(bytes) => socket
                .send(Message::Pong(bytes))
                .await
                .map_err(|_| erro_http("falha ao responder ping do Engine Qlik"))?,
            Message::Close(_) => return Err(erro_http("Engine Qlik encerrou a conexão")),
            Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
    Err(erro_http(
        "Engine Qlik encerrou a conexão antes da resposta",
    ))
}

async fn exportar_xlsx(
    cliente: &reqwest::Client,
    sessao: &SessaoAnonima,
) -> Result<Vec<u8>, ErroDeIngestao> {
    let token_csrf = obter_token_csrf(cliente, sessao).await?;
    let mut url_socket = sessao.url_painel.clone();
    url_socket
        .set_scheme("wss")
        .map_err(|_| erro_http("não foi possível definir WSS"))?;
    url_socket.set_path(&format!("/app/{APP_ID}"));
    url_socket.set_query(None);
    url_socket
        .query_pairs_mut()
        .append_pair("reloadUri", sessao.url_painel.as_str())
        .append_pair("qlik-csrf-token", &token_csrf);

    let mut requisicao = url_socket
        .as_str()
        .into_client_request()
        .map_err(|_| erro_http("URL de conexão com o Engine Qlik inválida"))?;
    let headers = requisicao.headers_mut();
    headers.insert(
        "origin",
        HeaderValue::from_static("https://www9qs.senado.leg.br"),
    );
    headers.insert(USER_AGENT, HeaderValue::from_static(USER_AGENT_PROJETO));
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&sessao.cabecalho_cookie())
            .map_err(|_| erro_http("cookie de sessão anônima inválido"))?,
    );
    let (mut socket, _) = tokio::time::timeout(
        Duration::from_secs(30),
        tokio_tungstenite::connect_async(requisicao),
    )
    .await
    .map_err(|_| erro_http("timeout no handshake do Engine Qlik"))?
    .map_err(|_| erro_http("falha no handshake do Engine Qlik"))?;

    let mut id = 0;
    let app = chamar_engine(&mut socket, &mut id, -1, "OpenDoc", json!([APP_ID])).await?;
    let handle_app = app
        .pointer("/qReturn/qHandle")
        .and_then(Value::as_i64)
        .ok_or_else(|| erro_http("Engine Qlik não abriu o app público"))?;
    let objeto = chamar_engine(
        &mut socket,
        &mut id,
        handle_app,
        "GetObject",
        json!([OBJETO_ID]),
    )
    .await?;
    let handle_objeto = objeto
        .pointer("/qReturn/qHandle")
        .and_then(Value::as_i64)
        .ok_or_else(|| erro_http("objeto de favorecidos não existe no app SIGA Brasil"))?;
    let exportacao = chamar_engine(
        &mut socket,
        &mut id,
        handle_objeto,
        "ExportData",
        json!(["OOXML", "/qHyperCubeDef"]),
    )
    .await?;
    if exportacao
        .get("qWarnings")
        .and_then(Value::as_array)
        .is_some_and(|avisos| !avisos.is_empty())
    {
        return Err(erro_http(
            "Qlik exportou a tabela com avisos de truncamento",
        ));
    }
    let url_exportacao = exportacao
        .get("qUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| erro_http("Engine Qlik não devolveu a URL temporária do XLSX"))?;
    let url_exportacao = sessao
        .url_painel
        .join(url_exportacao)
        .map_err(|_| erro_http("URL temporária de exportação do Qlik inválida"))?;
    if url_exportacao.scheme() != "https" || url_exportacao.host_str() != Some(HOST) {
        return Err(erro_http(
            "URL temporária de exportação saiu do host público do Senado",
        ));
    }
    let resposta = cliente
        .get(url_exportacao)
        .header(COOKIE, sessao.cabecalho_cookie())
        .send()
        .await
        .map_err(|_| erro_http("falha ao baixar o XLSX temporário do SIGA Brasil"))?;
    if !resposta.status().is_success() {
        return Err(erro_http(format!(
            "download temporário do SIGA Brasil respondeu HTTP {}",
            resposta.status()
        )));
    }
    let tamanho_esperado = resposta.content_length();
    if tamanho_esperado.is_some_and(|tamanho| tamanho > LIMITE_XLSX_BYTES) {
        return Err(erro_http("XLSX do SIGA Brasil excede o limite de 128 MiB"));
    }
    let tipo = resposta
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|valor| valor.to_str().ok())
        .unwrap_or_default();
    if !tipo.to_ascii_lowercase().contains("spreadsheetml") {
        return Err(erro_http("download do SIGA Brasil não retornou um XLSX"));
    }
    let mut stream = resposta.bytes_stream();
    let mut bytes =
        Vec::with_capacity(tamanho_esperado.unwrap_or_default().min(LIMITE_XLSX_BYTES) as usize);
    while let Some(lote) = stream.next().await {
        let lote = lote.map_err(|_| erro_http("corpo XLSX do SIGA Brasil incompleto"))?;
        if bytes.len() as u64 + lote.len() as u64 > LIMITE_XLSX_BYTES {
            return Err(erro_http("XLSX do SIGA Brasil excede o limite de 128 MiB"));
        }
        bytes.extend_from_slice(&lote);
    }
    if tamanho_esperado.is_some_and(|esperado| esperado != bytes.len() as u64) {
        return Err(erro_http("corpo XLSX do SIGA Brasil incompleto"));
    }
    if !bytes.starts_with(b"PK\x03\x04") {
        return Err(erro_http("arquivo do SIGA Brasil não tem assinatura XLSX"));
    }
    let _ = socket.close(None).await;
    Ok(bytes)
}

#[async_trait]
impl Tarefa for TarefaSigaBrasil {
    fn nome(&self) -> &str {
        NOME_TAREFA
    }

    async fn bytes_a_baixar(
        &self,
        _contexto: &ContextoDeExecucao,
        _precisao: PrecisaoDaEstimativa,
    ) -> Result<Option<u64>, ErroDeIngestao> {
        // O Qlik cria a URL temporária apenas após abrir uma sessão e exportar o snapshot.
        Ok(None)
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let cliente = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(180))
            .user_agent(USER_AGENT_PROJETO)
            .build()
            .map_err(|_| erro_http("falha ao preparar cliente HTTP do SIGA Brasil"))?;
        let sessao = abrir_sessao(&cliente).await?;
        let bytes = exportar_xlsx(&cliente, &sessao).await?;
        let hash = hex::encode(Sha256::digest(&bytes));

        if let Some(manifesto) =
            manifesto_vigente(contexto.armazenamento.as_ref(), NOME_FONTE, RECURSO).await?
            && manifesto
                .partes
                .iter()
                .any(|parte| parte.sha256 == hash && parte.bytes == bytes.len() as u64)
        {
            return Ok(ResumoDaTarefa {
                reaproveitada: true,
                ..ResumoDaTarefa::default()
            });
        }

        let local = LocalizacaoRaw::nova(
            NOME_FONTE,
            RECURSO,
            contexto.iniciada_em,
            &contexto.id_execucao,
        );
        let chave = local.parte(1, "xlsx");
        let tamanho = bytes.len() as u64;
        contexto
            .armazenamento
            .gravar(ObjetoRaw {
                chave: chave.clone(),
                conteudo: bytes,
                tipo_de_conteudo: TIPO_XLSX.to_owned(),
            })
            .await?;

        let manifesto = ManifestoRaw {
            fonte: NOME_FONTE.to_owned(),
            recurso: RECURSO.to_owned(),
            id_execucao: contexto.id_execucao.clone(),
            url_inicial: URL_PAINEL.to_owned(),
            iniciada_em: contexto.iniciada_em,
            concluida_em: Utc::now(),
            partes: vec![ParteRaw {
                chave,
                url: URL_PAINEL.to_owned(),
                bytes: tamanho,
                sha256: hash,
                tipo_de_conteudo: TIPO_XLSX.to_owned(),
                versao: None,
            }],
        };
        let bytes_manifesto =
            serde_json::to_vec_pretty(&manifesto).map_err(|erro| ErroDeIngestao::Manifesto {
                recurso: RECURSO.to_owned(),
                detalhe: erro.to_string(),
            })?;
        contexto
            .armazenamento
            .gravar(ObjetoRaw {
                chave: local.manifesto(),
                conteudo: bytes_manifesto,
                tipo_de_conteudo: "application/json".to_owned(),
            })
            .await?;
        publicar_ponteiro(
            contexto.armazenamento.as_ref(),
            NOME_FONTE,
            RECURSO,
            &UltimaVersao {
                versao: None,
                manifesto: local.manifesto(),
                iniciada_em: Some(contexto.iniciada_em),
            },
        )
        .await?;

        Ok(ResumoDaTarefa {
            partes: 1,
            bytes: tamanho,
            reaproveitada: false,
            motivo: None,
        })
    }
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::Utc;
    use todeolho_nucleo::{
        ArmazenamentoRaw, ClienteHttp, ContextoDeExecucao, DownloadHttp, ErroDeIngestao,
        InformacoesDoArquivo, ObjetoRaw, RespostaHttp,
    };
    use tokio::io::AsyncReadExt;

    use super::*;
    use todeolho_nucleo::FonteDeDados;

    #[derive(Default)]
    struct ArmazenamentoEmMemoria(Mutex<HashMap<String, Vec<u8>>>);

    #[async_trait]
    impl ArmazenamentoRaw for ArmazenamentoEmMemoria {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }

        async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            self.0.lock().unwrap().insert(objeto.chave, objeto.conteudo);
            Ok(())
        }

        async fn gravar_fluxo(
            &self,
            chave: &str,
            _tipo_de_conteudo: &str,
            fluxo: &mut (dyn tokio::io::AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            let mut bytes = Vec::new();
            fluxo
                .read_to_end(&mut bytes)
                .await
                .map_err(|erro| ErroDeIngestao::Armazenamento {
                    chave: chave.to_owned(),
                    detalhe: erro.to_string(),
                })?;
            self.0.lock().unwrap().insert(chave.to_owned(), bytes);
            Ok(())
        }

        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.0.lock().unwrap().get(chave).cloned())
        }
    }

    struct ClienteHttpInerte;

    #[async_trait]
    impl ClienteHttp for ClienteHttpInerte {
        async fn obter(&self, url: &str, _aceita: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            Err(ErroDeIngestao::Http {
                url: url.to_owned(),
                detalhe: "não usado neste teste".into(),
            })
        }

        async fn informacoes(&self, url: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            Err(ErroDeIngestao::Http {
                url: url.to_owned(),
                detalhe: "não usado neste teste".into(),
            })
        }

        async fn baixar(&self, url: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            Err(ErroDeIngestao::Http {
                url: url.to_owned(),
                detalhe: "não usado neste teste".into(),
            })
        }
    }

    #[test]
    fn fonte_publica_e_sob_demanda_com_uma_tarefa() {
        let fonte = FonteSenadoEmendas;
        let tarefas = fonte.criar_tarefas();

        assert_eq!(fonte.nome(), NOME_FONTE);
        assert!(fonte.sob_demanda());
        assert_eq!(tarefas.len(), 1);
        assert_eq!(tarefas[0].tarefa.nome(), NOME_TAREFA);
    }

    #[test]
    fn cookie_header_serializa_um_cookie_por_par() {
        let sessao = SessaoAnonima {
            cookies: BTreeMap::from([
                ("a".to_owned(), "1".to_owned()),
                ("b".to_owned(), "2".to_owned()),
            ]),
            url_painel: Url::parse(URL_PAINEL).unwrap(),
        };

        assert_eq!(sessao.cabecalho_cookie(), "a=1; b=2");
    }

    #[test]
    fn guarda_cookies_da_sessao_e_ignora_headers_sem_par_nome_valor() {
        let mut sessao = SessaoAnonima {
            cookies: BTreeMap::new(),
            url_painel: Url::parse(URL_PAINEL).unwrap(),
        };
        let mut headers = reqwest::header::HeaderMap::new();
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("sessao=abc123; Path=/"),
        );
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("preferencia=pt-BR; Path=/"),
        );
        headers.append(SET_COOKIE, HeaderValue::from_static("header-invalido"));

        sessao.guardar_cookies(&headers);

        assert_eq!(sessao.cookies.len(), 2);
        assert_eq!(
            sessao.cookies.get("sessao").map(String::as_str),
            Some("abc123")
        );
        assert_eq!(
            sessao.cookies.get("preferencia").map(String::as_str),
            Some("pt-BR")
        );
    }

    #[tokio::test]
    #[ignore = "integração opcional com o painel público do Senado; faz download do snapshot"]
    async fn captura_anônima_publica_o_xlsx_com_manifesto_sem_ticket() {
        let armazenamento = Arc::new(ArmazenamentoEmMemoria::default());
        let contexto = ContextoDeExecucao {
            id_execucao: "teste-siga".to_owned(),
            iniciada_em: Utc::now(),
            armazenamento: armazenamento.clone(),
            http: Arc::new(ClienteHttpInerte),
            progresso: None,
        };

        let resumo = TarefaSigaBrasil.executar(&contexto).await.unwrap();

        assert_eq!(resumo.partes, 1);
        assert!(resumo.bytes > 0);
        let objetos = armazenamento.0.lock().unwrap();
        let (chave_xlsx, bytes_xlsx) = objetos
            .iter()
            .find(|(chave, _)| chave.ends_with("parte-00001.xlsx"))
            .unwrap();
        assert!(bytes_xlsx.starts_with(b"PK\x03\x04"));
        assert!(
            objetos
                .keys()
                .any(|chave| chave.ends_with("_manifesto.json"))
        );
        assert!(
            objetos
                .keys()
                .any(|chave| chave.ends_with("_ultima_versao.json"))
        );
        assert!(chave_xlsx.starts_with("emendas-siga-brasil/"));
    }
}
