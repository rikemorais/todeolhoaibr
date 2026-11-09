//! Ingestão de APIs HTTP, com ou sem paginação: busca cada página (com requisição
//! condicional `If-None-Match` quando há validador da rodada anterior; incondicional
//! na primeira vez), reaproveita o que não mudou e grava o restante na camada Raw.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use todeolho_nucleo::{
    ContextoDeExecucao, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, MotivoDeIngestao, ObjetoRaw,
    ParteRaw, PrecisaoDaEstimativa, RespostaCondicional, ResumoDaTarefa, Tarefa, UltimaVersao,
    estado_na_raw, manifesto_vigente, publicar_ponteiro,
};
use tracing::warn;

use crate::paginacao::{EstrategiaDePaginacao, SemPaginacao};

/// Uma página resolvida durante a caminhada: o corpo (preciso para seguir a
/// paginação), seus metadados, e se ela pode ser reaproveitada sem reescrever.
struct PaginaResolvida {
    corpo: Vec<u8>,
    tipo_de_conteudo: String,
    versao: Option<String>,
    sha256: String,
    /// `Some` quando a página não mudou: a parte anterior é reaproveitada tal
    /// qual (mesma chave na Raw), sem gravar uma cópia nova.
    reuso: Option<ParteRaw>,
    /// A ingestão precisa reescrever o manifesto por causa desta página (corpo
    /// novo, ou validador atualizado sobre o mesmo corpo).
    mudou: bool,
}

#[derive(Debug)]
enum ErroDePagina {
    JsonInvalido(ErroDeIngestao),
    Outro(ErroDeIngestao),
}

impl From<ErroDeIngestao> for ErroDePagina {
    fn from(erro: ErroDeIngestao) -> Self {
        Self::Outro(erro)
    }
}

impl ErroDePagina {
    fn em_ingestao(self) -> ErroDeIngestao {
        match self {
            Self::JsonInvalido(erro) | Self::Outro(erro) => erro,
        }
    }
}

const ACEITA_JSON: &str = "application/json";
const LIMITE_DE_PAGINAS: usize = 1_000;
const TENTATIVAS_JSON_INVALIDO: u32 = 3;

/// Baixa um recurso HTTP e grava cada resposta, byte a byte, na camada Raw.
pub struct TarefaDeIngestaoHttp {
    fonte: String,
    recurso: String,
    url_inicial: String,
    paginacao: Arc<dyn EstrategiaDePaginacao>,
    limite_de_paginas: usize,
    tamanho_tipico: Option<u64>,
}

impl TarefaDeIngestaoHttp {
    /// Cria uma tarefa paginada, com a estratégia de paginação dada.
    pub fn paginada(
        fonte: &str,
        recurso: &str,
        url_inicial: String,
        paginacao: impl EstrategiaDePaginacao + 'static,
    ) -> Self {
        Self {
            fonte: fonte.to_owned(),
            recurso: recurso.to_owned(),
            url_inicial,
            paginacao: Arc::new(paginacao),
            limite_de_paginas: LIMITE_DE_PAGINAS,
            tamanho_tipico: None,
        }
    }

    /// Cria uma tarefa de uma única página (sem paginação).
    pub fn sem_paginacao(fonte: &str, recurso: &str, url: String) -> Self {
        Self::paginada(fonte, recurso, url, SemPaginacao)
    }

    /// APIs não informam o tamanho de antemão: este valor vem de uma ingestão real.
    pub fn com_tamanho_tipico(mut self, bytes: u64) -> Self {
        self.tamanho_tipico = Some(bytes);
        self
    }

    /// Substitui o limite padrão de páginas para APIs que expandem cada item em
    /// uma consulta adicional. O limite mínimo é uma página.
    pub fn com_limite_de_paginas(mut self, limite: usize) -> Self {
        self.limite_de_paginas = limite.max(1);
        self
    }

    fn erro_de_paginacao(&self, detalhe: String) -> ErroDeIngestao {
        ErroDeIngestao::Paginacao {
            recurso: self.recurso.clone(),
            detalhe,
        }
    }

    /// Verifica apenas a sintaxe de respostas JSON. O corpo continua opaco e é
    /// persistido byte a byte; nenhuma validação de schema é feita aqui.
    fn validar_resposta(
        &self,
        url: &str,
        status: u16,
        tipo_de_conteudo: &str,
        codificacao_de_conteudo: Option<&str>,
        corpo: &[u8],
    ) -> Result<(), ErroDePagina> {
        if matches!(status, 204 | 205) || !eh_json(tipo_de_conteudo) {
            return Ok(());
        }

        // IgnoredAny percorre a sintaxe sem montar uma árvore Value proporcional
        // ao documento. A precisão arbitrária aceita a gramática JSON de números
        // sem exigir que caibam em i64/u64/f64.
        let codificacao = codificacao_de_conteudo.unwrap_or("identity").trim();
        let decodificado = if codificacao.is_empty() || codificacao.eq_ignore_ascii_case("identity")
        {
            None
        } else if codificacao.eq_ignore_ascii_case("gzip") {
            let mut bytes = Vec::new();
            flate2::read::GzDecoder::new(corpo)
                .read_to_end(&mut bytes)
                .map_err(|erro| {
                    ErroDePagina::JsonInvalido(ErroDeIngestao::Http {
                        url: url.to_owned(),
                        detalhe: format!("resposta gzip inválida: {erro}"),
                    })
                })?;
            Some(bytes)
        } else {
            return Err(ErroDePagina::Outro(ErroDeIngestao::Http {
                url: url.to_owned(),
                detalhe: format!("Content-Encoding não suportado: {codificacao}"),
            }));
        };
        let mut deserializador =
            serde_json::Deserializer::from_slice(decodificado.as_deref().unwrap_or(corpo));
        let resultado = serde::de::IgnoredAny::deserialize(&mut deserializador)
            .and_then(|_| deserializador.end());
        resultado.map_err(|erro| {
            ErroDePagina::JsonInvalido(ErroDeIngestao::Http {
                url: url.to_owned(),
                detalhe: format!("resposta JSON inválida: {erro}"),
            })
        })?;
        Ok(())
    }

