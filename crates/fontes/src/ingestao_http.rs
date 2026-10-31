use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sha2::{Digest, Sha256};
use todeolho_nucleo::{
    ContextoDeExecucao, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw,
    RespostaCondicional, ResumoDaTarefa, Tarefa, UltimaVersao, manifesto_vigente,
};

use crate::paginacao::{EstrategiaDePaginacao, SemPaginacao};

/// Uma página lida durante a caminhada, antes de gravar a nova execução.
struct ParteColhida {
    url: String,
    tipo_de_conteudo: String,
    versao: Option<String>,
    corpo: Vec<u8>,
}

const ACEITA_JSON: &str = "application/json";
const LIMITE_DE_PAGINAS: usize = 1_000;

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

    pub fn sem_paginacao(fonte: &str, recurso: &str, url: String) -> Self {
        Self::paginada(fonte, recurso, url, SemPaginacao)
    }

    /// APIs não informam o tamanho de antemão: este valor vem de uma ingestão real.
    pub fn com_tamanho_tipico(mut self, bytes: u64) -> Self {
        self.tamanho_tipico = Some(bytes);
        self
    }

    fn erro_de_paginacao(&self, detalhe: String) -> ErroDeIngestao {
        ErroDeIngestao::Paginacao {
            recurso: self.recurso.clone(),
            detalhe,
        }
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
}

#[async_trait]
impl Tarefa for TarefaDeIngestaoHttp {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn bytes_a_baixar(&self, _: &ContextoDeExecucao) -> Result<Option<u64>, ErroDeIngestao> {
        Ok(self.tamanho_tipico)
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

        // Partes da ingestão anterior, por URL, para a requisição condicional.
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

        let mut colhidas: Vec<ParteColhida> = Vec::new();
        let mut visitadas = HashSet::new();
        let mut proxima = Some(self.url_inicial.clone());
        let mut tudo_reaproveitado = true;

        while let Some(url) = proxima.take() {
            if colhidas.len() >= self.limite_de_paginas {
                return Err(self.erro_de_paginacao(format!(
                    "limite de {} páginas excedido",
                    self.limite_de_paginas
                )));
            }
            if !visitadas.insert(url.clone()) {
                return Err(self.erro_de_paginacao(format!("página repetida: {url}")));
            }

            let anterior = anteriores.get(&url);
            let validador = anterior.and_then(|parte| parte.versao.as_deref());

            let colhida = match contexto
                .http
                .obter_condicional(&url, ACEITA_JSON, validador)
                .await?
            {
                RespostaCondicional::NaoModificado => {
                    // 304 não traz corpo: releio o guardado para seguir a paginação.
                    let anterior = anterior.expect("304 só ocorre quando havia validador anterior");
                    let corpo = contexto
                        .armazenamento
                        .ler(&anterior.chave)
                        .await?
                        .ok_or_else(|| ErroDeIngestao::Armazenamento {
                            chave: anterior.chave.clone(),
                            detalhe: "parte reaproveitada não encontrada na Raw".to_owned(),
                        })?;
                    ParteColhida {
                        url,
                        tipo_de_conteudo: anterior.tipo_de_conteudo.clone(),
                        versao: anterior.versao.clone(),
                        corpo,
                    }
                }
                RespostaCondicional::Modificado {
                    tipo_de_conteudo,
                    versao,
                    corpo,
                } => {
                    tudo_reaproveitado = false;
                    ParteColhida {
                        url,
                        tipo_de_conteudo,
                        versao,
                        corpo,
                    }
                }
            };

            proxima = self
                .paginacao
                .proxima_url(&colhida.corpo)
                .map_err(|detalhe| self.erro_de_paginacao(detalhe))?;
            colhidas.push(colhida);
        }

        // Nenhuma página mudou: reaproveita a ingestão anterior sem reescrever.
        if tudo_reaproveitado && !colhidas.is_empty() {
            return Ok(ResumoDaTarefa {
                reaproveitada: true,
                ..ResumoDaTarefa::default()
            });
        }

        let mut partes = Vec::with_capacity(colhidas.len());
        for (indice, colhida) in colhidas.into_iter().enumerate() {
            let chave = local.parte(indice + 1, extensao(&colhida.tipo_de_conteudo));
            partes.push(ParteRaw {
                chave: chave.clone(),
                url: colhida.url,
                bytes: colhida.corpo.len() as u64,
                sha256: hex::encode(Sha256::digest(&colhida.corpo)),
                tipo_de_conteudo: colhida.tipo_de_conteudo.clone(),
                versao: colhida.versao,
            });
            contexto
                .armazenamento
                .gravar(ObjetoRaw {
                    chave,
                    conteudo: colhida.corpo,
                    tipo_de_conteudo: colhida.tipo_de_conteudo,
                })
                .await?;
        }

        let resumo = ResumoDaTarefa {
            partes: partes.len(),
            bytes: partes.iter().map(|parte| parte.bytes).sum(),
            reaproveitada: false,
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
        self.gravar_json(contexto, local.manifesto(), &manifesto)
            .await?;

        // Ponteiro para a próxima rodada localizar este manifesto. Sem validador
        // de recurso: a requisição condicional é por página (ver docs/adr/0012).
        let ultima = UltimaVersao {
            versao: None,
            manifesto: local.manifesto(),
        };
        self.gravar_json(
            contexto,
            LocalizacaoRaw::ultima_versao(&self.fonte, &self.recurso),
            &ultima,
        )
        .await?;

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

#[cfg(test)]
mod testes {
    use std::collections::HashMap;
    use std::sync::Mutex;

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

        assert!(tarefa_paginada().executar(&contexto).await.is_err());

        let objetos = armazenamento.objetos.lock().unwrap();
        assert!(
            objetos
                .iter()
                .all(|o| !o.chave.ends_with("_manifesto.json"))
        );
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
            if validador == Some("v1") {
                Ok(RespostaCondicional::NaoModificado)
            } else {
                Ok(RespostaCondicional::Modificado {
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
}
