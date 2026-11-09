use async_trait::async_trait;
use std::time::Duration;

use s3::creds::Credentials;
use s3::error::S3Error;
use s3::{Bucket, BucketConfiguration, Region};
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use todeolho_nucleo::{ArmazenamentoRaw, ErroDeIngestao, ObjetoRaw};
use tokio::io::{AsyncRead, AsyncReadExt};
use tracing::{info, warn};

/// Uma parte por vez em memória: limita o consumo, seja qual for o tamanho do arquivo.
/// O `put_object_stream` do rust-s3 não serve: dimensiona a concorrência pela memória
/// livre da máquina, ignorando o limite do container.
const TAMANHO_DA_PARTE: usize = 16 * 1024 * 1024;
const TENTATIVAS_DE_ESCRITA: u32 = 4;
const ESPERA_BASE: std::time::Duration = std::time::Duration::from_millis(200);

/// Dados de conexão a um serviço compatível com S3.
#[derive(Debug, Clone)]
pub struct ConfiguracaoS3 {
    /// Endpoint do serviço (ex.: `http://armazenamento:7070`).
    pub endpoint: String,
    /// Região a enviar nas requisições.
    pub regiao: String,
    /// Nome do bucket.
    pub bucket: String,
    /// Chave de acesso (credencial).
    pub chave_de_acesso: String,
    /// Chave secreta (credencial).
    pub chave_secreta: String,
}

/// Camada Raw em qualquer serviço compatível com a API S3 (ver `docs/adr/0005`).
pub struct ArmazenamentoS3 {
    pub(crate) bucket: Box<Bucket>,
    regiao: Region,
    credenciais: Credentials,
}

impl ArmazenamentoS3 {
    async fn repetir_escrita<T, F, Fut>(
        &self,
        chave: &str,
        mut operacao: F,
    ) -> Result<T, ErroDeIngestao>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, S3Error>>,
    {
        for tentativa in 1..=TENTATIVAS_DE_ESCRITA {
            match operacao().await {
                Ok(resultado) => return Ok(resultado),
                Err(erro) if eh_transitorio(&erro) && tentativa < TENTATIVAS_DE_ESCRITA => {
                    let exponencial = ESPERA_BASE.saturating_mul(1 << (tentativa - 1));
                    let espera = exponencial + jitter(tentativa);
                    warn!(%chave, %erro, tentativa, ?espera, "gravação S3 falhou; tentando novamente");
                    tokio::time::sleep(espera).await;
                }
                Err(erro) => return Err(self.erro(chave, erro)),
            }
        }
        unreachable!("ao menos uma tentativa de gravação é feita")
    }

    /// Conecta ao serviço S3 com a configuração dada.
    pub fn novo(configuracao: ConfiguracaoS3) -> Result<Self, ErroDeIngestao> {
        let erro = |detalhe: String| ErroDeIngestao::Armazenamento {
            chave: configuracao.bucket.clone(),
            detalhe,
        };

        let regiao = Region::Custom {
            region: configuracao.regiao.clone(),
            endpoint: configuracao.endpoint.clone(),
        };
        let credenciais = Credentials::new(
            Some(&configuracao.chave_de_acesso),
            Some(&configuracao.chave_secreta),
            None,
            None,
            None,
        )
        .map_err(|e| erro(e.to_string()))?;
        // Timeout generoso: baixar/subir objetos de vários GB (ZIPs do TSE) passa do padrão.
        let bucket = Bucket::new(&configuracao.bucket, regiao.clone(), credenciais.clone())
            .map_err(|e| erro(e.to_string()))?
            .with_path_style()
            .with_request_timeout(Duration::from_secs(1800))
            .map_err(|e| erro(e.to_string()))?;

        Ok(Self {
            bucket,
            regiao,
            credenciais,
        })
    }