    async fn gravar_json<T: serde::Serialize>(
        &self,
        contexto: &ContextoDeExecucao,
        chave: String,
        valor: &T,
    ) -> Result<(), ErroDeIngestao> {
        let conteudo =
            serde_json::to_vec_pretty(valor).map_err(|erro| ErroDeIngestao::Manifesto {
                recurso: self.recurso.clone(),
                detalhe: erro.to_string(),
            })?;
        contexto
            .armazenamento
            .gravar(ObjetoRaw {
                chave,
                conteudo,
                tipo_de_conteudo: ACEITA_JSON.to_owned(),
            })
            .await
    }

    /// Resolve o corpo de uma página e decide se ela pode ser reaproveitada.
    ///
    /// Em um 304 valida a parte guardada (tamanho e sha256) antes de confiar
    /// nela; se estiver ausente, truncada ou corrompida, rebaixa a página sem
    /// validador. Em um 200 com corpo idêntico ao guardado, reaproveita a chave
    /// em vez de gravar outra cópia (ver docs/adr/0012).
    async fn resolver_pagina(
        &self,
        contexto: &ContextoDeExecucao,
        url: &str,
        anterior: Option<&ParteRaw>,
    ) -> Result<PaginaResolvida, ErroDePagina> {
        let validador = anterior.and_then(|parte| parte.versao.as_deref());
        match contexto
            .http
            .obter_condicional_com_codificacao(url, ACEITA_JSON, validador)
            .await?
        {
            (RespostaCondicional::NaoModificado, _) => {
                let anterior = anterior.expect("304 só ocorre quando havia validador anterior");
                match contexto.armazenamento.ler(&anterior.chave).await? {
                    Some(corpo)
                        if corpo.len() as u64 == anterior.bytes
                            && hex::encode(Sha256::digest(&corpo)) == anterior.sha256
                            && self
                                .validar_resposta(
                                    url,
                                    200,
                                    &anterior.tipo_de_conteudo,
                                    if corpo.starts_with(&[0x1f, 0x8b]) {
                                        Some("gzip")
                                    } else {
                                        None
                                    },
                                    &corpo,
                                )
                                .is_ok() =>
                    {
                        Ok(PaginaResolvida {
                            corpo,
                            tipo_de_conteudo: anterior.tipo_de_conteudo.clone(),
                            versao: anterior.versao.clone(),
                            sha256: anterior.sha256.clone(),
                            reuso: Some(anterior.clone()),
                            mudou: false,
                        })
                    }
                    // Parte ausente ou que não confere com o manifesto: não dá para
                    // reaproveitar um 304; rebaixa o corpo de verdade e reingere.
                    _ => self.rebaixar(contexto, url).await,
                }
            }
            (
                RespostaCondicional::Modificado {
                    status,
                    tipo_de_conteudo,
                    versao,
                    corpo,
                },
                codificacao_de_conteudo,
            ) => {
                let tipo_validado = if tipo_de_conteudo.trim().is_empty() {
                    anterior
                        .map(|parte| parte.tipo_de_conteudo.as_str())
                        .unwrap_or_default()
                } else {
                    tipo_de_conteudo.as_str()
                };
                self.validar_resposta(
                    url,
                    status,
                    tipo_validado,
                    codificacao_de_conteudo.as_deref(),
                    &corpo,
                )?;
                Ok(self.classificar_modificado(anterior, tipo_de_conteudo, versao, corpo))
            }
        }
    }

    /// Rebaixa a página sem validador (um GET incondicional), usado quando a parte
    /// guardada que um 304 reaproveitaria está ausente ou corrompida.
    async fn rebaixar(
        &self,
        contexto: &ContextoDeExecucao,
        url: &str,
    ) -> Result<PaginaResolvida, ErroDePagina> {
        match contexto
            .http
            .obter_condicional_com_codificacao(url, ACEITA_JSON, None)
            .await?
        {
            (
                RespostaCondicional::Modificado {
                    status,
                    tipo_de_conteudo,
                    versao,
                    corpo,
                },
                codificacao_de_conteudo,
            ) => {
                self.validar_resposta(
                    url,
                    status,
                    &tipo_de_conteudo,
                    codificacao_de_conteudo.as_deref(),
                    &corpo,
                )?;
                Ok(self.classificar_modificado(None, tipo_de_conteudo, versao, corpo))
            }
            (RespostaCondicional::NaoModificado, _) => Err(self
                .erro_de_paginacao(format!("origem respondeu 304 sem validador para {url}"))
                .into()),
        }
    }

    /// Classifica um 200: se o corpo bate com a parte anterior (sha256 e tamanho),
    /// reaproveita a chave — só reescreve o manifesto se o validador mudou.
    fn classificar_modificado(
        &self,
        anterior: Option<&ParteRaw>,
        tipo_de_conteudo: String,
        versao: Option<String>,
        corpo: Vec<u8>,
    ) -> PaginaResolvida {
        let sha256 = hex::encode(Sha256::digest(&corpo));
        if let Some(anterior) = anterior
            && anterior.sha256 == sha256
            && anterior.bytes == corpo.len() as u64
        {
            let versao_final = versao.or_else(|| anterior.versao.clone());
            let tipo_final = if tipo_de_conteudo.trim().is_empty() {
                anterior.tipo_de_conteudo.clone()
            } else {
                tipo_de_conteudo
            };
            let mut parte = anterior.clone();
            let mudou = versao_final != parte.versao || tipo_final != parte.tipo_de_conteudo;
            parte.versao = versao_final.clone();
            parte.tipo_de_conteudo = tipo_final.clone();
            return PaginaResolvida {
                corpo,
                tipo_de_conteudo: tipo_final,
                versao: versao_final,
                sha256,
                reuso: Some(parte),
                mudou,
            };
        }
        PaginaResolvida {
            corpo,
            tipo_de_conteudo,
            versao,
            sha256,
            reuso: None,
            mudou: true,
        }
    }
}

