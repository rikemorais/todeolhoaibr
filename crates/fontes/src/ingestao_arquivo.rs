use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use sha2::{Digest, Sha256};
use todeolho_nucleo::{
    ContextoDeExecucao, DecisaoDeIngestao, ErroDeIngestao, FluxoDeBytes, LocalizacaoRaw,
    ManifestoRaw, ObjetoRaw, ParteRaw, ResumoDaTarefa, Tarefa, UltimaVersao, decidir,
    estado_na_raw,
};
use tokio::io::{AsyncRead, ReadBuf};
use tracing::warn;

use crate::ingestao_http::extensao;

const TENTATIVAS: u32 = 3;
const ESPERA_ENTRE_TENTATIVAS: Duration = Duration::from_secs(15);

/// Baixa um arquivo grande para a camada Raw sem carregá-lo na memória.
///
/// Não baixa de novo o que não mudou na origem: compara a versão (ETag) com a da última
/// ingestão concluída. Isso também permite retomar uma carga longa que foi interrompida.
pub struct TarefaDeArquivo {
    nome: String,
    fonte: String,
    recurso: String,
    url: String,
    espera_entre_tentativas: Duration,
}

struct ArquivoGravado {
    parte: ParteRaw,
    versao: Option<String>,
}

impl TarefaDeArquivo {
    /// `recurso` pode conter subpastas, como `votacao_secao/ano=2024/uf=AC`.
    pub fn nova(nome: String, fonte: &str, recurso: String, url: String) -> Self {
        Self {
            nome,
            fonte: fonte.to_owned(),
            recurso,
            url,
            espera_entre_tentativas: ESPERA_ENTRE_TENTATIVAS,
        }
    }

    async fn transferir(
        &self,
        contexto: &ContextoDeExecucao,
        local: &LocalizacaoRaw,
    ) -> Result<ArquivoGravado, ErroDeIngestao> {
        let download = contexto.http.baixar(&self.url).await?;
        let chave = local.parte(1, extensao(&download.tipo_de_conteudo));
        let mut leitor = LeitorComResumo::novo(download.fluxo);

        contexto
            .armazenamento
            .gravar_fluxo(&chave, &download.tipo_de_conteudo, &mut leitor)
            .await?;

        Ok(ArquivoGravado {
            parte: ParteRaw {
                chave,
                url: self.url.clone(),
                bytes: leitor.bytes,
                sha256: hex::encode(leitor.hash.finalize()),
                tipo_de_conteudo: download.tipo_de_conteudo,
            },
            versao: download.versao,
        })
    }

    fn erro_de_manifesto(&self, erro: serde_json::Error) -> ErroDeIngestao {
        ErroDeIngestao::Manifesto {
            recurso: self.recurso.clone(),
            detalhe: erro.to_string(),
        }
    }
}

#[async_trait]
impl Tarefa for TarefaDeArquivo {
    fn nome(&self) -> &str {
        &self.nome
    }

