//! Portas de saída: o que o domínio precisa do mundo externo, sem saber como é feito.

use async_trait::async_trait;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::erros::ErroDeIngestao;

/// Objeto pronto para ser gravado na camada Raw.
#[derive(Debug, Clone)]
pub struct ObjetoRaw {
    /// Chave de destino na camada Raw.
    pub chave: String,
    /// Conteúdo a gravar.
    pub conteudo: Vec<u8>,
    /// Tipo de conteúdo (`Content-Type`) a registrar.
    pub tipo_de_conteudo: String,
}

/// Falha ao remover uma chave individual durante um `remover_lote`.
#[derive(Debug, Clone)]
pub struct FalhaDeRemocao {
    /// Chave que não pôde ser removida.
    pub chave: String,
    /// Motivo da falha.
    pub detalhe: String,
}

/// Destino dos dados brutos.
#[async_trait]
pub trait ArmazenamentoRaw: Send + Sync {
    /// Garante que o destino existe. Precisa ser idempotente.
    async fn preparar(&self) -> Result<(), ErroDeIngestao>;

    /// Grava um objeto por inteiro na camada Raw.
    async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao>;

    /// Grava sem carregar tudo na memória. O objeto só fica visível ao final da transferência.
    async fn gravar_fluxo(
        &self,
        chave: &str,
        tipo_de_conteudo: &str,
        fluxo: &mut (dyn AsyncRead + Send + Unpin),
    ) -> Result<(), ErroDeIngestao>;

    /// Remove um objeto publicado na camada Raw.
    async fn remover(&self, chave: &str) -> Result<(), ErroDeIngestao> {
        Err(ErroDeIngestao::Armazenamento {
            chave: chave.to_owned(),
            detalhe: "remoção não implementada pelo adaptador".into(),
        })
    }