#[async_trait]
impl Tarefa for TarefaDeIngestaoHttp {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn bytes_a_baixar(
        &self,
        contexto: &ContextoDeExecucao,
        _precisao: PrecisaoDaEstimativa,
    ) -> Result<Option<u64>, ErroDeIngestao> {
        // Uma rodada sobre uma ingestão completa e íntegra não baixa dado novo no
        // caso comum (páginas reaproveitadas via requisição condicional), então
        // não pede espaço novo. Só a primeira ingestão — ou uma reparação, quando
        // o estado está ausente/incompleto — usa a estimativa típica.
        let estado =
            estado_na_raw(contexto.armazenamento.as_ref(), &self.fonte, &self.recurso).await?;
        match estado {
            Some(estado) if estado.completa && estado.integra => Ok(Some(0)),
            _ => Ok(self.tamanho_tipico),
        }
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let mut novas = Vec::new();
        let mut manifesto_gravado = false;
        let resultado = self
            .executar_com_rastreamento(contexto, &mut novas, &mut manifesto_gravado)
            .await;
        if resultado.is_err() && !manifesto_gravado {
            for chave in novas {
                if let Err(erro) = contexto.armazenamento.remover(&chave).await {
                    tracing::warn!(%chave, %erro, "não foi possível remover parte Raw órfã");
                }
            }
        }
        resultado
    }
}

impl TarefaDeIngestaoHttp {
    /// Repete somente falhas sintáticas de JSON, que podem resultar de uma
    /// interrupção transitória após uma resposta HTTP 200. Cada tentativa baixa
    /// o corpo novamente; nenhuma gravação acontece antes da validação.
    async fn resolver_pagina_com_retentativas(
        &self,
        contexto: &ContextoDeExecucao,
        url: &str,
        anterior: Option<&ParteRaw>,
    ) -> Result<PaginaResolvida, ErroDeIngestao> {
        for tentativa in 1..=TENTATIVAS_JSON_INVALIDO {
            match self.resolver_pagina(contexto, url, anterior).await {
                Err(ErroDePagina::JsonInvalido(erro)) if tentativa < TENTATIVAS_JSON_INVALIDO => {
                    let espera = std::time::Duration::from_millis(250 * u64::from(tentativa));
                    warn!(%url, %erro, tentativa, ?espera, "resposta JSON inválida; baixando novamente");
                    tokio::time::sleep(espera).await;
                }
                resultado => return resultado.map_err(ErroDePagina::em_ingestao),
            }
        }
        unreachable!("ao menos uma tentativa JSON é feita")
    }