    pub(crate) fn erro(&self, chave: &str, detalhe: impl ToString) -> ErroDeIngestao {
        ErroDeIngestao::Armazenamento {
            chave: chave.to_owned(),
            detalhe: detalhe.to_string(),
        }
    }

    /// Lista todas as chaves sob um prefixo (percorrendo as páginas). Usado pelo refino
    /// para descobrir quais recursos já foram ingeridos no bucket.
    pub async fn listar_chaves(&self, prefixo: &str) -> Result<Vec<String>, ErroDeIngestao> {
        let paginas = self
            .bucket
            .list(prefixo.to_owned(), None)
            .await
            .map_err(|e| self.erro(prefixo, e))?;
        Ok(paginas
            .into_iter()
            .flat_map(|pagina| pagina.contents)
            .map(|objeto| objeto.key)
            .collect())
    }
}

#[async_trait]
impl ArmazenamentoRaw for ArmazenamentoS3 {
    async fn preparar(&self) -> Result<(), ErroDeIngestao> {
        let nome = self.bucket.name();
        if self
            .bucket
            .exists()
            .await
            .map_err(|e| self.erro(&nome, e))?
        {
            return Ok(());
        }

        let criacao = Bucket::create_with_path_style(
            &nome,
            self.regiao.clone(),
            self.credenciais.clone(),
            BucketConfiguration::default(),
        )
        .await;

        match criacao {
            Ok(_) => {
                info!(bucket = %nome, "bucket criado");
                Ok(())
            }
            // Outro processo criou o bucket entre a verificação e a criação.
            Err(S3Error::HttpFailWithBody(409, _)) => Ok(()),
            Err(erro) => Err(self.erro(&nome, erro)),
        }
    }

    async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao> {
        self.repetir_escrita(&objeto.chave, || {
            self.bucket.put_object_with_content_type(
                &objeto.chave,
                &objeto.conteudo,
                &objeto.tipo_de_conteudo,
            )
        })
        .await?;
        Ok(())
    }

    async fn remover(&self, chave: &str) -> Result<(), ErroDeIngestao> {
        self.repetir_escrita(chave, || self.bucket.delete_object(chave))
            .await?;
        Ok(())
    }

    async fn gravar_fluxo(
        &self,
        chave: &str,
        tipo_de_conteudo: &str,
        fluxo: &mut (dyn AsyncRead + Send + Unpin),
    ) -> Result<(), ErroDeIngestao> {
        let primeira = ler_parte(fluxo).await.map_err(|e| self.erro(chave, e))?;
        if primeira.len() < TAMANHO_DA_PARTE {
            self.repetir_escrita(chave, || {
                self.bucket
                    .put_object_with_content_type(chave, &primeira, tipo_de_conteudo)
            })
            .await?;
            return Ok(());
        }

        let envio = self
            .repetir_escrita(chave, || {
                self.bucket
                    .initiate_multipart_upload(chave, tipo_de_conteudo)
            })
            .await?;

        let resultado = self
            .enviar_partes(chave, tipo_de_conteudo, &envio.upload_id, primeira, fluxo)
            .await;
        if resultado.is_err() {
            // Sem isso, as partes já enviadas ficariam ocupando espaço no volume.
            if let Err(erro) = self.bucket.abort_upload(chave, &envio.upload_id).await {
                warn!(%chave, %erro, "não foi possível cancelar o envio");
            }
        }
        resultado
    }

    async fn descartar_envios_incompletos(&self) -> Result<usize, ErroDeIngestao> {
        let nome = self.bucket.name();
        let paginas = self
            .bucket
            .list_multiparts_uploads(None, None)
            .await
            .map_err(|e| self.erro(&nome, e))?;

        let mut descartados = 0;
        for envio in paginas.iter().flat_map(|pagina| &pagina.uploads) {
            self.bucket
                .abort_upload(&envio.key, &envio.id)
                .await
                .map_err(|e| self.erro(&envio.key, e))?;
            descartados += 1;
        }
        Ok(descartados)
    }

