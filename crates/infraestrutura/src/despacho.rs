use std::sync::Arc;

use async_trait::async_trait;
use todeolho_nucleo::{
    ClienteHttp, DownloadHttp, ErroDeIngestao, InformacoesDoArquivo, RespostaCondicional,
    RespostaHttp,
};

/// Encaminha requisições para o cliente correto com base no esquema da URL.
///
/// URLs `ftp://` vão para o cliente FTP; todas as demais, para o HTTP.
pub struct ClienteDespacho {
    http: Arc<dyn ClienteHttp>,
    ftp: Arc<dyn ClienteHttp>,
}

impl ClienteDespacho {
    /// Cria o despacho com os clientes HTTP e FTP concretos.
    pub fn novo(http: Arc<dyn ClienteHttp>, ftp: Arc<dyn ClienteHttp>) -> Self {
        Self { http, ftp }
    }

    fn cliente_para(&self, url: &str) -> &dyn ClienteHttp {
        if url.starts_with("ftp://") {
            &*self.ftp
        } else {
            &*self.http
        }
    }
}

#[async_trait]
impl ClienteHttp for ClienteDespacho {
    async fn obter(&self, url: &str, aceita: &str) -> Result<RespostaHttp, ErroDeIngestao> {
        self.cliente_para(url).obter(url, aceita).await
    }

    async fn obter_condicional(
        &self,
        url: &str,
        aceita: &str,
        validador: Option<&str>,
    ) -> Result<RespostaCondicional, ErroDeIngestao> {
        self.cliente_para(url)
            .obter_condicional(url, aceita, validador)
            .await
    }

    async fn obter_condicional_com_codificacao(
        &self,
        url: &str,
        aceita: &str,
        validador: Option<&str>,
    ) -> Result<(RespostaCondicional, Option<String>), ErroDeIngestao> {
        self.cliente_para(url)
            .obter_condicional_com_codificacao(url, aceita, validador)
            .await
    }

    async fn informacoes(&self, url: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
        self.cliente_para(url).informacoes(url).await
    }

    async fn baixar(&self, url: &str) -> Result<DownloadHttp, ErroDeIngestao> {
        self.cliente_para(url).baixar(url).await
    }

    async fn baixar_a_partir_de(
        &self,
        url: &str,
        inicio: u64,
        versao: Option<&str>,
    ) -> Result<DownloadHttp, ErroDeIngestao> {
        self.cliente_para(url)
            .baixar_a_partir_de(url, inicio, versao)
            .await
    }
}
