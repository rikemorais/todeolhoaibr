//! Portas de saída: o que o domínio precisa do mundo externo, sem saber como é feito.

use async_trait::async_trait;
use tokio::io::AsyncRead;

use crate::erros::ErroDeIngestao;

/// Objeto pronto para ser gravado na camada Raw.
#[derive(Debug, Clone)]
pub struct ObjetoRaw {
    pub chave: String,
    pub conteudo: Vec<u8>,
    pub tipo_de_conteudo: String,
}

/// Destino dos dados brutos.
#[async_trait]
pub trait ArmazenamentoRaw: Send + Sync {
    /// Garante que o destino existe. Precisa ser idempotente.
    async fn preparar(&self) -> Result<(), ErroDeIngestao>;

    async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao>;

    /// Grava sem carregar tudo na memória. O objeto só fica visível ao final da transferência.
    async fn gravar_fluxo(
        &self,
        chave: &str,
        tipo_de_conteudo: &str,
        fluxo: &mut (dyn AsyncRead + Send + Unpin),
    ) -> Result<(), ErroDeIngestao>;

    /// Apaga transferências que nunca terminaram, como as de um processo interrompido,
    /// para que não ocupem disco. Uma ingestão em andamento perde a transferência atual e
    /// a recomeça, então nenhum dado se perde.
    async fn descartar_envios_incompletos(&self) -> Result<usize, ErroDeIngestao> {
        Ok(0)
    }

    /// Conteúdo do objeto, ou `None` se ele não existir.
    async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao>;
}

/// Resposta bem-sucedida de uma API.
#[derive(Debug, Clone)]
pub struct RespostaHttp {
    pub tipo_de_conteudo: String,
    pub corpo: Vec<u8>,
}

/// O que a origem informa sobre um arquivo sem precisar baixá-lo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InformacoesDoArquivo {
    /// Identifica a versão do arquivo (ETag).
    pub versao: Option<String>,
    pub tamanho: Option<u64>,
}

pub type FluxoDeBytes = Box<dyn AsyncRead + Send + Unpin>;

/// Resposta bem-sucedida cujo corpo ainda será lido aos poucos.
pub struct DownloadHttp {
    pub tipo_de_conteudo: String,
    /// Identifica a versão do arquivo na origem (ETag), quando informada.
    pub versao: Option<String>,
    pub fluxo: FluxoDeBytes,
}

/// Acesso às APIs públicas. Falhas transitórias são tratadas pela implementação.
#[async_trait]
pub trait ClienteHttp: Send + Sync {
    async fn obter(&self, url: &str, aceita: &str) -> Result<RespostaHttp, ErroDeIngestao>;

    /// Consulta o arquivo na origem, sem baixá-lo.
    async fn informacoes(&self, url: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao>;

    async fn baixar(&self, url: &str) -> Result<DownloadHttp, ErroDeIngestao>;
}