    async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
        match self.bucket.get_object(chave).await {
            Ok(resposta) => Ok(Some(resposta.bytes().to_vec())),
            Err(S3Error::HttpFailWithBody(404, _)) => Ok(None),
            Err(erro) => Err(self.erro(chave, erro)),
        }
    }

    async fn ler_fluxo(
        &self,
        chave: &str,
        escrita: &mut (dyn tokio::io::AsyncWrite + Send + Unpin),
    ) -> Result<bool, ErroDeIngestao> {
        match self.bucket.get_object_to_writer(chave, escrita).await {
            Ok(200) => Ok(true),
            Ok(404) => Ok(false),
            Ok(status) => Err(self.erro(
                chave,
                S3Error::HttpFailWithBody(status, "status inesperado".into()),
            )),
            Err(S3Error::HttpFailWithBody(404, _)) => Ok(false),
            Err(erro) => Err(self.erro(chave, erro)),
        }
    }

    /// Um `HEAD`, sem baixar o objeto: a pré-checagem confere o tamanho de
    /// arquivos grandes sem carregá-los na memória (ver `docs/adr/0012`).
    async fn tamanho(&self, chave: &str) -> Result<Option<u64>, ErroDeIngestao> {
        match self.bucket.head_object(chave).await {
            Ok((cabecalho, _)) => Ok(cabecalho
                .content_length
                .and_then(|bytes| u64::try_from(bytes).ok())),
            Err(S3Error::HttpFailWithBody(404, _)) => Ok(None),
            Err(erro) => Err(self.erro(chave, erro)),
        }
    }
}

impl ArmazenamentoS3 {
    async fn enviar_partes(
        &self,
        chave: &str,
        tipo_de_conteudo: &str,
        id_do_envio: &str,
        primeira: Vec<u8>,
        fluxo: &mut (dyn AsyncRead + Send + Unpin),
    ) -> Result<(), ErroDeIngestao> {
        let mut partes = Vec::new();
        let mut atual = primeira;
        loop {
            let parte = self
                .bucket
                .put_multipart_chunk(
                    atual,
                    chave,
                    partes.len() as u32 + 1,
                    id_do_envio,
                    tipo_de_conteudo,
                )
                .await
                .map_err(|e| self.erro(chave, e))?;
            partes.push(parte);

            atual = ler_parte(fluxo).await.map_err(|e| self.erro(chave, e))?;
            if atual.is_empty() {
                break;
            }
        }

        self.repetir_escrita(chave, || {
            self.bucket
                .complete_multipart_upload(chave, id_do_envio, partes.clone())
        })
        .await?;
        Ok(())
    }
}

/// Lê até completar uma parte; menos que isso só no fim do fluxo.
async fn ler_parte(fluxo: &mut (dyn AsyncRead + Send + Unpin)) -> std::io::Result<Vec<u8>> {
    let mut parte = Vec::with_capacity(TAMANHO_DA_PARTE);
    fluxo
        .take(TAMANHO_DA_PARTE as u64)
        .read_to_end(&mut parte)
        .await?;
    Ok(parte)
}

fn eh_transitorio(erro: &S3Error) -> bool {
    match erro {
        S3Error::HttpFail => true,
        S3Error::HttpFailWithBody(status, detalhe) => {
            (500..=599).contains(status)
                || *status == 408
                || *status == 429
                || detalhe
                    .to_ascii_lowercase()
                    .contains("end of file before message length reached")
        }
        S3Error::Reqwest(erro) => {
            erro.is_timeout()
                || erro.is_connect()
                || erro.is_body()
                || erro
                    .to_string()
                    .to_ascii_lowercase()
                    .contains("end of file before message length reached")
        }
        S3Error::Io(erro) => matches!(
            erro.kind(),
            std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::UnexpectedEof
        ),
        _ => false,
    }
}

