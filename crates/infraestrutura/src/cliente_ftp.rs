use std::time::Duration;

use async_trait::async_trait;
use suppaftp::Status;
use suppaftp::tokio::AsyncFtpStream;
use suppaftp::types::{FileType, FtpError, Response};
use todeolho_nucleo::{
    ClienteHttp, DownloadHttp, ErroDeIngestao, InformacoesDoArquivo, RespostaCondicional,
    RespostaHttp,
};
use tracing::warn;
use url::Url;

use crate::limite::aguardar_limite;
use crate::PoliticaDeRetentativas;

const TEMPO_DE_CONEXAO: Duration = Duration::from_secs(30);

pub struct ClienteFtp {
    politica: PoliticaDeRetentativas,
}

struct UrlFtp {
    host: String,
    porta: u16,
    diretorio: String,
    arquivo: String,
}

impl ClienteFtp {
    pub fn novo(politica: PoliticaDeRetentativas) -> Self {
        Self { politica }
    }

    async fn conectar(&self, url_ftp: &UrlFtp) -> Result<AsyncFtpStream, ErroDeIngestao> {
        let endereco = format!("{}:{}", url_ftp.host, url_ftp.porta);
        let mut ftp = tokio::time::timeout(
            TEMPO_DE_CONEXAO,
            AsyncFtpStream::connect(&endereco),
        )
        .await
        .map_err(|_| erro_ftp(&endereco, "timeout na conexão FTP"))?
        .map_err(|e| mapear_erro_ftp(&endereco, e))?;

        ftp.login("anonymous", "todeolhoai@")
            .await
            .map_err(|e| mapear_erro_ftp(&endereco, e))?;
        ftp.transfer_type(FileType::Binary)
            .await
            .map_err(|e| mapear_erro_ftp(&endereco, e))?;

        if !url_ftp.diretorio.is_empty() {
            ftp.cwd(&url_ftp.diretorio)
                .await
                .map_err(|e| mapear_erro_ftp(&endereco, e))?;
        }

        Ok(ftp)
    }
}

#[async_trait]
impl ClienteHttp for ClienteFtp {
    async fn obter(&self, url: &str, _aceita: &str) -> Result<RespostaHttp, ErroDeIngestao> {
        Err(ErroDeIngestao::Http {
            url: url.to_owned(),
            detalhe: "FTP: obter em memória não suportado; use baixar()".into(),
        })
    }

    async fn obter_condicional(
        &self,
        url: &str,
        aceita: &str,
        _validador: Option<&str>,
    ) -> Result<RespostaCondicional, ErroDeIngestao> {
        let resposta = self.obter(url, aceita).await?;
        Ok(RespostaCondicional::Modificado {
            status: 200,
            tipo_de_conteudo: resposta.tipo_de_conteudo,
            versao: None,
            corpo: resposta.corpo,
        })
    }

