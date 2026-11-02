use async_trait::async_trait;
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
        let bucket = Bucket::new(&configuracao.bucket, regiao.clone(), credenciais.clone())
            .map_err(|e| erro(e.to_string()))?
            .with_path_style();

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
        self.bucket
            .delete_object(chave)
            .await
            .map_err(|e| self.erro(chave, e))?;
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