fn jitter(tentativa: u32) -> std::time::Duration {
    static SEQUENCIA: AtomicU64 = AtomicU64::new(0x9e37_79b9_7f4a_7c15);
    let agora = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duracao| duracao.subsec_nanos() as u64);
    let valor = SEQUENCIA
        .fetch_add(0x9e37_79b9_7f4a_7c15, Ordering::Relaxed)
        .wrapping_add(agora)
        .wrapping_add(u64::from(tentativa));
    std::time::Duration::from_millis(valor % 200)
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread::JoinHandle;
    use std::time::{Duration, Instant};

    /// Lê `nome`, caindo para `alternativa` (as mesmas variáveis do serviço local,
    /// `TODEOLHO_S3_*`) e, por fim, para `padrao`.
    fn var(nome: &str, alternativa: &str, padrao: &str) -> String {
        std::env::var(nome)
            .or_else(|_| std::env::var(alternativa))
            .unwrap_or_else(|_| padrao.to_owned())
    }

    fn config_de_teste(endpoint: String, bucket: String) -> ConfiguracaoS3 {
        ConfiguracaoS3 {
            endpoint,
            bucket,
            regiao: var(
                "TODEOLHO_TESTE_S3_REGIAO",
                "TODEOLHO_S3_REGIAO",
                "sa-east-1",
            ),
            chave_de_acesso: var(
                "TODEOLHO_TESTE_S3_CHAVE",
                "TODEOLHO_S3_CHAVE_DE_ACESSO",
                "todeolho",
            ),
            chave_secreta: var(
                "TODEOLHO_TESTE_S3_SEGREDO",
                "TODEOLHO_S3_CHAVE_SECRETA",
                "todeolho-local",
            ),
        }
    }

    fn armazenamento_local(endpoint: String) -> ArmazenamentoS3 {
        ArmazenamentoS3::novo(config_de_teste(endpoint, "teste-raw-79".to_owned()))
            .expect("configuração S3 válida")
    }

    struct RespostaS3 {
        status: u16,
        cabecalhos: String,
        corpo: String,
    }

    fn resposta_s3(status: u16, cabecalhos: &str, corpo: &str) -> RespostaS3 {
        RespostaS3 {
            status,
            cabecalhos: cabecalhos.to_owned(),
            corpo: corpo.to_owned(),
        }
    }

    /// Simula somente o protocolo HTTP que o SDK usa nos cenários exercitados.
    /// Cada resposta fecha a conexão para manter o teste independente de estado.
    fn servidor_s3(respostas: Vec<RespostaS3>) -> (String, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        listener
            .set_nonblocking(true)
            .expect("listener não bloqueante");
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let servidor = std::thread::spawn(move || {
            let mut requisicoes = Vec::new();
            for resposta in respostas {
                let prazo = Instant::now() + Duration::from_secs(10);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(erro) if erro.kind() == std::io::ErrorKind::WouldBlock => {
                            if Instant::now() >= prazo {
                                return requisicoes;
                            }
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(erro) => panic!("falha ao aceitar conexão do SDK: {erro}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();

                let mut leitor = BufReader::new(stream.try_clone().unwrap());
                let requisicao = ler_requisicao(&mut leitor, &mut stream).unwrap();
                requisicoes.push(requisicao);

                let texto_status = match resposta.status {
                    200 => "OK",
                    204 => "No Content",
                    404 => "Not Found",
                    503 => "Service Unavailable",
                    status => panic!("status de mock não previsto: {status}"),
                };
                let tamanho_corpo = resposta
                    .cabecalhos
                    .lines()
                    .find_map(|linha| {
                        let (nome, valor) = linha.split_once(':')?;
                        nome.eq_ignore_ascii_case("content-length")
                            .then(|| valor.trim().to_owned())
                    })
                    .unwrap_or_else(|| resposta.corpo.len().to_string());
                let content_length = if resposta
                    .cabecalhos
                    .lines()
                    .any(|linha| linha.to_ascii_lowercase().starts_with("content-length:"))
                {
                    String::new()
                } else {
                    format!("Content-Length: {tamanho_corpo}\r\n")
                };
                let cabecalho = format!(
                    "HTTP/1.1 {} {}\r\n{}Connection: close\r\n{}\r\n",
                    resposta.status, texto_status, content_length, resposta.cabecalhos,
                );
                stream.write_all(cabecalho.as_bytes()).unwrap();
                stream.write_all(resposta.corpo.as_bytes()).unwrap();
                stream.flush().unwrap();
            }
            requisicoes
        });

        (endpoint, servidor)
    }

    fn ler_requisicao(
        leitor: &mut BufReader<TcpStream>,
        stream: &mut TcpStream,
    ) -> std::io::Result<String> {
        let mut texto = String::new();
        let mut linha = String::new();
        leitor.read_line(&mut linha)?;
        texto.push_str(&linha);
        let mut tamanho = 0u64;
        let mut chunked = false;
        let mut espera = false;
        loop {
            linha.clear();
            leitor.read_line(&mut linha)?;
            if linha == "\r\n" || linha.is_empty() {
                break;
            }
            let (nome, valor) = linha.split_once(':').unwrap_or(("", ""));
            match nome.to_ascii_lowercase().as_str() {
                "content-length" => tamanho = valor.trim().parse().unwrap_or(0),
                "transfer-encoding" => chunked = valor.to_ascii_lowercase().contains("chunked"),
                "expect" => espera = valor.trim().eq_ignore_ascii_case("100-continue"),
                _ => {}
            }
            texto.push_str(&linha);
        }
        if espera {
            stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
        }

        if chunked {
            loop {
                linha.clear();
                leitor.read_line(&mut linha)?;
                let tamanho_chunk =
                    u64::from_str_radix(linha.trim().split(';').next().unwrap_or("0"), 16)
                        .unwrap_or(0);
                if tamanho_chunk == 0 {
                    loop {
                        linha.clear();
                        leitor.read_line(&mut linha)?;
                        if linha == "\r\n" || linha.is_empty() {
                            break;
                        }
                    }
                    break;
                }
                std::io::copy(
                    &mut (&mut *leitor).take(tamanho_chunk),
                    &mut std::io::sink(),
                )?;
                let mut crlf = [0; 2];
                leitor.read_exact(&mut crlf)?;
            }
        } else if tamanho > 0 {
            std::io::copy(&mut (&mut *leitor).take(tamanho), &mut std::io::sink())?;
        }
        Ok(texto)
    }

    /// Conecta a um **bucket dedicado e único** por teste, para que um teste não
    /// interfira em outro nem em dados reais — `descartar_envios_incompletos`, por
    /// exemplo, cancela todos os multiparts do bucket. Devolve `None` (pula) quando
    /// `TODEOLHO_TESTE_S3_ENDPOINT` não está definido; mesmo assim os testes de
    /// integração são `#[ignore]`, então só rodam com `--ignored` e o versitygw no ar.
    fn s3_em_bucket_novo(prefixo: &str) -> Option<ArmazenamentoS3> {
        let endpoint = std::env::var("TODEOLHO_TESTE_S3_ENDPOINT").ok()?;
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let bucket = format!("teste-{prefixo}-{nanos}");
        Some(
            ArmazenamentoS3::novo(config_de_teste(endpoint, bucket))
                .expect("config de teste válida"),
        )
    }

    // --- Unit: rodam sempre, sem S3 ---

    #[tokio::test]
    async fn ler_parte_respeita_o_limite_e_depois_esgota() {
        // Entrada maior que uma parte: a 1ª leitura é exatamente uma parte, a 2ª traz
        // o resto, a 3ª é vazia. Pega a remoção do `take(TAMANHO_DA_PARTE)`.
        let dados = vec![7u8; TAMANHO_DA_PARTE + 100];
        let mut fluxo: &[u8] = &dados;
        assert_eq!(ler_parte(&mut fluxo).await.unwrap().len(), TAMANHO_DA_PARTE);
        assert_eq!(ler_parte(&mut fluxo).await.unwrap().len(), 100);
        assert!(ler_parte(&mut fluxo).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn ler_e_tamanho_usam_get_e_head() {
        let (endpoint, servidor) = servidor_s3(vec![
            resposta_s3(200, "Content-Type: text/plain\r\n", "conteudo"),
            resposta_s3(200, "Content-Length: 123\r\n", ""),
        ]);
        let s3 = armazenamento_local(endpoint);

        assert_eq!(
            s3.ler("presente").await.unwrap(),
            Some(b"conteudo".to_vec())
        );
        assert_eq!(s3.tamanho("presente").await.unwrap(), Some(123));

        let requisicoes = servidor.join().unwrap();
        assert!(requisicoes[0].starts_with("GET /teste-raw-79/presente "));
        assert!(requisicoes[1].starts_with("HEAD /teste-raw-79/presente "));
    }

    #[tokio::test]
    async fn ler_de_chave_ausente_devolve_none() {
        let (endpoint, servidor) = servidor_s3(vec![resposta_s3(
            404,
            "Content-Type: application/xml\r\n",
            "<Error><Code>NoSuchKey</Code><Message>ausente</Message></Error>",
        )]);
        let s3 = armazenamento_local(endpoint);

        assert_eq!(s3.ler("ausente").await.unwrap(), None);
        let requisicoes = servidor.join().unwrap();
        assert!(requisicoes[0].starts_with("GET /teste-raw-79/ausente "));
    }

    #[tokio::test]
    async fn tamanho_de_chave_ausente_devolve_none() {
        let (endpoint, servidor) = servidor_s3(vec![resposta_s3(404, "", "")]);
        let s3 = armazenamento_local(endpoint);

        assert_eq!(s3.tamanho("ausente").await.unwrap(), None);
        let requisicoes = servidor.join().unwrap();
        assert!(requisicoes[0].starts_with("HEAD /teste-raw-79/ausente "));
    }

    #[tokio::test]
    async fn descartar_envios_incompletos_cancela_cada_upload_listado() {
        let uploads = "<ListMultipartUploadsResult><Bucket>teste-raw-79</Bucket><IsTruncated>false</IsTruncated><Upload><Key>um.bin</Key><UploadId>upload-um</UploadId><Initiated>2026-01-01T00:00:00.000Z</Initiated><StorageClass>STANDARD</StorageClass></Upload><Upload><Key>dois.bin</Key><UploadId>upload-dois</UploadId><Initiated>2026-01-01T00:00:00.000Z</Initiated><StorageClass>STANDARD</StorageClass></Upload></ListMultipartUploadsResult>";
        let (endpoint, servidor) = servidor_s3(vec![
            resposta_s3(200, "Content-Type: application/xml\r\n", uploads),
            resposta_s3(204, "", ""),
            resposta_s3(204, "", ""),
        ]);
        let s3 = armazenamento_local(endpoint);

        assert_eq!(s3.descartar_envios_incompletos().await.unwrap(), 2);

        let requisicoes = servidor.join().unwrap();
        assert!(requisicoes[0].starts_with("GET /teste-raw-79/?uploads"));
        assert!(requisicoes[1].starts_with("DELETE /teste-raw-79/um.bin?uploadId=upload-um"));
        assert!(requisicoes[2].starts_with("DELETE /teste-raw-79/dois.bin?uploadId=upload-dois"));
    }

    #[tokio::test]
    async fn gravar_fluxo_envia_partes_e_conclui_upload_multipart() {
        let iniciada = "<InitiateMultipartUploadResult><Bucket>teste-raw-79</Bucket><Key>grande.bin</Key><UploadId>upload-1</UploadId></InitiateMultipartUploadResult>";
        let concluida = "<CompleteMultipartUploadResult><Location>http://localhost/grande.bin</Location><Bucket>teste-raw-79</Bucket><Key>grande.bin</Key><ETag>\"final\"</ETag></CompleteMultipartUploadResult>";
        let (endpoint, servidor) = servidor_s3(vec![
            resposta_s3(200, "Content-Type: application/xml\r\n", iniciada),
            resposta_s3(200, "ETag: \"parte-1\"\r\n", ""),
            resposta_s3(200, "ETag: \"parte-2\"\r\n", ""),
            resposta_s3(200, "Content-Type: application/xml\r\n", concluida),
        ]);
        let s3 = armazenamento_local(endpoint);
        let dados = vec![7u8; TAMANHO_DA_PARTE + 1];
        let mut fluxo: &[u8] = &dados;

        s3.gravar_fluxo("grande.bin", "application/octet-stream", &mut fluxo)
            .await
            .unwrap();

        let requisicoes = servidor.join().unwrap();
        assert_eq!(requisicoes.len(), 4, "inicia, envia duas partes e conclui");
        assert!(requisicoes[0].starts_with("POST /teste-raw-79/grande.bin?uploads"));
        assert!(requisicoes[1].starts_with("PUT /teste-raw-79/grande.bin?partNumber=1"));
        assert!(
            requisicoes[1]
                .to_ascii_lowercase()
                .contains(&format!("content-length: {TAMANHO_DA_PARTE}"))
        );
        assert!(requisicoes[2].starts_with("PUT /teste-raw-79/grande.bin?partNumber=2"));
        assert!(
            requisicoes[2]
                .to_ascii_lowercase()
                .contains("content-length: 1")
        );
        assert!(requisicoes[3].starts_with("POST /teste-raw-79/grande.bin?uploadId=upload-1"));
    }

    #[test]
    fn novo_constroi_sem_rede_com_config_valida() {
        let config = config_de_teste("http://127.0.0.1:7070".to_owned(), "teste-raw".to_owned());
        assert!(ArmazenamentoS3::novo(config).is_ok());
    }

    #[test]
    fn classifica_falhas_transitorias_e_permanentes_do_s3() {
        assert!(eh_transitorio(&S3Error::HttpFail));
        assert!(eh_transitorio(&S3Error::HttpFailWithBody(
            503,
            "indisponível".into()
        )));
        assert!(eh_transitorio(&S3Error::HttpFailWithBody(
            429,
            "limite".into()
        )));
        assert!(!eh_transitorio(&S3Error::HttpFailWithBody(
            403,
            "negado".into()
        )));
        assert!(!eh_transitorio(&S3Error::HttpFailWithBody(
            404,
            "ausente".into()
        )));
    }

    #[test]
    fn jitter_fica_limitado_a_duzentos_milisegundos() {
        for tentativa in 1..=TENTATIVAS_DE_ESCRITA {
            assert!(jitter(tentativa) < std::time::Duration::from_millis(200));
        }
    }

    #[tokio::test]
    async fn remover_repete_apos_falha_transitoria() {
        // 1ª tentativa: 503 (transitório, reconhecido por `eh_transitorio`).
        // 2ª tentativa: 204 (sucesso).
        let (endpoint, servidor) = servidor_s3(vec![
            resposta_s3(503, "", "Service Unavailable"),
            resposta_s3(204, "", ""),
        ]);
        let s3 = armazenamento_local(endpoint);

        // Deve ter sucesso após retry automático.
        s3.remover("objeto-teste").await.unwrap();

        let requisicoes = servidor.join().unwrap();
        assert_eq!(requisicoes.len(), 2, "deve ter tentado duas vezes");
        assert!(requisicoes[0].starts_with("DELETE /teste-raw-79/objeto-teste"));
        assert!(requisicoes[1].starts_with("DELETE /teste-raw-79/objeto-teste"));
    }

    // --- Integração: #[ignore] por padrão; `cargo test -- --ignored` com o versitygw no ar ---

    #[tokio::test]
    #[ignore = "requer versitygw: defina TODEOLHO_TESTE_S3_ENDPOINT e rode com --ignored"]
    async fn preparar_cria_um_bucket_novo_e_e_idempotente() {
        let Some(s3) = s3_em_bucket_novo("preparar") else {
            return;
        };
        assert!(!s3.bucket.exists().await.unwrap(), "bucket novo não existe");
        s3.preparar().await.unwrap();
        assert!(s3.bucket.exists().await.unwrap(), "preparar criou o bucket");
        s3.preparar().await.unwrap(); // idempotente: chamar de novo não falha
    }

    #[tokio::test]
    #[ignore = "requer versitygw: defina TODEOLHO_TESTE_S3_ENDPOINT e rode com --ignored"]
    async fn grava_le_mede_e_limpa() {
        let Some(s3) = s3_em_bucket_novo("round-trip") else {
            return;
        };
        s3.preparar().await.unwrap();
        let chave = "objeto.txt";
        let conteudo = b"ola, raw".to_vec();
        s3.gravar(ObjetoRaw {
            chave: chave.to_owned(),
            conteudo: conteudo.clone(),
            tipo_de_conteudo: "text/plain".to_owned(),
        })
        .await
        .unwrap();

        assert_eq!(s3.ler(chave).await.unwrap(), Some(conteudo.clone()));
        assert_eq!(
            s3.tamanho(chave).await.unwrap(),
            Some(conteudo.len() as u64)
        );

        // Não deixa o objeto acumulando a cada rodada.
        s3.bucket.delete_object(chave).await.unwrap();
        assert_eq!(s3.ler(chave).await.unwrap(), None);
    }

    #[tokio::test]
    #[ignore = "requer versitygw: defina TODEOLHO_TESTE_S3_ENDPOINT e rode com --ignored"]
    async fn gravar_fluxo_grava_em_parte_unica() {
        let Some(s3) = s3_em_bucket_novo("fluxo") else {
            return;
        };
        s3.preparar().await.unwrap();
        let chave = "fluxo.bin";
        let conteudo = b"conteudo via fluxo".to_vec();
        let mut fonte: &[u8] = &conteudo;
        s3.gravar_fluxo(chave, "application/octet-stream", &mut fonte)
            .await
            .unwrap();
        assert_eq!(s3.ler(chave).await.unwrap(), Some(conteudo));
        s3.bucket.delete_object(chave).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requer versitygw: defina TODEOLHO_TESTE_S3_ENDPOINT e rode com --ignored"]
    async fn ler_e_tamanho_de_chave_ausente_sao_none() {
        let Some(s3) = s3_em_bucket_novo("ausente") else {
            return;
        };
        s3.preparar().await.unwrap();
        assert_eq!(s3.ler("nao-existe").await.unwrap(), None);
        assert_eq!(s3.tamanho("nao-existe").await.unwrap(), None);
    }

    #[tokio::test]
    #[ignore = "requer versitygw: defina TODEOLHO_TESTE_S3_ENDPOINT e rode com --ignored"]
    async fn descartar_cancela_o_multipart_pendente() {
        let Some(s3) = s3_em_bucket_novo("descartar") else {
            return;
        };
        s3.preparar().await.unwrap();
        let chave = "pendente.bin";
        // Bucket dedicado: o único multipart pendente aqui é o deste teste.
        s3.bucket
            .initiate_multipart_upload(chave, "application/octet-stream")
            .await
            .unwrap();
        assert!(s3.descartar_envios_incompletos().await.unwrap() >= 1);

        // Confere o essencial: este upload específico some após a limpeza.
        let restantes = s3.bucket.list_multiparts_uploads(None, None).await.unwrap();
        assert!(
            restantes
                .iter()
                .flat_map(|pagina| &pagina.uploads)
                .all(|envio| envio.key != chave),
            "o multipart pendente deveria ter sido cancelado"
        );
    }
}
