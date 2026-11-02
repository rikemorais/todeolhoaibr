//! Ingestão de arquivos grandes (como os ZIPs do TSE): baixa em fluxo, sem
//! carregar o arquivo na memória, e pula o que não mudou na origem.

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use sha2::{Digest, Sha256};
use todeolho_nucleo::{
    ClienteHttp, ContextoDeExecucao, DecisaoDeIngestao, DownloadHttp, ErroDeIngestao, FluxoDeBytes,
    LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw, ResumoDaTarefa, Tarefa, UltimaVersao,
    decidir, estado_na_raw, publicar_ponteiro,
};
use tokio::io::{AsyncRead, ReadBuf};
use tracing::warn;

use crate::ingestao_http::extensao;

const TENTATIVAS: u32 = 3;
const ESPERA_ENTRE_TENTATIVAS: Duration = Duration::from_secs(15);
/// Quantas vezes retomar um mesmo arquivo (reconexões) antes de desistir desta
/// transferência. Alto porque a origem da Câmara derruba arquivos grandes com
/// frequência; estourar o limite faz a tarefa recomeçar o arquivo do zero.
const LIMITE_DE_RETOMADAS: u32 = 20;
/// Espera base entre retomadas de um fluxo caído; cresce um pouco a cada falha.
const ESPERA_ENTRE_RETOMADAS: Duration = Duration::from_secs(2);

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
    espera_entre_retomadas: Duration,
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
            espera_entre_retomadas: ESPERA_ENTRE_RETOMADAS,
        }
    }

    async fn transferir(
        &self,
        contexto: &ContextoDeExecucao,
        local: &LocalizacaoRaw,
        tamanho_informado: Option<u64>,
    ) -> Result<ArquivoGravado, ErroDeIngestao> {
        let primeiro = contexto.http.baixar_a_partir_de(&self.url, 0, None).await?;
        // A primeira resposta tem de começar do início: um 206 fora de posição
        // seria um pedaço tratado como o arquivo inteiro.
        if primeiro.inicio != 0 {
            return Err(ErroDeIngestao::Http {
                url: self.url.clone(),
                detalhe: format!(
                    "a origem respondeu um pedaço (offset {}) em vez do arquivo inteiro",
                    primeiro.inicio
                ),
            });
        }
        let tipo_de_conteudo = primeiro.tipo_de_conteudo.clone();
        let versao = primeiro.versao.clone();
        // Total autoritativo: o do GET ou, se ele omitir, o que o HEAD informou.
        // É o que a guarda final e a checagem de completude usam.
        let total_esperado = primeiro.tamanho_total.or(tamanho_informado);
        let chave = local.parte(1, extensao(&tipo_de_conteudo));

        // O fluxo se retoma via Range quando a conexão cai, sem perder nem
        // duplicar byte, entregando ao `gravar_fluxo` um fluxo contínuo.
        let fluxo = FluxoRetomavel::novo(
            contexto.http.clone(),
            self.url.clone(),
            primeiro,
            self.espera_entre_retomadas,
            total_esperado,
        );
        let mut leitor = LeitorComResumo::novo(Box::new(fluxo));

        contexto
            .armazenamento
            .gravar_fluxo(&chave, &tipo_de_conteudo, &mut leitor)
            .await?;

        // Garantia final: só publicamos se o tamanho bater com o esperado.
        if let Some(total) = total_esperado
            && leitor.bytes != total
        {
            return Err(ErroDeIngestao::Http {
                url: self.url.clone(),
                detalhe: format!(
                    "transferência incompleta: {} de {total} bytes",
                    leitor.bytes
                ),
            });
        }

        Ok(ArquivoGravado {
            parte: ParteRaw {
                chave,
                url: self.url.clone(),
                bytes: leitor.bytes,
                sha256: hex::encode(leitor.hash.finalize()),
                tipo_de_conteudo,
                versao: versao.clone(),
            },
            versao,
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
        let informacoes = contexto.http.informacoes(&self.url).await?;
        let estado =
            estado_na_raw(contexto.armazenamento.as_ref(), &self.fonte, &self.recurso).await?;
        let motivo = match decidir(&informacoes, estado.as_ref()) {
            DecisaoDeIngestao::Reaproveitar => {
                return Ok(ResumoDaTarefa {
                    reaproveitada: true,
                    ..ResumoDaTarefa::default()
                });
            }
            DecisaoDeIngestao::Ingerir(motivo) => motivo,
        };

        // A conexão pode cair no meio de um arquivo grande. Nada fica visível na Raw
        // antes do fim da transferência, então basta recomeçar.
        let mut falhas = 0;
        let arquivo = loop {
            match self.transferir(contexto, &local, informacoes.tamanho).await {
                Ok(arquivo) => break arquivo,
                Err(erro) => {
                    if matches!(erro, ErroDeIngestao::Autenticacao { .. }) {
                        return Err(erro);
                    }
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
            motivo: Some(motivo),
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
        // a pré-checagem por tamanho reaproveitar numa próxima rodada. A
        // publicação não deixa uma rodada mais antiga sobrescrever uma mais nova
        // (ver issue #16).
        let ultima = UltimaVersao {
            versao: arquivo.versao,
            manifesto: local.manifesto(),
            iniciada_em: Some(contexto.iniciada_em),
        };
        let publicou = publicar_ponteiro(
            contexto.armazenamento.as_ref(),
            &self.fonte,
            &self.recurso,
            &ultima,
        )
        .await?;
        if !publicou {
            warn!(
                tarefa = %self.nome,
                "outra rodada já publicou um ponteiro mais novo; este manifesto não é o vigente"
            );
        }

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

/// Fluxo de download que se **retoma** via Range quando a conexão cai, sem perder
/// nem duplicar byte: ao cair, pede `bytes=<entregue>-` e continua exatamente do
/// ponto certo, entregando ao `gravar_fluxo` um fluxo contínuo. Se a origem não
/// honrar a retomada (responder `200`, começar noutro offset, ou mudar o ETag/tamanho),
/// devolve erro — e a tarefa recomeça o arquivo do zero, nunca emenda versões diferentes.
struct FluxoRetomavel {
    http: Arc<dyn ClienteHttp>,
    url: String,
    /// Validador (ETag) da primeira resposta; exigido igual em cada retomada.
    versao: Option<String>,
    /// Tamanho total anunciado pela origem, quando conhecido.
    tamanho_total: Option<u64>,
    /// Bytes já entregues ao chamador — é o offset exato de uma retomada.
    entregue: u64,
    /// Quantas reconexões já foram feitas nesta transferência.
    retomadas: u32,
    /// Fluxo atualmente em leitura.
    fluxo: FluxoDeBytes,
    /// Espera base entre retomadas.
    espera: Duration,
    /// Reconexão em andamento (buscando o próximo pedaço via Range).
    reconectando: Option<Reconexao>,
}

/// Uma reconexão em andamento: busca o próximo pedaço do download via Range.
type Reconexao = Pin<Box<dyn Future<Output = Result<DownloadHttp, ErroDeIngestao>> + Send>>;

/// Um ETag fraco (`W/"..."`) não é validador forte: a origem não o aceita no
/// `If-Range` e ele não prova identidade byte a byte, então não serve para retomar.
fn validador_forte(etag: &str) -> bool {
    !etag.trim_start().starts_with("W/")
}

impl FluxoRetomavel {
    fn novo(
        http: Arc<dyn ClienteHttp>,
        url: String,
        primeiro: DownloadHttp,
        espera: Duration,
        tamanho_total: Option<u64>,
    ) -> Self {
        Self {
            http,
            url,
            // Só um ETag forte serve de validador para retomar; sem ele, uma queda
            // recomeça o arquivo do zero (nunca emendamos sem garantia de identidade).
            versao: primeiro.versao.filter(|etag| validador_forte(etag)),
            tamanho_total,
            entregue: 0,
            retomadas: 0,
            fluxo: primeiro.fluxo,
            espera,
            reconectando: None,
        }
    }

    /// Agenda uma reconexão do offset já entregue. Erra se estourar o limite.
    fn agendar_reconexao(&mut self) -> io::Result<()> {
        // Sem validador forte não dá para garantir que a origem serviria o MESMO
        // arquivo numa nova conexão: retomar poderia emendar versões diferentes do
        // mesmo tamanho. Falha, e a tarefa recomeça o arquivo do zero.
        if self.versao.is_none() {
            return Err(io::Error::other(
                "origem sem validador forte (ETag): retomada insegura, recomeçando do zero",
            ));
        }
        self.retomadas += 1;
        if self.retomadas > LIMITE_DE_RETOMADAS {
            return Err(io::Error::other(format!(
                "excedeu {LIMITE_DE_RETOMADAS} retomadas do download"
            )));
        }
        warn!(
            url = %self.url,
            entregue = self.entregue,
            "conexão caiu; retomando o download do offset exato"
        );
        let http = self.http.clone();
        let url = self.url.clone();
        let inicio = self.entregue;
        let versao = self.versao.clone();
        let espera = self.espera.saturating_mul(self.retomadas.min(5));
        self.reconectando = Some(Box::pin(async move {
            tokio::time::sleep(espera).await;
            http.baixar_a_partir_de(&url, inicio, versao.as_deref())
                .await
        }));
        Ok(())
    }
}

impl AsyncRead for FluxoRetomavel {
    fn poll_read(
        self: Pin<&mut Self>,
        contexto: &mut Context<'_>,
        destino: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let este = self.get_mut();
        loop {
            // Reconexão em andamento: espera o novo fluxo e valida a integridade.
            if let Some(futuro) = este.reconectando.as_mut() {
                match futuro.as_mut().poll(contexto) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(download)) => {
                        este.reconectando = None;
                        // A origem precisa recomeçar EXATAMENTE onde paramos.
                        if download.inicio != este.entregue {
                            return Poll::Ready(Err(io::Error::other(format!(
                                "a origem não retomou no offset exato: pedi {}, veio {}",
                                este.entregue, download.inicio
                            ))));
                        }
                        // E precisa ser o MESMO arquivo (ETag/tamanho inalterados).
                        if download.versao != este.versao {
                            return Poll::Ready(Err(io::Error::other(
                                "o arquivo mudou na origem durante o download (ETag divergente)",
                            )));
                        }
                        // Se não conhecíamos o total, adota o que a retomada informa
                        // (passa a valer para a checagem de EOF e a guarda final).
                        if este.tamanho_total.is_none() {
                            este.tamanho_total = download.tamanho_total;
                        }
                        if let (Some(antes), Some(agora)) =
                            (este.tamanho_total, download.tamanho_total)
                            && antes != agora
                        {
                            return Poll::Ready(Err(io::Error::other(
                                "o tamanho do arquivo mudou na origem durante o download",
                            )));
                        }
                        este.fluxo = download.fluxo;
                    }
                    Poll::Ready(Err(_)) => {
                        // Falha ao reconectar: tenta de novo do mesmo offset (até o
                        // limite) antes de desistir e recomeçar o arquivo do zero.
                        este.reconectando = None;
                        este.agendar_reconexao()?;
                    }
                }
            }

            let antes = destino.filled().len();
            match Pin::new(&mut este.fluxo).poll_read(contexto, destino) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Ok(())) => {
                    let lidos = destino.filled().len() - antes;
                    if lidos > 0 {
                        este.entregue += lidos as u64;
                        return Poll::Ready(Ok(()));
                    }
                    // Fim do fluxo atual: só é o fim de verdade se entregamos tudo.
                    match este.tamanho_total {
                        Some(total) if este.entregue < total => este.agendar_reconexao()?,
                        _ => return Poll::Ready(Ok(())),
                    }
                }
                Poll::Ready(Err(_)) => {
                    // Erro de transporte após já termos entregue tudo que a origem
                    // anunciou não é perda: a transferência está completa.
                    if let Some(total) = este.tamanho_total
                        && este.entregue >= total
                    {
                        return Poll::Ready(Ok(()));
                    }
                    este.agendar_reconexao()?;
                }
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use std::collections::{HashMap, VecDeque};
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

    /// HTTP falso que serve `CONTEUDO` por Range e pode cortar a conexão no meio,
    /// simulando as quedas da origem nos arquivos grandes.
    ///
    /// `cortes` é consumido a cada download: `Some(n)` serve só `n` bytes a partir
    /// do offset pedido e depois **cai**; `None` (ou fila vazia) serve até o fim.
    /// Registra em `pedidos` cada offset solicitado, para os testes conferirem que
    /// a retomada começa no ponto exato — sem perder nem duplicar byte.
    struct HttpFalso {
        conteudo: Vec<u8>,
        versao: Mutex<Option<String>>,
        /// ETag devolvido numa retomada (`inicio > 0`): simula mudança no meio.
        versao_na_retomada: Mutex<Option<String>>,
        /// Se `true`, responde sempre do zero (ignora o `Range`).
        ignora_range: bool,
        /// Se `true`, toda requisição falha de imediato (origem indisponível).
        sempre_falha: bool,
        cortes: Mutex<VecDeque<Option<usize>>>,
        pedidos: Mutex<Vec<u64>>,
        downloads: Mutex<u32>,
    }

    impl HttpFalso {
        fn novo(versao: Option<&str>) -> Self {
            Self {
                conteudo: CONTEUDO.to_vec(),
                versao: Mutex::new(versao.map(str::to_owned)),
                versao_na_retomada: Mutex::new(None),
                ignora_range: false,
                sempre_falha: false,
                cortes: Mutex::new(VecDeque::new()),
                pedidos: Mutex::new(Vec::new()),
                downloads: Mutex::new(0),
            }
        }

        /// A cada download, serve `n` bytes (a partir do offset) e cai a conexão.
        fn com_cortes(self, cortes: impl IntoIterator<Item = usize>) -> Self {
            *self.cortes.lock().unwrap() = cortes.into_iter().map(Some).collect();
            self
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
                tamanho: Some(self.conteudo.len() as u64),
            })
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
            *self.downloads.lock().unwrap() += 1;
            self.pedidos.lock().unwrap().push(inicio);

            if self.sempre_falha {
                return Err(ErroDeIngestao::Http {
                    url: url.to_owned(),
                    detalhe: "origem indisponível".into(),
                });
            }

            // A origem pode ignorar o Range e responder do zero.
            let inicio_real = if self.ignora_range {
                0
            } else {
                inicio as usize
            };
            let corte = self.cortes.lock().unwrap().pop_front().flatten();
            let resto = &self.conteudo[inicio_real..];
            // `Some(n)`: serve `n` bytes (limitado ao que resta) e CAI a conexão —
            // mesmo quando `n` cobre o resto, para simular queda após entregar tudo.
            let (dados, cai) = match corte {
                Some(n) => (resto[..n.min(resto.len())].to_vec(), true),
                None => (resto.to_vec(), false),
            };

            // Numa retomada pode mudar o ETag, simulando arquivo alterado no meio.
            let versao = if inicio > 0 {
                self.versao_na_retomada
                    .lock()
                    .unwrap()
                    .clone()
                    .or_else(|| self.versao.lock().unwrap().clone())
            } else {
                self.versao.lock().unwrap().clone()
            };

            Ok(DownloadHttp {
                tipo_de_conteudo: "application/zip".into(),
                versao,
                inicio: inicio_real as u64,
                tamanho_total: Some(self.conteudo.len() as u64),
                fluxo: Box::new(FatiaComFinal { dados, pos: 0, cai }),
            })
        }
    }

    /// Entrega `dados` e, no fim, cai (erro, se `cai`) ou termina (EOF).
    struct FatiaComFinal {
        dados: Vec<u8>,
        pos: usize,
        cai: bool,
    }

    impl AsyncRead for FatiaComFinal {
        fn poll_read(
            self: Pin<&mut Self>,
            _: &mut Context<'_>,
            destino: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let este = self.get_mut();
            if este.pos < este.dados.len() {
                let n = (este.dados.len() - este.pos).min(destino.remaining());
                destino.put_slice(&este.dados[este.pos..este.pos + n]);
                este.pos += n;
                return Poll::Ready(Ok(()));
            }
            if este.cai {
                este.cai = false;
                Poll::Ready(Err(io::Error::other("conexão perdida")))
            } else {
                Poll::Ready(Ok(()))
            }
        }
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
        tarefa.espera_entre_retomadas = Duration::ZERO;
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

    /// O conteúdo gravado na parte única (para conferir integridade byte a byte).
    fn parte_gravada(cenario: &Cenario) -> Vec<u8> {
        let chave = chaves(&cenario.armazenamento, "/parte-00001.zip")[0].clone();
        cenario.armazenamento.objetos.lock().unwrap()[&chave].clone()
    }

    #[tokio::test]
    async fn grava_o_arquivo_sem_alterar_com_manifesto_e_versao() {
        let cenario = cenario(HttpFalso::novo(Some("v1")));

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
        let cenario = cenario(HttpFalso::novo(Some("v1")));

        tarefa().executar(&cenario.contexto).await.unwrap();
        let segunda = tarefa().executar(&cenario.contexto).await.unwrap();

        assert!(segunda.reaproveitada);
        assert_eq!(segunda.bytes, 0);
        assert_eq!(*cenario.http.downloads.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn baixa_de_novo_quando_a_origem_muda() {
        let cenario = cenario(HttpFalso::novo(Some("v1")));

        tarefa().executar(&cenario.contexto).await.unwrap();
        *cenario.http.versao.lock().unwrap() = Some("v2".into());
        let segunda = tarefa().executar(&cenario.contexto).await.unwrap();

        assert!(!segunda.reaproveitada);
        assert_eq!(*cenario.http.downloads.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn sem_validador_reaproveita_quando_o_tamanho_bate() {
        let cenario = cenario(HttpFalso::novo(None));

        tarefa().executar(&cenario.contexto).await.unwrap();
        let segunda = tarefa().executar(&cenario.contexto).await.unwrap();

        assert!(segunda.reaproveitada);
        assert_eq!(*cenario.http.downloads.lock().unwrap(), 1);
        // Sem validador, o ponteiro ainda é gravado para a checagem por tamanho.
        assert!(!chaves(&cenario.armazenamento, "_ultima_versao.json").is_empty());
    }

    #[tokio::test]
    async fn reingere_quando_a_parte_na_raw_foi_truncada() {
        let cenario = cenario(HttpFalso::novo(Some("v1")));
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
        let cenario = cenario(HttpFalso::novo(Some("v1")));
        let antes = tarefa().bytes_a_baixar(&cenario.contexto).await.unwrap();
        assert_eq!(antes, Some(CONTEUDO.len() as u64));

        tarefa().executar(&cenario.contexto).await.unwrap();

        let depois = tarefa().bytes_a_baixar(&cenario.contexto).await.unwrap();
        assert_eq!(depois, Some(0));
    }

    #[tokio::test]
    async fn retoma_do_offset_exato_sem_perder_bytes() {
        // Cai após 12 bytes; a retomada pede `bytes=12-` e completa o arquivo.
        let cenario = cenario(HttpFalso::novo(Some("v1")).com_cortes([12]));

        let resumo = tarefa().executar(&cenario.contexto).await.unwrap();

        assert_eq!(resumo.bytes, CONTEUDO.len() as u64);
        // O arquivo gravado é idêntico ao da origem, byte a byte.
        assert_eq!(parte_gravada(&cenario), CONTEUDO);
        // Retomou exatamente onde parou: offsets 0 e depois 12.
        assert_eq!(*cenario.http.pedidos.lock().unwrap(), vec![0, 12]);
    }

    #[tokio::test]
    async fn retoma_varias_quedas_sem_perder_nem_duplicar_byte() {
        // Três quedas seguidas (em 5, +9, +3 bytes) antes de completar.
        let cenario = cenario(HttpFalso::novo(Some("v1")).com_cortes([5, 9, 3]));

        let resumo = tarefa().executar(&cenario.contexto).await.unwrap();

        assert_eq!(resumo.bytes, CONTEUDO.len() as u64);
        assert_eq!(parte_gravada(&cenario), CONTEUDO);
        // Cada retomada começa no byte seguinte ao último entregue.
        assert_eq!(*cenario.http.pedidos.lock().unwrap(), vec![0, 5, 14, 17]);
    }

    #[tokio::test]
    async fn recomeca_do_zero_quando_a_origem_ignora_o_range() {
        // Cai em 10 bytes, mas a origem ignora o Range (responde do zero). Emendar
        // corromperia o arquivo, então a transferência falha e recomeça do zero.
        let mut http = HttpFalso::novo(Some("v1")).com_cortes([10]);
        http.ignora_range = true;
        let cenario = cenario(http);

        let resumo = tarefa().executar(&cenario.contexto).await.unwrap();

        assert_eq!(parte_gravada(&cenario), CONTEUDO);
        assert_eq!(resumo.bytes, CONTEUDO.len() as u64);
        // Pediu 0, tentou retomar em 10 (recusado), recomeçou do 0.
        assert_eq!(*cenario.http.pedidos.lock().unwrap(), vec![0, 10, 0]);
    }

    #[tokio::test]
    async fn recomeca_quando_o_arquivo_muda_na_origem_no_meio() {
        // Cai em 10 bytes; a retomada traz um ETag diferente (arquivo mudou).
        // Emendar misturaria versões, então recomeça do zero com o arquivo atual.
        let http = HttpFalso::novo(Some("v1")).com_cortes([10]);
        *http.versao_na_retomada.lock().unwrap() = Some("v2".into());
        let cenario = cenario(http);

        let resumo = tarefa().executar(&cenario.contexto).await.unwrap();

        assert_eq!(parte_gravada(&cenario), CONTEUDO);
        assert_eq!(resumo.bytes, CONTEUDO.len() as u64);
        assert_eq!(*cenario.http.pedidos.lock().unwrap(), vec![0, 10, 0]);
    }

    #[tokio::test]
    async fn sem_etag_nao_retoma_e_recomeca_do_zero() {
        // Origem sem validador (ETag): retomar poderia emendar versões diferentes
        // do mesmo tamanho. Ao cair em 10 bytes, recomeça o arquivo do zero em vez
        // de pedir `bytes=10-`.
        let cenario = cenario(HttpFalso::novo(None).com_cortes([10]));

        let resumo = tarefa().executar(&cenario.contexto).await.unwrap();

        assert_eq!(parte_gravada(&cenario), CONTEUDO);
        assert_eq!(resumo.bytes, CONTEUDO.len() as u64);
        // Nunca pediu retomada (offset > 0): só recomeços do zero.
        assert_eq!(*cenario.http.pedidos.lock().unwrap(), vec![0, 0]);
    }

    #[tokio::test]
    async fn conclui_quando_cai_apos_entregar_tudo() {
        // A origem entrega o arquivo inteiro e só então a conexão cai: não é perda,
        // a transferência está completa — não reconecta pedindo `bytes=<total>-`.
        let cenario = cenario(HttpFalso::novo(Some("v1")).com_cortes([CONTEUDO.len()]));

        let resumo = tarefa().executar(&cenario.contexto).await.unwrap();

        assert_eq!(parte_gravada(&cenario), CONTEUDO);
        assert_eq!(resumo.bytes, CONTEUDO.len() as u64);
        assert_eq!(*cenario.http.pedidos.lock().unwrap(), vec![0]);
    }

    #[tokio::test]
    async fn desiste_sem_manifesto_nem_versao_depois_das_tentativas() {
        let mut http = HttpFalso::novo(Some("v1"));
        http.sempre_falha = true;
        let cenario = cenario(http);

        assert!(tarefa().executar(&cenario.contexto).await.is_err());

        assert_eq!(*cenario.http.downloads.lock().unwrap(), TENTATIVAS);
        assert!(chaves(&cenario.armazenamento, "_manifesto.json").is_empty());
        assert!(chaves(&cenario.armazenamento, "_ultima_versao.json").is_empty());
    }
}