    async fn informacoes(&self, url: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
        let url_ftp = analisar_url(url)?;
        let mut ultima_falha = None;
        for tentativa in 0..self.politica.tentativas {
            if tentativa > 0 {
                let espera = self.politica.espera_inicial * 2u32.saturating_pow(tentativa - 1);
                tokio::time::sleep(espera).await;
            }
            aguardar_limite(&url_ftp.host).await;
            match self.informacoes_interno(&url_ftp, url).await {
                Ok(info) => return Ok(info),
                Err(e) if e_transitorio(&e) => {
                    warn!("FTP informacoes tentativa {}: {e}", tentativa + 1);
                    ultima_falha = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(ultima_falha.unwrap())
    }

    async fn baixar(&self, url: &str) -> Result<DownloadHttp, ErroDeIngestao> {
        self.baixar_a_partir_de(url, 0, None).await
    }

    async fn baixar_a_partir_de(
        &self,
        url: &str,
        inicio: u64,
        _versao: Option<&str>,
    ) -> Result<DownloadHttp, ErroDeIngestao> {
        let url_ftp = analisar_url(url)?;
        let mut ultima_falha = None;
        for tentativa in 0..self.politica.tentativas {
            if tentativa > 0 {
                let espera = self.politica.espera_inicial * 2u32.saturating_pow(tentativa - 1);
                tokio::time::sleep(espera).await;
            }
            aguardar_limite(&url_ftp.host).await;
            match self.baixar_interno(&url_ftp, url, inicio).await {
                Ok(download) => return Ok(download),
                Err(e) if e_transitorio(&e) => {
                    warn!("FTP baixar tentativa {}: {e}", tentativa + 1);
                    ultima_falha = Some(e);
                }
                Err(e) => return Err(e),
            }
        }
        Err(ultima_falha.unwrap())
    }
}

impl ClienteFtp {
    async fn informacoes_interno(
        &self,
        url_ftp: &UrlFtp,
        url: &str,
    ) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
        let mut ftp = self.conectar(url_ftp).await?;
        let tamanho = ftp
            .size(&url_ftp.arquivo)
            .await
            .map(|t| t as u64)
            .map_err(|e| mapear_erro_ftp(url, e))?;
        let versao = ftp
            .mdtm(&url_ftp.arquivo)
            .await
            .ok()
            .map(|dt| dt.to_string());
        let _ = ftp.quit().await;
        Ok(InformacoesDoArquivo {
            versao,
            tamanho: Some(tamanho),
        })
    }

    async fn baixar_interno(
        &self,
        url_ftp: &UrlFtp,
        url: &str,
        inicio: u64,
    ) -> Result<DownloadHttp, ErroDeIngestao> {
        let mut ftp = self.conectar(url_ftp).await?;

        let tamanho = ftp
            .size(&url_ftp.arquivo)
            .await
            .map(|t| t as u64)
            .map_err(|e| mapear_erro_ftp(url, e))?;

        let versao = ftp
            .mdtm(&url_ftp.arquivo)
            .await
            .ok()
            .map(|dt| dt.to_string());

        if inicio > 0 {
            ftp.resume_transfer(inicio as usize)
                .await
                .map_err(|e| mapear_erro_ftp(url, e))?;
        }

        let stream = ftp
            .retr_as_stream(&url_ftp.arquivo)
            .await
            .map_err(|e| mapear_erro_ftp(url, e))?;

        Ok(DownloadHttp {
            status: if inicio > 0 { 206 } else { 200 },
            tipo_de_conteudo: "application/octet-stream".to_owned(),
            versao,
            inicio,
            tamanho_total: Some(tamanho),
            enquadramento_verificavel: true,
            fluxo: Box::new(stream),
        })
    }
}

fn analisar_url(url: &str) -> Result<UrlFtp, ErroDeIngestao> {
    let parsed = Url::parse(url).map_err(|e| erro_ftp(url, &format!("URL inválida: {e}")))?;
    if parsed.scheme() != "ftp" {
        return Err(erro_ftp(url, "esquema deve ser ftp://"));
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| erro_ftp(url, "host ausente"))?
        .to_owned();
    let porta = parsed.port().unwrap_or(21);

    let caminho = decodificar_porcento(parsed.path());
    let (diretorio, arquivo) = match caminho.rfind('/') {
        Some(pos) if pos < caminho.len() - 1 => {
            (caminho[..pos].to_owned(), caminho[pos + 1..].to_owned())
        }
        _ => return Err(erro_ftp(url, "caminho sem nome de arquivo")),
    };

    Ok(UrlFtp {
        host,
        porta,
        diretorio,
        arquivo,
    })
}

fn decodificar_porcento(texto: &str) -> String {
    let mut resultado = Vec::with_capacity(texto.len());
    let bytes = texto.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (
                (bytes[i + 1] as char).to_digit(16),
                (bytes[i + 2] as char).to_digit(16),
            ) {
                resultado.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        resultado.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(resultado).unwrap_or_else(|_| texto.to_owned())
}

fn erro_ftp(url: &str, detalhe: &str) -> ErroDeIngestao {
    ErroDeIngestao::Http {
        url: url.to_owned(),
        detalhe: format!("FTP: {detalhe}"),
    }
}

fn mapear_erro_ftp(url: &str, erro: FtpError) -> ErroDeIngestao {
    if let FtpError::UnexpectedResponse(Response { status, .. }) = &erro {
        if *status == Status::FileUnavailable {
            return ErroDeIngestao::NaoEncontrado {
                url: url.to_owned(),
            };
        }
    }
    ErroDeIngestao::Http {
        url: url.to_owned(),
        detalhe: format!("FTP: {erro}"),
    }
}

fn e_transitorio(erro: &ErroDeIngestao) -> bool {
    matches!(erro, ErroDeIngestao::Http { .. })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn analisa_url_simples() {
        let u = analisar_url("ftp://ftp.example.org/dados/arquivo.7z").unwrap();
        assert_eq!(u.host, "ftp.example.org");
        assert_eq!(u.porta, 21);
        assert_eq!(u.diretorio, "/dados");
        assert_eq!(u.arquivo, "arquivo.7z");
    }

    #[test]
    fn analisa_url_com_espaco_codificado() {
        let u =
            analisar_url("ftp://ftp.mtps.gov.br/pdet/microdados/NOVO%20CAGED/2024/202401/CAGEDMOV202401.7z")
                .unwrap();
        assert_eq!(u.host, "ftp.mtps.gov.br");
        assert_eq!(u.diretorio, "/pdet/microdados/NOVO CAGED/2024/202401");
        assert_eq!(u.arquivo, "CAGEDMOV202401.7z");
    }

    #[test]
    fn analisa_url_com_porta() {
        let u = analisar_url("ftp://ftp.example.org:2121/dados/a.7z").unwrap();
        assert_eq!(u.porta, 2121);
    }

    #[test]
    fn rejeita_url_http() {
        assert!(analisar_url("https://example.org/dados.7z").is_err());
    }

    #[test]
    fn rejeita_url_sem_arquivo() {
        assert!(analisar_url("ftp://ftp.example.org/").is_err());
    }

    #[test]
    fn decodifica_porcento_basico() {
        assert_eq!(decodificar_porcento("NOVO%20CAGED"), "NOVO CAGED");
        assert_eq!(decodificar_porcento("sem%2Fbarra"), "sem/barra");
        assert_eq!(decodificar_porcento("nenhum"), "nenhum");
    }
}