    /// O que já está atualizado na Raw não será baixado de novo, então não ocupa espaço novo.
    async fn bytes_a_baixar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<Option<u64>, ErroDeIngestao> {
        let informacoes = contexto.http.informacoes(&self.url).await?;
        let estado =
            estado_na_raw(contexto.armazenamento.as_ref(), &self.fonte, &self.recurso).await?;
        match decidir(&informacoes, estado.as_ref()) {
            DecisaoDeIngestao::Reaproveitar => Ok(Some(0)),
            DecisaoDeIngestao::Ingerir(_) => Ok(informacoes.tamanho),
        }
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let local = LocalizacaoRaw::nova(
            &self.fonte,
            &self.recurso,
            contexto.iniciada_em,
            &contexto.id_execucao,
        );
        let chave_da_versao = LocalizacaoRaw::ultima_versao(&self.fonte, &self.recurso);

        let informacoes = contexto.http.informacoes(&self.url).await?;
        let estado =
            estado_na_raw(contexto.armazenamento.as_ref(), &self.fonte, &self.recurso).await?;
        if matches!(
            decidir(&informacoes, estado.as_ref()),
            DecisaoDeIngestao::Reaproveitar
        ) {
            return Ok(ResumoDaTarefa {
                reaproveitada: true,
                ..ResumoDaTarefa::default()
            });
        }

        // A conexão pode cair no meio de um arquivo grande. Nada fica visível na Raw
        // antes do fim da transferência, então basta recomeçar.
        let mut falhas = 0;
        let arquivo = loop {
            match self.transferir(contexto, &local).await {
                Ok(arquivo) => break arquivo,
                Err(erro) => {
                    falhas += 1;
                    if falhas >= TENTATIVAS {
                        return Err(erro);
                    }
                    warn!(tarefa = %self.nome, %erro, "transferência falhou; recomeçando o arquivo");
                    tokio::time::sleep(self.espera_entre_tentativas).await;
                }
            }
        };

        let resumo = ResumoDaTarefa {
            partes: 1,
            bytes: arquivo.parte.bytes,
            reaproveitada: false,
        };

        let manifesto = ManifestoRaw {
            fonte: self.fonte.clone(),
            recurso: self.recurso.clone(),
            id_execucao: contexto.id_execucao.clone(),
            url_inicial: self.url.clone(),
            iniciada_em: contexto.iniciada_em,
            concluida_em: Utc::now(),
            partes: vec![arquivo.parte],
        };
        gravar_json(
            contexto,
            local.manifesto(),
            serde_json::to_vec_pretty(&manifesto).map_err(|e| self.erro_de_manifesto(e))?,
        )
        .await?;

        // Grava o ponteiro mesmo sem validador (versao `None`): é o que permite
        // a pré-checagem por tamanho reaproveitar numa próxima rodada.
        let ultima = UltimaVersao {
            versao: arquivo.versao,
            manifesto: local.manifesto(),
        };
        gravar_json(
            contexto,
            chave_da_versao,
            serde_json::to_vec_pretty(&ultima).map_err(|e| self.erro_de_manifesto(e))?,
        )
        .await?;

        Ok(resumo)
    }
}

async fn gravar_json(
    contexto: &ContextoDeExecucao,
    chave: String,
    conteudo: Vec<u8>,
) -> Result<(), ErroDeIngestao> {
    contexto
        .armazenamento
        .gravar(ObjetoRaw {
            chave,
            conteudo,
            tipo_de_conteudo: "application/json".to_owned(),
        })
        .await
}

/// Calcula tamanho e hash enquanto os bytes passam.
struct LeitorComResumo {
    fluxo: FluxoDeBytes,
    hash: Sha256,
    bytes: u64,
}

impl LeitorComResumo {
    fn novo(fluxo: FluxoDeBytes) -> Self {
        Self {
            fluxo,
            hash: Sha256::new(),
            bytes: 0,
        }
    }
}

impl AsyncRead for LeitorComResumo {
    fn poll_read(
        self: Pin<&mut Self>,
        contexto: &mut Context<'_>,
        destino: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let este = self.get_mut();
        let antes = destino.filled().len();
        let resultado = Pin::new(&mut este.fluxo).poll_read(contexto, destino);
        if let Poll::Ready(Ok(())) = resultado {
            let novos = &destino.filled()[antes..];
            este.hash.update(novos);
            este.bytes += novos.len() as u64;
        }
        resultado
    }
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use todeolho_nucleo::{
        ArmazenamentoRaw, ClienteHttp, DownloadHttp, InformacoesDoArquivo, RespostaHttp,
    };
    use tokio::io::AsyncReadExt;

    use super::*;

    const CONTEUDO: &[u8] = b"PK conteudo do arquivo";

    #[derive(Default)]
    struct ArmazenamentoEmMemoria {
        objetos: Mutex<HashMap<String, Vec<u8>>>,
    }

    #[async_trait]
    impl ArmazenamentoRaw for ArmazenamentoEmMemoria {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }

        async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            self.objetos
                .lock()
                .unwrap()
                .insert(objeto.chave, objeto.conteudo);
            Ok(())
        }

        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.objetos.lock().unwrap().get(chave).cloned())
        }

        async fn gravar_fluxo(
            &self,
            chave: &str,
            _: &str,
            fluxo: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            let mut conteudo = Vec::new();
            fluxo.read_to_end(&mut conteudo).await.map_err(|erro| {
                ErroDeIngestao::Armazenamento {
                    chave: chave.to_owned(),
                    detalhe: erro.to_string(),
                }
            })?;
            self.objetos
                .lock()
                .unwrap()
                .insert(chave.to_owned(), conteudo);
            Ok(())
        }
    }

    /// Falha nos `falhas` primeiros downloads, cortando o fluxo no meio.
    struct HttpFalso {
        versao: Mutex<Option<String>>,
        falhas: Mutex<u32>,
        downloads: Mutex<u32>,
    }

    impl HttpFalso {
        fn novo(versao: Option<&str>, falhas: u32) -> Self {
            Self {
                versao: Mutex::new(versao.map(str::to_owned)),
                falhas: Mutex::new(falhas),
                downloads: Mutex::new(0),
            }
        }
    }

    #[async_trait]
    impl ClienteHttp for HttpFalso {
        async fn obter(&self, _: &str, _: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            unreachable!("a tarefa de arquivo não usa respostas em memória")
        }

        async fn informacoes(&self, _: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            Ok(InformacoesDoArquivo {
                versao: self.versao.lock().unwrap().clone(),
                tamanho: Some(CONTEUDO.len() as u64),
            })
        }

        async fn baixar(&self, _: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            *self.downloads.lock().unwrap() += 1;
            let mut falhas = self.falhas.lock().unwrap();
            let fluxo: FluxoDeBytes = if *falhas > 0 {
                *falhas -= 1;
                Box::new(leitor_com_erro())
            } else {
                Box::new(CONTEUDO)
            };
            Ok(DownloadHttp {
                tipo_de_conteudo: "application/zip".into(),
                versao: self.versao.lock().unwrap().clone(),
                fluxo,
            })
        }
    }

    fn leitor_com_erro() -> impl AsyncRead + Send + Unpin {
        struct Quebrado;
        impl AsyncRead for Quebrado {
            fn poll_read(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
                _: &mut ReadBuf<'_>,
            ) -> Poll<io::Result<()>> {
                Poll::Ready(Err(io::Error::other("conexão perdida")))
            }
        }
        Quebrado
    }

    struct Cenario {
        contexto: ContextoDeExecucao,
        armazenamento: Arc<ArmazenamentoEmMemoria>,
        http: Arc<HttpFalso>,
    }

    fn cenario(http: HttpFalso) -> Cenario {
        let armazenamento = Arc::new(ArmazenamentoEmMemoria::default());
        let http = Arc::new(http);
        Cenario {
            contexto: ContextoDeExecucao {
                id_execucao: "exec".into(),
                iniciada_em: Utc::now(),
                armazenamento: armazenamento.clone(),
                http: http.clone(),
            },
            armazenamento,
            http,
        }
    }

    fn tarefa() -> TarefaDeArquivo {
        let mut tarefa = TarefaDeArquivo::nova(
            "2024_AC".into(),
            "tse",
            "votacao_secao/ano=2024/uf=AC".into(),
            "https://origem/arquivo.zip".into(),
        );
        tarefa.espera_entre_tentativas = Duration::ZERO;
        tarefa
    }

    fn chaves(armazenamento: &ArmazenamentoEmMemoria, sufixo: &str) -> Vec<String> {
        armazenamento
            .objetos
            .lock()
            .unwrap()
            .keys()
            .filter(|chave| chave.ends_with(sufixo))
            .cloned()
            .collect()
    }

    #[tokio::test]
    async fn grava_o_arquivo_sem_alterar_com_manifesto_e_versao() {
        let cenario = cenario(HttpFalso::novo(Some("v1"), 0));

        let resumo = tarefa().executar(&cenario.contexto).await.unwrap();

        assert_eq!(resumo.bytes, CONTEUDO.len() as u64);
        assert!(!resumo.reaproveitada);

        let parte = &chaves(&cenario.armazenamento, "/parte-00001.zip")[0];
        assert!(parte.starts_with("tse/votacao_secao/ano=2024/uf=AC/data_ingestao="));
        let objetos = cenario.armazenamento.objetos.lock().unwrap();
        assert_eq!(objetos[parte], CONTEUDO);

        let chave_do_manifesto = parte.replace("parte-00001.zip", "_manifesto.json");
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&objetos[&chave_do_manifesto]).unwrap();
        assert_eq!(
            manifesto.partes[0].sha256,
            hex::encode(Sha256::digest(CONTEUDO))
        );
    }

    #[tokio::test]
    async fn nao_baixa_de_novo_o_que_nao_mudou_na_origem() {
        let cenario = cenario(HttpFalso::novo(Some("v1"), 0));

        tarefa().executar(&cenario.contexto).await.unwrap();
        let segunda = tarefa().executar(&cenario.contexto).await.unwrap();

        assert!(segunda.reaproveitada);
        assert_eq!(segunda.bytes, 0);
        assert_eq!(*cenario.http.downloads.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn baixa_de_novo_quando_a_origem_muda() {
        let cenario = cenario(HttpFalso::novo(Some("v1"), 0));

        tarefa().executar(&cenario.contexto).await.unwrap();
        *cenario.http.versao.lock().unwrap() = Some("v2".into());
        let segunda = tarefa().executar(&cenario.contexto).await.unwrap();

        assert!(!segunda.reaproveitada);
        assert_eq!(*cenario.http.downloads.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn sem_validador_reaproveita_quando_o_tamanho_bate() {
        let cenario = cenario(HttpFalso::novo(None, 0));

        tarefa().executar(&cenario.contexto).await.unwrap();
        let segunda = tarefa().executar(&cenario.contexto).await.unwrap();

        assert!(segunda.reaproveitada);
        assert_eq!(*cenario.http.downloads.lock().unwrap(), 1);
        // Sem validador, o ponteiro ainda é gravado para a checagem por tamanho.
        assert!(!chaves(&cenario.armazenamento, "_ultima_versao.json").is_empty());
    }

    #[tokio::test]
    async fn reingere_quando_a_parte_na_raw_foi_truncada() {
        let cenario = cenario(HttpFalso::novo(Some("v1"), 0));
        tarefa().executar(&cenario.contexto).await.unwrap();

        // Mesmo com o validador igual, uma parte truncada força a reingestão.
        {
            let mut objetos = cenario.armazenamento.objetos.lock().unwrap();
            let chave = objetos
                .keys()
                .find(|c| c.ends_with("/parte-00001.zip"))
                .cloned()
                .unwrap();
            objetos.insert(chave, b"truncado".to_vec());
        }

        let segunda = tarefa().executar(&cenario.contexto).await.unwrap();

        assert!(!segunda.reaproveitada);
        assert_eq!(*cenario.http.downloads.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn estima_o_tamanho_e_zero_para_o_que_ja_esta_atualizado() {
        let cenario = cenario(HttpFalso::novo(Some("v1"), 0));
        let antes = tarefa().bytes_a_baixar(&cenario.contexto).await.unwrap();
        assert_eq!(antes, Some(CONTEUDO.len() as u64));

        tarefa().executar(&cenario.contexto).await.unwrap();

        let depois = tarefa().bytes_a_baixar(&cenario.contexto).await.unwrap();
        assert_eq!(depois, Some(0));
    }

    #[tokio::test]
    async fn recomeca_o_arquivo_quando_a_conexao_cai() {
        let cenario = cenario(HttpFalso::novo(Some("v1"), 2));

        let resumo = tarefa().executar(&cenario.contexto).await.unwrap();

        assert_eq!(resumo.bytes, CONTEUDO.len() as u64);
        assert_eq!(*cenario.http.downloads.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn desiste_sem_manifesto_nem_versao_depois_das_tentativas() {
        let cenario = cenario(HttpFalso::novo(Some("v1"), 3));

        assert!(tarefa().executar(&cenario.contexto).await.is_err());

        assert_eq!(*cenario.http.downloads.lock().unwrap(), TENTATIVAS);
        assert!(chaves(&cenario.armazenamento, "_manifesto.json").is_empty());
        assert!(chaves(&cenario.armazenamento, "_ultima_versao.json").is_empty());
    }
}