    async fn executar_com_rastreamento(
        &self,
        contexto: &ContextoDeExecucao,
        novas: &mut Vec<String>,
        manifesto_gravado: &mut bool,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let local = LocalizacaoRaw::nova(
            &self.fonte,
            &self.recurso,
            contexto.iniciada_em,
            &contexto.id_execucao,
        );

        // Estado anterior na Raw, para o motivo da reingestão (ausente, incompleta
        // ou desatualizada) refletir o que de fato havia guardado.
        let estado_anterior =
            estado_na_raw(contexto.armazenamento.as_ref(), &self.fonte, &self.recurso).await?;

        // Partes da ingestão anterior, por URL, para a requisição condicional e
        // para reaproveitar, sem reescrever, o que não mudou.
        let anteriores: HashMap<String, ParteRaw> =
            manifesto_vigente(contexto.armazenamento.as_ref(), &self.fonte, &self.recurso)
                .await?
                .map(|manifesto| {
                    manifesto
                        .partes
                        .into_iter()
                        .map(|parte| (parte.url.clone(), parte))
                        .collect()
                })
                .unwrap_or_default();

        // As partes guardam só metadados (pequenos); o corpo de cada página é
        // gravado na hora e liberado, para não acumular toda a paginação na
        // memória do contêiner (ver docs/adr/0012).
        let mut partes: Vec<ParteRaw> = Vec::new();
        let mut visitadas = HashSet::new();
        let mut proxima = Some(self.url_inicial.clone());
        let mut alguma_mudou = false;
        let mut numero = 0usize;

        while let Some(url) = proxima.take() {
            numero += 1;
            if numero > self.limite_de_paginas {
                return Err(self.erro_de_paginacao(format!(
                    "limite de {} páginas excedido",
                    self.limite_de_paginas
                )));
            }
            if !visitadas.insert(url.clone()) {
                return Err(self.erro_de_paginacao(format!("página repetida: {url}")));
            }

            let anterior = anteriores.get(&url);
            let resolvida = self
                .resolver_pagina_com_retentativas(contexto, &url, anterior)
                .await?;

            let corpo_paginacao = if resolvida.corpo.starts_with(&[0x1f, 0x8b]) {
                let mut decodificado = Vec::new();
                flate2::read::GzDecoder::new(resolvida.corpo.as_slice())
                    .read_to_end(&mut decodificado)
                    .map_err(|erro| self.erro_de_paginacao(erro.to_string()))?;
                Some(decodificado)
            } else {
                None
            };
            proxima = self
                .paginacao
                .proxima_url(corpo_paginacao.as_deref().unwrap_or(&resolvida.corpo))
                .map_err(|detalhe| self.erro_de_paginacao(detalhe))?;

            let PaginaResolvida {
                corpo,
                tipo_de_conteudo,
                versao,
                sha256,
                reuso,
                mudou,
            } = resolvida;
            if mudou {
                alguma_mudou = true;
            }

            let parte = match reuso {
                // Página inalterada: reaproveita a parte anterior tal qual (mesma
                // chave), sem gravar uma cópia nova.
                Some(parte) => parte,
                None => {
                    let chave = local.parte(numero, extensao(&tipo_de_conteudo));
                    novas.push(chave.clone());
                    let parte = ParteRaw {
                        chave: chave.clone(),
                        url: url.clone(),
                        bytes: corpo.len() as u64,
                        sha256,
                        tipo_de_conteudo: tipo_de_conteudo.clone(),
                        versao,
                    };
                    contexto
                        .armazenamento
                        .gravar(ObjetoRaw {
                            chave,
                            conteudo: corpo,
                            tipo_de_conteudo,
                        })
                        .await?;
                    parte
                }
            };
            partes.push(parte);
        }

        // Nada mudou e o conjunto de páginas é o mesmo: a ingestão anterior segue
        // válida, nada a reescrever.
        let mesmo_conjunto = partes.len() == anteriores.len()
            && partes
                .iter()
                .all(|parte| anteriores.contains_key(&parte.url));
        if !alguma_mudou && mesmo_conjunto && !partes.is_empty() {
            return Ok(ResumoDaTarefa {
                reaproveitada: true,
                ..ResumoDaTarefa::default()
            });
        }

        let motivo = match &estado_anterior {
            None => MotivoDeIngestao::AusenteNaRaw,
            Some(estado) if !estado.completa || !estado.integra => {
                MotivoDeIngestao::IncompletaOuCorrompida
            }
            Some(_) => MotivoDeIngestao::Desatualizada,
        };
        let resumo = ResumoDaTarefa {
            partes: partes.len(),
            bytes: partes.iter().map(|parte| parte.bytes).sum(),
            reaproveitada: false,
            motivo: Some(motivo),
        };

        let manifesto = ManifestoRaw {
            fonte: self.fonte.clone(),
            recurso: self.recurso.clone(),
            id_execucao: contexto.id_execucao.clone(),
            url_inicial: self.url_inicial.clone(),
            iniciada_em: contexto.iniciada_em,
            concluida_em: Utc::now(),
            partes,
        };
        // A resposta de uma gravação remota pode ser perdida depois do commit;
        // a partir daqui não apagamos partes que um manifesto talvez já referencie.
        *manifesto_gravado = true;
        self.gravar_json(contexto, local.manifesto(), &manifesto)
            .await?;

        // Ponteiro para a próxima rodada localizar este manifesto. Sem validador
        // de recurso: a requisição condicional é por página (ver docs/adr/0012).
        // A publicação não deixa uma rodada mais antiga sobrescrever uma mais
        // nova (ver issue #16).
        let ultima = UltimaVersao {
            versao: None,
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
                recurso = %self.recurso,
                "outra rodada já publicou um ponteiro mais novo; este manifesto não é o vigente"
            );
        }

        Ok(resumo)
    }
}

pub(crate) fn extensao(tipo_de_conteudo: &str) -> &'static str {
    let tipo = tipo_de_conteudo.to_ascii_lowercase();
    if tipo.contains("json") {
        "json"
    } else if tipo.contains("xml") {
        "xml"
    } else if tipo.contains("zip") {
        "zip"
    } else if tipo.contains("csv") {
        "csv"
    } else {
        "bin"
    }
}

fn eh_json(tipo_de_conteudo: &str) -> bool {
    let tipo = tipo_de_conteudo
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    tipo == "application/json"
        || tipo
            .split_once('/')
            .is_some_and(|(_, subtipo)| subtipo.ends_with("+json"))
}

#[cfg(test)]
mod testes {
    use std::collections::{HashMap, VecDeque};
    use std::io::Write;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use todeolho_nucleo::{
        ArmazenamentoRaw, ClienteHttp, DownloadHttp, InformacoesDoArquivo, RespostaCondicional,
        RespostaHttp,
    };

    use super::*;
    use crate::paginacao::LinksDePaginacao;

    #[derive(Default)]
    struct ArmazenamentoEmMemoria {
        objetos: Mutex<Vec<ObjetoRaw>>,
    }

    #[async_trait]
    impl ArmazenamentoRaw for ArmazenamentoEmMemoria {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }

