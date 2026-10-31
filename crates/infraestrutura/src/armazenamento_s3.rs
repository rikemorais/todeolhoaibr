use async_trait::async_trait;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::{Bucket, BucketConfiguration, Region};
use todeolho_nucleo::{ArmazenamentoRaw, ErroDeIngestao, ObjetoRaw};
use tokio::io::{AsyncRead, AsyncReadExt};
use tracing::{info, warn};

/// Uma parte por vez em memória: limita o consumo, seja qual for o tamanho do arquivo.
/// O `put_object_stream` do rust-s3 não serve: dimensiona a concorrência pela memória
/// livre da máquina, ignorando o limite do container.
const TAMANHO_DA_PARTE: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ConfiguracaoS3 {
    pub endpoint: String,
    pub regiao: String,
    pub bucket: String,
    pub chave_de_acesso: String,
    pub chave_secreta: String,
}

/// Camada Raw em qualquer serviço compatível com a API S3 (ver `docs/adr/0005`).
pub struct ArmazenamentoS3 {
    pub(crate) bucket: Box<Bucket>,
    regiao: Region,
    credenciais: Credentials,
}

impl ArmazenamentoS3 {
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
        self.bucket
            .put_object_with_content_type(&objeto.chave, &objeto.conteudo, &objeto.tipo_de_conteudo)
            .await
            .map_err(|e| self.erro(&objeto.chave, e))?;
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
            self.bucket
                .put_object_with_content_type(chave, &primeira, tipo_de_conteudo)
                .await
                .map_err(|e| self.erro(chave, e))?;
            return Ok(());
        }

        let envio = self
            .bucket
            .initiate_multipart_upload(chave, tipo_de_conteudo)
            .await
            .map_err(|e| self.erro(chave, e))?;

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

        self.bucket
            .complete_multipart_upload(chave, id_do_envio, partes)
            .await
            .map_err(|e| self.erro(chave, e))?;
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