    /// Remove várias chaves com concorrência limitada e deadline global.
    ///
    /// Falhas individuais não abortam as demais remoções: são coletadas e devolvidas
    /// ao chamador para observabilidade e recuperação posterior. Quando o deadline
    /// expira, as remoções pendentes são canceladas e as chaves restantes aparecem
    /// como falha.
    async fn remover_lote(
        &self,
        chaves: &[String],
        concorrencia: usize,
        deadline: Duration,
    ) -> Vec<FalhaDeRemocao> {
        use futures::stream::{self, StreamExt};

        if chaves.is_empty() {
            return Vec::new();
        }

        let concorrencia = concorrencia.max(1);
        let falhas = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let removidas = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashSet::<
            String,
        >::new()));

        let trabalho = stream::iter(chaves.iter().cloned())
            .map(|chave| {
                let falhas = falhas.clone();
                let removidas = removidas.clone();
                async move {
                    match self.remover(&chave).await {
                        Ok(()) => {
                            removidas.lock().await.insert(chave);
                        }
                        Err(erro) => {
                            falhas.lock().await.push(FalhaDeRemocao {
                                chave,
                                detalhe: erro.to_string(),
                            });
                        }
                    }
                }
            })
            .buffer_unordered(concorrencia)
            .collect::<()>();

        let expirou = tokio::time::timeout(deadline, trabalho).await.is_err();

        let mut resultado = std::sync::Arc::try_unwrap(falhas)
            .expect("único dono do Arc de falhas")
            .into_inner();

        if expirou {
            let ok = std::sync::Arc::try_unwrap(removidas)
                .expect("único dono do Arc de removidas")
                .into_inner();
            let ja_falharam: std::collections::HashSet<String> =
                resultado.iter().map(|f| f.chave.clone()).collect();
            for chave in chaves {
                if !ok.contains(chave) && !ja_falharam.contains(chave) {
                    resultado.push(FalhaDeRemocao {
                        chave: chave.clone(),
                        detalhe: "deadline de remoção expirado".into(),
                    });
                }
            }
        }

        resultado
    }

    /// Apaga transferências que nunca terminaram, como as de um processo interrompido,
    /// para que não ocupem disco. Uma ingestão em andamento perde a transferência atual e
    /// a recomeça, então nenhum dado se perde.
    async fn descartar_envios_incompletos(&self) -> Result<usize, ErroDeIngestao> {
        Ok(0)
    }

    /// Conteúdo do objeto, ou `None` se ele não existir.
    async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao>;

    /// Baixa o objeto escrevendo-o em `escrita`, sem carregar tudo na memória; devolve `false`
    /// se não existir. Útil para objetos grandes (ex.: ZIPs de vários GB do TSE). O padrão usa
    /// [`Self::ler`] (carrega na memória); a infraestrutura deve transmitir em fluxo.
    async fn ler_fluxo(
        &self,
        chave: &str,
        escrita: &mut (dyn AsyncWrite + Send + Unpin),
    ) -> Result<bool, ErroDeIngestao> {
        use tokio::io::AsyncWriteExt;
        match self.ler(chave).await? {
            Some(bytes) => {
                escrita
                    .write_all(&bytes)
                    .await
                    .map_err(|erro| ErroDeIngestao::Armazenamento {
                        chave: chave.to_owned(),
                        detalhe: erro.to_string(),
                    })?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Tamanho do objeto em bytes, sem baixá-lo (um `HEAD`), ou `None` se não
    /// existir. A implementação padrão lê o objeto; as de infraestrutura devem
    /// sobrescrever com um `HEAD` para não carregar arquivos grandes na memória.
    async fn tamanho(&self, chave: &str) -> Result<Option<u64>, ErroDeIngestao> {
        Ok(self.ler(chave).await?.map(|conteudo| conteudo.len() as u64))
    }
}

/// Resposta bem-sucedida de uma API.
#[derive(Debug, Clone)]
pub struct RespostaHttp {
    /// Tipo de conteúdo (`Content-Type`) devolvido.
    pub tipo_de_conteudo: String,
    /// Corpo da resposta.
    pub corpo: Vec<u8>,
}

/// Resultado de um GET condicional.
#[derive(Debug, Clone)]
pub enum RespostaCondicional {
    /// 304: o conteúdo não mudou desde o validador enviado.
    NaoModificado,
    /// A origem devolveu um corpo novo.
    Modificado {
        /// Status HTTP que devolveu o corpo.
        status: u16,
        /// Tipo de conteúdo (`Content-Type`) do novo corpo.
        tipo_de_conteudo: String,
        /// Validador do novo conteúdo (ETag), quando informado.
        versao: Option<String>,
        /// Corpo novo.
        corpo: Vec<u8>,
    },
}

/// O que a origem informa sobre um arquivo sem precisar baixá-lo.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InformacoesDoArquivo {
    /// Identifica a versão do arquivo (ETag).
    pub versao: Option<String>,
    /// Tamanho do arquivo em bytes (`Content-Length`), quando informado.
    pub tamanho: Option<u64>,
}

/// Fluxo de bytes lido aos poucos, sem carregar tudo na memória.
pub type FluxoDeBytes = Box<dyn AsyncRead + Send + Unpin>;

/// Resposta bem-sucedida cujo corpo ainda será lido aos poucos.
pub struct DownloadHttp {
    /// Status HTTP da resposta.
    pub status: u16,
    /// Tipo de conteúdo (`Content-Type`) do arquivo.
    pub tipo_de_conteudo: String,
    /// Identifica a versão do arquivo na origem (ETag), quando informada.
    pub versao: Option<String>,
    /// Deslocamento, em bytes, onde este fluxo começa no arquivo. `0` num
    /// download do início; numa retomada (`206 Partial Content`) é o início do
    /// `Content-Range`. Quem retoma compara com o offset pedido: se não bater, a
    /// origem não honrou a retomada e o download deve recomeçar do zero.
    pub inicio: u64,
    /// Tamanho total do arquivo em bytes, quando a origem informa
    /// (`Content-Length` de um `200`, ou o total do `Content-Range` de um `206`).
    pub tamanho_total: Option<u64>,
    /// `true` quando o protocolo distingue fim válido de EOF truncado sem total.
    pub enquadramento_verificavel: bool,
    /// Fluxo do corpo, lido sob demanda.
    pub fluxo: FluxoDeBytes,
}

/// Acesso às APIs públicas. Falhas transitórias são tratadas pela implementação.
#[async_trait]
pub trait ClienteHttp: Send + Sync {
    /// GET simples: devolve o corpo inteiro em memória.
    async fn obter(&self, url: &str, aceita: &str) -> Result<RespostaHttp, ErroDeIngestao>;

    /// GET condicional: envia o validador (If-None-Match) e devolve
    /// `NaoModificado` em caso de 304. A implementação padrão ignora o validador
    /// e sempre baixa, para fontes que não suportam requisição condicional.
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

    /// GET condicional com o valor de `Content-Encoding` da resposta. O padrão
    /// preserva os clientes existentes, que entregam corpos sem codificação.
    async fn obter_condicional_com_codificacao(
        &self,
        url: &str,
        aceita: &str,
        validador: Option<&str>,
    ) -> Result<(RespostaCondicional, Option<String>), ErroDeIngestao> {
        self.obter_condicional(url, aceita, validador)
            .await
            .map(|resposta| (resposta, None))
    }

    /// Consulta o arquivo na origem, sem baixá-lo.
    async fn informacoes(&self, url: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao>;

    /// Baixa um arquivo como fluxo, para gravá-lo sem carregá-lo na memória.
    async fn baixar(&self, url: &str) -> Result<DownloadHttp, ErroDeIngestao>;

    /// Baixa a partir de `inicio` (em bytes; `0` é do começo), para **retomar**
    /// um download interrompido sem perder nem duplicar byte. `versao` é o
    /// validador (ETag) da primeira resposta: a implementação deve enviar
    /// `If-Range`, de modo a só retomar se o arquivo não mudou. O
    /// [`DownloadHttp::inicio`] devolvido diz onde o fluxo realmente começa — se
    /// não for igual a `inicio`, a origem não honrou a retomada e o chamador deve
    /// recomeçar do zero em vez de emendar.
    ///
    /// A implementação padrão ignora `inicio`/`versao` e baixa do começo, para
    /// clientes que não suportam `Range`; quem retoma detecta o `inicio` devolvido
    /// (`0`) diferente do pedido e recomeça.
    async fn baixar_a_partir_de(
        &self,
        url: &str,
        inicio: u64,
        versao: Option<&str>,
    ) -> Result<DownloadHttp, ErroDeIngestao> {
        let _ = (inicio, versao);
        self.baixar(url).await
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::AsyncRead;

    /// Adapter stub que falha sempre no `remover`, simulando S3 indisponível.
    struct ArmazenamentoQueFalha;

    #[async_trait]
    impl ArmazenamentoRaw for ArmazenamentoQueFalha {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn gravar(&self, _: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn gravar_fluxo(
            &self,
            _: &str,
            _: &str,
            _: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn remover(&self, chave: &str) -> Result<(), ErroDeIngestao> {
            Err(ErroDeIngestao::Armazenamento {
                chave: chave.to_owned(),
                detalhe: "S3 indisponível".into(),
            })
        }
        async fn ler(&self, _: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(None)
        }
    }

    /// Adapter stub que aceita remoções e conta chamadas.
    struct ArmazenamentoContador {
        chamadas: AtomicUsize,
    }

    #[async_trait]
    impl ArmazenamentoRaw for ArmazenamentoContador {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn gravar(&self, _: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn gravar_fluxo(
            &self,
            _: &str,
            _: &str,
            _: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn remover(&self, _chave: &str) -> Result<(), ErroDeIngestao> {
            self.chamadas.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
        async fn ler(&self, _: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(None)
        }
    }

    /// Adapter stub que demora para sempre no `remover`, simulando timeout.
    struct ArmazenamentoLento;

    #[async_trait]
    impl ArmazenamentoRaw for ArmazenamentoLento {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn gravar(&self, _: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn gravar_fluxo(
            &self,
            _: &str,
            _: &str,
            _: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn remover(&self, _chave: &str) -> Result<(), ErroDeIngestao> {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            Ok(())
        }
        async fn ler(&self, _: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn remover_lote_sem_chaves_devolve_vazio() {
        let arm = ArmazenamentoContador {
            chamadas: AtomicUsize::new(0),
        };
        let falhas = arm.remover_lote(&[], 4, Duration::from_secs(5)).await;
        assert!(falhas.is_empty());
        assert_eq!(arm.chamadas.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn remover_lote_com_sucesso_nao_devolve_falhas() {
        let arm = ArmazenamentoContador {
            chamadas: AtomicUsize::new(0),
        };
        let chaves: Vec<String> = (0..10).map(|i| format!("chave-{i}")).collect();
        let falhas = arm.remover_lote(&chaves, 4, Duration::from_secs(5)).await;
        assert!(falhas.is_empty());
        assert_eq!(arm.chamadas.load(Ordering::Relaxed), 10);
    }

    #[tokio::test]
    async fn remover_lote_coleta_falhas_individuais() {
        let arm = ArmazenamentoQueFalha;
        let chaves = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        let falhas = arm.remover_lote(&chaves, 4, Duration::from_secs(5)).await;
        assert_eq!(falhas.len(), 3);
        let chaves_falhadas: std::collections::HashSet<&str> =
            falhas.iter().map(|f| f.chave.as_str()).collect();
        assert_eq!(
            chaves_falhadas,
            ["a", "b", "c"]
                .into_iter()
                .collect::<std::collections::HashSet<&str>>(),
        );
        for falha in &falhas {
            assert!(
                falha.detalhe.contains("S3 indisponível"),
                "{}",
                falha.detalhe
            );
        }
    }

    #[tokio::test]
    async fn remover_lote_respeita_deadline() {
        let arm = ArmazenamentoLento;
        let chaves = vec!["x".to_owned(), "y".to_owned()];
        let inicio = std::time::Instant::now();
        let falhas = arm
            .remover_lote(&chaves, 4, Duration::from_millis(100))
            .await;
        let decorrido = inicio.elapsed();
        // Deve ter terminado rapidamente (timeout de 100ms + margem), não 3600s.
        assert!(
            decorrido < Duration::from_secs(30),
            "deadline deveria ter cortado; decorrido: {decorrido:?}"
        );
        // Todas as chaves pendentes aparecem como falha por timeout.
        let chaves_falhadas: std::collections::HashSet<&str> =
            falhas.iter().map(|f| f.chave.as_str()).collect();
        assert_eq!(
            chaves_falhadas,
            ["x", "y"]
                .into_iter()
                .collect::<std::collections::HashSet<&str>>(),
        );
        assert!(
            falhas.iter().all(|f| f.detalhe.contains("deadline")),
            "todas as falhas devem ser por deadline"
        );
    }

    #[tokio::test]
    async fn adapter_sem_suporte_a_remocao_devolve_erro() {
        // A porta default retorna erro "remoção não implementada pelo adaptador".
        struct SemRemocao;

        #[async_trait]
        impl ArmazenamentoRaw for SemRemocao {
            async fn preparar(&self) -> Result<(), ErroDeIngestao> {
                Ok(())
            }
            async fn gravar(&self, _: ObjetoRaw) -> Result<(), ErroDeIngestao> {
                Ok(())
            }
            async fn gravar_fluxo(
                &self,
                _: &str,
                _: &str,
                _: &mut (dyn AsyncRead + Send + Unpin),
            ) -> Result<(), ErroDeIngestao> {
                Ok(())
            }
            // Não sobrescreve remover() — usa o default que retorna erro.
            async fn ler(&self, _: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
                Ok(None)
            }
        }

        let arm = SemRemocao;
        let chaves = vec!["a".to_owned(), "b".to_owned()];
        let falhas = arm.remover_lote(&chaves, 4, Duration::from_secs(5)).await;
        assert_eq!(falhas.len(), 2);
        for falha in &falhas {
            assert!(
                falha.detalhe.contains("remoção não implementada"),
                "{}",
                falha.detalhe
            );
        }
    }
}