        async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            self.objetos.lock().unwrap().push(objeto);
            Ok(())
        }

        async fn remover(&self, chave: &str) -> Result<(), ErroDeIngestao> {
            self.objetos
                .lock()
                .unwrap()
                .retain(|objeto| objeto.chave != chave);
            Ok(())
        }

        async fn gravar_fluxo(
            &self,
            _: &str,
            _: &str,
            _: &mut (dyn tokio::io::AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            unreachable!("a ingestão paginada grava respostas em memória")
        }

        async fn ler(&self, _: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(None)
        }
    }

    struct HttpFalso(HashMap<&'static str, &'static str>);

    struct HttpCodificado {
        corpo: Vec<u8>,
        codificacao: &'static str,
    }

    #[async_trait]
    impl ClienteHttp for HttpCodificado {
        async fn obter(&self, _: &str, _: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            unreachable!("usa GET condicional")
        }

        async fn obter_condicional_com_codificacao(
            &self,
            _: &str,
            _: &str,
            _: Option<&str>,
        ) -> Result<(RespostaCondicional, Option<String>), ErroDeIngestao> {
            Ok((
                RespostaCondicional::Modificado {
                    status: 200,
                    tipo_de_conteudo: "application/json".into(),
                    versao: None,
                    corpo: self.corpo.clone(),
                },
                Some(self.codificacao.into()),
            ))
        }

        async fn informacoes(&self, _: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            unreachable!("não consulta arquivos")
        }

        async fn baixar(&self, _: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            unreachable!("não baixa arquivos")
        }
    }

    struct HttpSequencial {
        respostas: Mutex<VecDeque<&'static str>>,
        chamadas: AtomicUsize,
    }

    #[async_trait]
    impl ClienteHttp for HttpSequencial {
        async fn informacoes(&self, _: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            unreachable!("a ingestão paginada não consulta arquivos")
        }

        async fn baixar(&self, _: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            unreachable!("a ingestão paginada grava respostas em memória")
        }

        async fn obter(&self, url: &str, _: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            self.chamadas.fetch_add(1, Ordering::SeqCst);
            let corpo =
                self.respostas
                    .lock()
                    .unwrap()
                    .pop_front()
                    .ok_or_else(|| ErroDeIngestao::Http {
                        url: url.into(),
                        detalhe: "503: sem resposta de teste restante".into(),
                    })?;
            Ok(RespostaHttp {
                tipo_de_conteudo: "application/json; charset=utf-8".into(),
                corpo: corpo.as_bytes().to_vec(),
            })
        }
    }

    #[async_trait]
    impl ClienteHttp for HttpFalso {
        async fn informacoes(&self, _: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            unreachable!("a ingestão paginada não consulta arquivos")
        }

        async fn baixar(&self, _: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            unreachable!("a ingestão paginada grava respostas em memória")
        }

        async fn obter(&self, url: &str, _: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            self.0
                .get(url)
                .map(|corpo| RespostaHttp {
                    tipo_de_conteudo: "application/json; charset=utf-8".into(),
                    corpo: corpo.as_bytes().to_vec(),
                })
                .ok_or_else(|| ErroDeIngestao::Http {
                    url: url.into(),
                    detalhe: "503".into(),
                })
        }
    }

    fn contexto(
        respostas: &[(&'static str, &'static str)],
    ) -> (ContextoDeExecucao, Arc<ArmazenamentoEmMemoria>) {
        let armazenamento = Arc::new(ArmazenamentoEmMemoria::default());
        let contexto = ContextoDeExecucao {
            id_execucao: "exec".into(),
            iniciada_em: Utc::now(),
            armazenamento: armazenamento.clone(),
            http: Arc::new(HttpFalso(respostas.iter().copied().collect())),
            progresso: None,
        };
        (contexto, armazenamento)
    }

    fn contexto_com_respostas_sequenciais(
        respostas: &[&'static str],
    ) -> (
        ContextoDeExecucao,
        Arc<ArmazenamentoEmMemoria>,
        Arc<HttpSequencial>,
    ) {
        let armazenamento = Arc::new(ArmazenamentoEmMemoria::default());
        let http = Arc::new(HttpSequencial {
            respostas: Mutex::new(respostas.iter().copied().collect()),
            chamadas: AtomicUsize::new(0),
        });
        let contexto = ContextoDeExecucao {
            id_execucao: "exec".into(),
            iniciada_em: Utc::now(),
            armazenamento: armazenamento.clone(),
            http: http.clone(),
            progresso: None,
        };
        (contexto, armazenamento, http)
    }

    fn contexto_codificado(
        corpo: Vec<u8>,
        codificacao: &'static str,
    ) -> (ContextoDeExecucao, Arc<ArmazenamentoEmMemoria>) {
        let armazenamento = Arc::new(ArmazenamentoEmMemoria::default());
        let contexto = ContextoDeExecucao {
            id_execucao: "exec".into(),
            iniciada_em: Utc::now(),
            armazenamento: armazenamento.clone(),
            http: Arc::new(HttpCodificado { corpo, codificacao }),
            progresso: None,
        };
        (contexto, armazenamento)
    }

    fn tarefa_paginada() -> TarefaDeIngestaoHttp {
        TarefaDeIngestaoHttp::paginada("f", "r", "https://api/1".into(), LinksDePaginacao)
    }

    const PAGINA_1: &str = r#"{"dados": [1],  "links":[{"rel":"next","href":"https://api/2"}]}"#;
    const PAGINA_2: &str = r#"{"dados":[2],"links":[]}"#;

    #[tokio::test]
    async fn grava_cada_pagina_sem_alterar_e_termina_com_o_manifesto() {
        let (contexto, armazenamento) =
            contexto(&[("https://api/1", PAGINA_1), ("https://api/2", PAGINA_2)]);

        let resumo = tarefa_paginada().executar(&contexto).await.unwrap();

        assert_eq!(resumo.partes, 2);
        assert_eq!(resumo.bytes, (PAGINA_1.len() + PAGINA_2.len()) as u64);

        let objetos = armazenamento.objetos.lock().unwrap();
        assert_eq!(objetos.len(), 4);
        assert_eq!(objetos[0].conteudo, PAGINA_1.as_bytes());
        assert!(objetos[0].chave.ends_with("/parte-00001.json"));
        assert!(objetos[1].chave.ends_with("/parte-00002.json"));
        assert!(objetos[2].chave.ends_with("/_manifesto.json"));
        assert!(objetos[3].chave.ends_with("/_ultima_versao.json"));

        let manifesto: ManifestoRaw = serde_json::from_slice(&objetos[2].conteudo).unwrap();
        assert_eq!(manifesto.partes.len(), 2);
        assert_eq!(manifesto.partes[1].url, "https://api/2");
        assert_eq!(
            manifesto.partes[0].sha256,
            hex::encode(Sha256::digest(PAGINA_1))
        );
    }

    #[tokio::test]
    async fn sem_manifesto_quando_uma_pagina_falha() {
        let (contexto, armazenamento) = contexto(&[("https://api/1", PAGINA_1)]);

        let erro = tarefa_paginada().executar(&contexto).await.unwrap_err();
        assert!(matches!(
            erro,
            ErroDeIngestao::Http { ref url, .. } if url == "https://api/2"
        ));

        let objetos = armazenamento.objetos.lock().unwrap();
        assert!(
            objetos
                .iter()
                .all(|o| !o.chave.ends_with("_manifesto.json"))
        );
        assert!(
            objetos.is_empty(),
            "partes novas devem ser removidas no rollback"
        );
    }

    #[tokio::test]
    async fn nao_publica_resposta_json_truncada() {
        const CORPO_PARCIAL: &str = r#"{"dados":[{"id":1}"#;
        let (contexto, armazenamento) = contexto(&[("https://api/estados", CORPO_PARCIAL)]);
        let tarefa =
            TarefaDeIngestaoHttp::sem_paginacao("ibge", "estados", "https://api/estados".into());

        let resultado = tarefa.executar(&contexto).await;

        assert!(
            resultado.is_err(),
            "a resposta parcial foi aceita como completa"
        );
        assert!(
            armazenamento.objetos.lock().unwrap().is_empty(),
            "a resposta parcial não deve publicar parte, manifesto ou ponteiro"
        );
    }

    fn gzip(corpo: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(corpo).unwrap();
        encoder.finish().unwrap()
    }

    #[tokio::test]
    async fn valida_gzip_e_preserva_bytes_recebidos_na_raw() {
        let corpo = gzip(br#"{"dados":[1]}"#);
        let (contexto, armazenamento) = contexto_codificado(corpo.clone(), "gzip");
        let tarefa = TarefaDeIngestaoHttp::sem_paginacao("f", "r", "https://api/r".into());

        tarefa.executar(&contexto).await.unwrap();

        let objetos = armazenamento.objetos.lock().unwrap();
        assert_eq!(objetos[0].conteudo, corpo);
        assert!(objetos[1].chave.ends_with("/_manifesto.json"));
    }

    #[tokio::test]
    async fn paginacao_le_json_gzip_decodificado() {
        let corpo = gzip(br#"{"dados":[1],"links":[]}"#);
        let (contexto, armazenamento) = contexto_codificado(corpo.clone(), "gzip");
        let tarefa =
            TarefaDeIngestaoHttp::paginada("f", "r", "https://api/r".into(), LinksDePaginacao);

        let resumo = tarefa.executar(&contexto).await.unwrap();

        assert_eq!(resumo.partes, 1);
        assert_eq!(armazenamento.objetos.lock().unwrap()[0].conteudo, corpo);
    }

    #[tokio::test]
    async fn rejeita_gzip_truncado_sem_publicar() {
        let mut corpo = gzip(br#"{"dados":[1]}"#);
        corpo.truncate(corpo.len() - 4);
        let (contexto, armazenamento) = contexto_codificado(corpo, "gzip");
        let tarefa = TarefaDeIngestaoHttp::sem_paginacao("f", "r", "https://api/r".into());

        assert!(tarefa.executar(&contexto).await.is_err());
        assert!(armazenamento.objetos.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejeita_content_encoding_desconhecido_sem_publicar() {
        let (contexto, armazenamento) = contexto_codificado(br#"{"dados":[1]}"#.to_vec(), "br");
        let tarefa = TarefaDeIngestaoHttp::sem_paginacao("f", "r", "https://api/r".into());

        assert!(tarefa.executar(&contexto).await.is_err());
        assert!(armazenamento.objetos.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn repete_json_truncado_e_publica_a_resposta_valida() {
        const CORPO_PARCIAL: &str = r#"{"dados":[{"id":1}"#;
        const CORPO_VALIDO: &str = r#"{"dados":[{"id":1}]}"#;
        let (contexto, armazenamento, http) =
            contexto_com_respostas_sequenciais(&[CORPO_PARCIAL, CORPO_VALIDO]);
        let tarefa =
            TarefaDeIngestaoHttp::sem_paginacao("ibge", "estados", "https://api/estados".into());

        tarefa.executar(&contexto).await.unwrap();

        assert_eq!(http.chamadas.load(Ordering::SeqCst), 2);
        let objetos = armazenamento.objetos.lock().unwrap();
        assert_eq!(objetos.len(), 3);
        assert_eq!(objetos[0].conteudo, CORPO_VALIDO.as_bytes());
        assert!(objetos[1].chave.ends_with("/_manifesto.json"));
    }

    #[tokio::test]
    async fn interrompe_apos_o_limite_de_json_truncado_sem_publicar() {
        const CORPO_PARCIAL: &str = r#"{"dados":[{"id":1}"#;
        let (contexto, armazenamento, http) =
            contexto_com_respostas_sequenciais(&[CORPO_PARCIAL, CORPO_PARCIAL, CORPO_PARCIAL]);
        let tarefa =
            TarefaDeIngestaoHttp::sem_paginacao("ibge", "estados", "https://api/estados".into());

        let erro = tarefa.executar(&contexto).await.unwrap_err();

        assert!(matches!(erro, ErroDeIngestao::Http { .. }));
        assert_eq!(
            http.chamadas.load(Ordering::SeqCst),
            TENTATIVAS_JSON_INVALIDO as usize
        );
        assert!(armazenamento.objetos.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn remove_partes_anteriores_quando_pagina_seguinte_tem_json_invalido() {
        let (contexto, armazenamento) = contexto(&[
            ("https://api/1", PAGINA_1),
            ("https://api/2", r#"{"dados":["#),
        ]);

        assert!(tarefa_paginada().executar(&contexto).await.is_err());
        assert!(armazenamento.objetos.lock().unwrap().is_empty());
    }

    #[test]
    fn reconhece_json_por_media_type_sem_interpretar_outros_formatos() {
        assert!(eh_json("application/json; charset=utf-8"));
        assert!(eh_json("application/problem+json"));
        assert!(eh_json("application/vnd.api+json"));
        assert!(!eh_json("text/plain; charset=utf-8"));
    }

    #[test]
    fn valida_json_sem_materializar_dom_e_aceita_numero_fora_da_faixa_f64() {
        let tarefa = TarefaDeIngestaoHttp::sem_paginacao("f", "r", "https://api/r".into());

        tarefa
            .validar_resposta("https://api/r", 200, "application/json", None, b"1e400")
            .unwrap();
        assert!(
            tarefa
                .validar_resposta("https://api/r", 200, "application/json", None, b"{\"a\":")
                .is_err()
        );
    }

    #[test]
    fn resposta_sem_corpo_e_valida_so_em_status_sem_conteudo() {
        let tarefa = TarefaDeIngestaoHttp::sem_paginacao("f", "r", "https://api/r".into());

        tarefa
            .validar_resposta("https://api/r", 204, "application/json", None, b"")
            .unwrap();
        assert!(
            tarefa
                .validar_resposta("https://api/r", 200, "application/json", None, b"")
                .is_err()
        );
    }

    #[test]
    fn resposta_identica_atualiza_tipo_de_conteudo_retido() {
        let tarefa = TarefaDeIngestaoHttp::sem_paginacao("f", "r", "https://api/r".into());
        let corpo = b"{malformado";
        let anterior = ParteRaw {
            chave: "parte.json".into(),
            url: "https://api/r".into(),
            bytes: corpo.len() as u64,
            sha256: hex::encode(Sha256::digest(corpo)),
            tipo_de_conteudo: "application/json".into(),
            versao: Some("v1".into()),
        };

        let resolvida = tarefa.classificar_modificado(
            Some(&anterior),
            "text/plain".into(),
            Some("v2".into()),
            corpo.to_vec(),
        );

        assert!(resolvida.mudou);
        assert_eq!(resolvida.tipo_de_conteudo, "text/plain");
        assert_eq!(resolvida.reuso.unwrap().tipo_de_conteudo, "text/plain");
    }

    #[tokio::test]
    async fn detecta_paginacao_em_loop() {
        const EM_LOOP: &str = r#"{"links":[{"rel":"next","href":"https://api/1"}]}"#;
        let (contexto, _) = contexto(&[("https://api/1", EM_LOOP)]);

        let erro = tarefa_paginada().executar(&contexto).await.unwrap_err();

        assert!(matches!(erro, ErroDeIngestao::Paginacao { .. }));
    }

    #[tokio::test]
    async fn respeita_o_limite_de_paginas() {
        let (contexto, _) = contexto(&[("https://api/1", PAGINA_1), ("https://api/2", PAGINA_2)]);
        let mut tarefa = tarefa_paginada();
        tarefa.limite_de_paginas = 1;

        let erro = tarefa.executar(&contexto).await.unwrap_err();

        assert!(matches!(erro, ErroDeIngestao::Paginacao { .. }));
    }

    const UNICA: &str = r#"{"dados":[1]}"#;

    /// Armazenamento com leitura funcional, para a requisição condicional reusar
    /// a parte guardada numa segunda rodada.
    #[derive(Default)]
    struct ArmazenamentoMapa {
        objetos: Mutex<HashMap<String, Vec<u8>>>,
    }

    #[async_trait]
    impl ArmazenamentoRaw for ArmazenamentoMapa {
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

        async fn gravar_fluxo(
            &self,
            _: &str,
            _: &str,
            _: &mut (dyn tokio::io::AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            unreachable!("a ingestão paginada não grava fluxo")
        }

        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.objetos.lock().unwrap().get(chave).cloned())
        }
    }

    /// Entrega o corpo com ETag `v1` na primeira vez e responde 304 quando o
    /// validador `v1` volta na requisição condicional.
    #[derive(Default)]
    struct HttpCondicional {
        chamadas: Mutex<u32>,
        validadores: Mutex<Vec<Option<String>>>,
    }

    #[async_trait]
    impl ClienteHttp for HttpCondicional {
        async fn obter(&self, _: &str, _: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            unreachable!("usa obter_condicional")
        }

        async fn informacoes(&self, _: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            unreachable!("a ingestão paginada não consulta arquivos")
        }

        async fn baixar(&self, _: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            unreachable!("a ingestão paginada não baixa arquivos")
        }

        async fn obter_condicional(
            &self,
            _: &str,
            _: &str,
            validador: Option<&str>,
        ) -> Result<RespostaCondicional, ErroDeIngestao> {
            *self.chamadas.lock().unwrap() += 1;
            self.validadores
                .lock()
                .unwrap()
                .push(validador.map(str::to_owned));
            if validador == Some("v1") {
                Ok(RespostaCondicional::NaoModificado)
            } else {
                Ok(RespostaCondicional::Modificado {
                    status: 200,
                    tipo_de_conteudo: "application/json".to_owned(),
                    versao: Some("v1".to_owned()),
                    corpo: UNICA.as_bytes().to_vec(),
                })
            }
        }
    }

    #[tokio::test]
    async fn reaproveita_a_pagina_que_nao_mudou() {
        let armazenamento = Arc::new(ArmazenamentoMapa::default());
        let http = Arc::new(HttpCondicional::default());
        let contexto = ContextoDeExecucao {
            id_execucao: "e1".into(),
            iniciada_em: Utc::now(),
            armazenamento: armazenamento.clone(),
            http: http.clone(),
            progresso: None,
        };
        let tarefa =
            TarefaDeIngestaoHttp::sem_paginacao("ibge", "estados", "https://api/estados".into());

        let primeira = tarefa.executar(&contexto).await.unwrap();
        assert!(!primeira.reaproveitada);

        let segunda = tarefa.executar(&contexto).await.unwrap();
        assert!(segunda.reaproveitada);
        // Uma chamada por rodada: a segunda recebeu 304 e não rebaixou.
        assert_eq!(*http.chamadas.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn rebaixa_quando_o_304_reaproveitaria_parte_corrompida() {
        let armazenamento = Arc::new(ArmazenamentoMapa::default());
        let http = Arc::new(HttpCondicional::default());
        let contexto = ContextoDeExecucao {
            id_execucao: "e1".into(),
            iniciada_em: Utc::now(),
            armazenamento: armazenamento.clone(),
            http: http.clone(),
            progresso: None,
        };
        let tarefa =
            TarefaDeIngestaoHttp::sem_paginacao("ibge", "estados", "https://api/estados".into());

        tarefa.executar(&contexto).await.unwrap();

        // Corrompe a parte guardada: o 304 não pode reaproveitá-la como válida.
        {
            let mut objetos = armazenamento.objetos.lock().unwrap();
            let chave = objetos
                .keys()
                .find(|c| c.ends_with("/parte-00001.json"))
                .cloned()
                .unwrap();
            objetos.insert(chave, b"corrompido".to_vec());
        }

        let segunda = tarefa.executar(&contexto).await.unwrap();

        assert!(!segunda.reaproveitada);
        assert_eq!(
            segunda.motivo,
            Some(MotivoDeIngestao::IncompletaOuCorrompida)
        );
        // Rodada 1: 1 chamada. Rodada 2: 304 + rebaixamento sem validador.
        assert_eq!(*http.chamadas.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn rebaixa_quando_parte_json_do_304_tem_sintaxe_invalida() {
        let armazenamento = Arc::new(ArmazenamentoMapa::default());
        let http = Arc::new(HttpCondicional::default());
        let contexto = ContextoDeExecucao {
            id_execucao: "e1".into(),
            iniciada_em: Utc::now(),
            armazenamento: armazenamento.clone(),
            http: http.clone(),
            progresso: None,
        };
        let tarefa =
            TarefaDeIngestaoHttp::sem_paginacao("ibge", "estados", "https://api/estados".into());
        let corpo = b"{\"dados\":[";
        let chave = "parte.json".to_owned();
        armazenamento
            .gravar(ObjetoRaw {
                chave: chave.clone(),
                conteudo: corpo.to_vec(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        let anterior = ParteRaw {
            chave,
            url: "https://api/estados".into(),
            bytes: corpo.len() as u64,
            sha256: hex::encode(Sha256::digest(corpo)),
            tipo_de_conteudo: "application/json".into(),
            versao: Some("v1".into()),
        };

        let resolvida = tarefa
            .resolver_pagina(&contexto, "https://api/estados", Some(&anterior))
            .await
            .unwrap();

        assert_eq!(resolvida.corpo, UNICA.as_bytes());
        assert_eq!(*http.chamadas.lock().unwrap(), 2);
        assert_eq!(
            *http.validadores.lock().unwrap(),
            vec![Some("v1".to_owned()), None]
        );
    }

    /// Sempre responde 200 com o mesmo corpo e sem validador (não suporta
    /// requisição condicional).
    #[derive(Default)]
    struct HttpSempre200 {
        chamadas: Mutex<u32>,
    }

    #[async_trait]
    impl ClienteHttp for HttpSempre200 {
        async fn obter(&self, _: &str, _: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            unreachable!("usa obter_condicional")
        }

        async fn informacoes(&self, _: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            unreachable!("a ingestão paginada não consulta arquivos")
        }

        async fn baixar(&self, _: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            unreachable!("a ingestão paginada não baixa arquivos")
        }

        async fn obter_condicional(
            &self,
            _: &str,
            _: &str,
            _: Option<&str>,
        ) -> Result<RespostaCondicional, ErroDeIngestao> {
            *self.chamadas.lock().unwrap() += 1;
            Ok(RespostaCondicional::Modificado {
                status: 200,
                tipo_de_conteudo: "application/json".to_owned(),
                versao: None,
                corpo: UNICA.as_bytes().to_vec(),
            })
        }
    }

    #[tokio::test]
    async fn nao_reescreve_pagina_identica_sem_validador() {
        let armazenamento = Arc::new(ArmazenamentoMapa::default());
        let http = Arc::new(HttpSempre200::default());
        let contexto = ContextoDeExecucao {
            id_execucao: "e1".into(),
            iniciada_em: Utc::now(),
            armazenamento: armazenamento.clone(),
            http: http.clone(),
            progresso: None,
        };
        let tarefa =
            TarefaDeIngestaoHttp::sem_paginacao("ibge", "estados", "https://api/estados".into());

        tarefa.executar(&contexto).await.unwrap();
        let apos_primeira = armazenamento.objetos.lock().unwrap().len();

        let segunda = tarefa.executar(&contexto).await.unwrap();

        // O corpo é idêntico (mesmo sha): reaproveita sem gravar cópia nova nem
        // reescrever o manifesto.
        assert!(segunda.reaproveitada);
        assert_eq!(armazenamento.objetos.lock().unwrap().len(), apos_primeira);
        assert_eq!(*http.chamadas.lock().unwrap(), 2);
    }
}
