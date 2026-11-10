//! Monta o [`EstadoNaRaw`] de um recurso: localiza o manifesto vigente pelo
//! ponteiro `_ultima_versao.json`, confere completude e integridade das partes.
//!
//! Diferente de [`crate::decisao`], que é regra pura, aqui há I/O, sempre pelo
//! porto [`ArmazenamentoRaw`]. O resultado alimenta [`crate::decisao::decidir`].

use crate::decisao::EstadoNaRaw;
use crate::erros::ErroDeIngestao;
use crate::portas::{ArmazenamentoRaw, ObjetoRaw};
use crate::raw::{LocalizacaoRaw, ManifestoRaw, UltimaVersao};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

type Trava = Arc<tokio::sync::Mutex<()>>;
static TRAVAS: OnceLock<Mutex<HashMap<String, Trava>>> = OnceLock::new();

/// Guarda a publicação de um recurso dentro deste processo, inclusive a
/// conferência final da Bronze antes de publicar sua Silver.
pub struct GuardaPublicacao {
    _guarda: tokio::sync::OwnedMutexGuard<()>,
}

/// Serializa somente tarefas da mesma fonte e recurso neste processo.
pub async fn bloquear_publicacao(fonte: &str, recurso: &str) -> GuardaPublicacao {
    let chave = format!("{fonte}/{recurso}");
    let trava = {
        let mut travas = TRAVAS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap();
        Arc::clone(
            travas
                .entry(chave)
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(()))),
        )
    };
    GuardaPublicacao {
        _guarda: trava.lock_owned().await,
    }
}

/// Lê o estado de um recurso na Raw.
///
/// `None` quando não há ingestão registrada (sem ponteiro `_ultima_versao.json`).
/// Quando o ponteiro aponta para um manifesto que não existe, devolve um estado
/// incompleto, para que a decisão reingira.
pub async fn estado_na_raw(
    armazenamento: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
) -> Result<Option<EstadoNaRaw>, ErroDeIngestao> {
    let chave_ponteiro = LocalizacaoRaw::ultima_versao(fonte, recurso);
    let Some(bytes) = armazenamento.ler(&chave_ponteiro).await? else {
        return Ok(None);
    };
    // Ponteiro ilegível não pode travar o recurso: trata como estado incompleto
    // para que a decisão reingira e o sobrescreva (ver issue #13).
    let Ok(ponteiro) = serde_json::from_slice::<UltimaVersao>(&bytes) else {
        return Ok(Some(EstadoNaRaw::incompleto(None)));
    };

    let Some(bytes_manifesto) = armazenamento.ler(&ponteiro.manifesto).await? else {
        return Ok(Some(EstadoNaRaw::incompleto(ponteiro.versao)));
    };
    // Manifesto ilegível recebe o mesmo tratamento do ponteiro corrompido.
    let Ok(manifesto) = serde_json::from_slice::<ManifestoRaw>(&bytes_manifesto) else {
        return Ok(Some(EstadoNaRaw::incompleto(ponteiro.versao)));
    };

    // Manifesto sem partes não descreve dado nenhum: incompleto, para reingerir.
    if manifesto.partes.is_empty() {
        return Ok(Some(EstadoNaRaw::incompleto(ponteiro.versao)));
    }

    let tamanho = manifesto.partes.iter().map(|parte| parte.bytes).sum();
    let integra = partes_integras(armazenamento, &manifesto).await?;

    Ok(Some(EstadoNaRaw {
        completa: true,
        integra,
        versao: ponteiro.versao,
        tamanho: Some(tamanho),
    }))
}

/// O manifesto da ingestão vigente de um recurso, ou `None` se não houver
/// ponteiro ou manifesto. Usado pela ingestão paginada para a requisição
/// condicional página a página.
pub async fn manifesto_vigente(
    armazenamento: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
) -> Result<Option<ManifestoRaw>, ErroDeIngestao> {
    let chave_ponteiro = LocalizacaoRaw::ultima_versao(fonte, recurso);
    let Some(bytes) = armazenamento.ler(&chave_ponteiro).await? else {
        return Ok(None);
    };
    // Ponteiro ou manifesto ilegível: segue como se não houvesse ingestão
    // anterior, para que a reingestão o substitua (ver issue #13).
    let Ok(ponteiro) = serde_json::from_slice::<UltimaVersao>(&bytes) else {
        return Ok(None);
    };
    let Some(bytes_manifesto) = armazenamento.ler(&ponteiro.manifesto).await? else {
        return Ok(None);
    };
    let Ok(manifesto) = serde_json::from_slice::<ManifestoRaw>(&bytes_manifesto) else {
        return Ok(None);
    };
    Ok(Some(manifesto))
}

/// Publica o ponteiro `_ultima_versao.json` sem deixar uma rodada mais antiga
/// sobrescrever uma mais nova.
///
/// Só grava quando não há ponteiro vigente, quando o vigente é ilegível, ou
/// quando o snapshot atual (`nova.iniciada_em`) é mais recente que o guardado.
/// Devolve `true` se gravou, `false` se preservou um ponteiro mais novo ou se
/// houve conflito de escrita condicional (CAS).
///
/// Usa [`ArmazenamentoRaw::gravar_condicional`] para que a gravação só ocorra
/// se o ponteiro não mudou entre a leitura e a escrita. Quando o backend
/// suporta CAS real (e.g. S3 `If-Match`), a operação é atômica; com o
/// fallback padrão, o guard por recência persiste sem regressão.
pub async fn publicar_ponteiro(
    armazenamento: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
    nova: &UltimaVersao,
) -> Result<bool, ErroDeIngestao> {
    let guarda = bloquear_publicacao(fonte, recurso).await;
    publicar_ponteiro_sob_guarda(armazenamento, fonte, recurso, nova, &guarda).await
}

/// Publica após o chamador adquirir a guarda do recurso Bronze correspondente.
/// Usado quando conferência Bronze e troca Silver precisam ser indivisíveis no processo.
pub async fn publicar_ponteiro_sob_guarda(
    armazenamento: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
    nova: &UltimaVersao,
    _guarda: &GuardaPublicacao,
) -> Result<bool, ErroDeIngestao> {
    let chave = LocalizacaoRaw::ultima_versao(fonte, recurso);
    let bytes_vigente = armazenamento.ler(&chave).await?;

    if let Some(ref bytes) = bytes_vigente
        && let Ok(vigente) = serde_json::from_slice::<UltimaVersao>(bytes)
        && (vigente.iniciada_em > nova.iniciada_em
            || (vigente.iniciada_em.is_some()
                && vigente.iniciada_em == nova.iniciada_em
                && vigente.manifesto != nova.manifesto))
    {
        return Ok(false);
    }

    let conteudo = serde_json::to_vec_pretty(nova).map_err(|erro| ErroDeIngestao::Manifesto {
        recurso: recurso.to_owned(),
        detalhe: erro.to_string(),
    })?;
    armazenamento
        .gravar_condicional(
            ObjetoRaw {
                chave,
                conteudo,
                tipo_de_conteudo: "application/json".to_owned(),
            },
            bytes_vigente.as_deref(),
        )
        .await
}

/// Confere que cada parte do manifesto existe e tem o tamanho esperado.
///
/// A checagem é por tamanho (um `HEAD`), não relendo o conteúdo: reler arquivos
/// grandes a cada rodada estouraria a memória do contêiner (ver `docs/adr/0012`).
/// O `sha256` do manifesto segue como procedência, usado na comparação com a
/// origem, não para re-hashear o arquivo local.
async fn partes_integras(
    armazenamento: &dyn ArmazenamentoRaw,
    manifesto: &ManifestoRaw,
) -> Result<bool, ErroDeIngestao> {
    for parte in &manifesto.partes {
        match armazenamento.tamanho(&parte.chave).await? {
            Some(bytes) if bytes == parte.bytes => {}
            _ => return Ok(false),
        }
    }
    Ok(true)
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::Utc;
    use tokio::io::AsyncRead;

    use super::*;
    use crate::portas::ObjetoRaw;
    use crate::raw::ParteRaw;

    const FONTE: &str = "ibge";
    const RECURSO: &str = "estados";
    const CHAVE_PARTE: &str = "ibge/estados/data_ingestao=2026-10-31/execucao=e1/parte-00001.json";
    const CHAVE_MANIFESTO: &str =
        "ibge/estados/data_ingestao=2026-10-31/execucao=e1/_manifesto.json";
    const CONTEUDO: &[u8] = b"[{\"uf\":\"AC\"}]";

    #[derive(Default)]
    struct ArmazenamentoEmMemoria {
        objetos: Mutex<HashMap<String, Vec<u8>>>,
    }

    impl ArmazenamentoEmMemoria {
        fn com(objetos: impl IntoIterator<Item = (String, Vec<u8>)>) -> Self {
            Self {
                objetos: Mutex::new(objetos.into_iter().collect()),
            }
        }
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

        async fn gravar_fluxo(
            &self,
            _: &str,
            _: &str,
            _: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            unreachable!("a pré-checagem não grava fluxo")
        }

        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.objetos.lock().unwrap().get(chave).cloned())
        }
    }

    fn parte(bytes: u64, sha256: &str) -> ParteRaw {
        ParteRaw {
            chave: CHAVE_PARTE.to_owned(),
            url: "https://origem/estados".to_owned(),
            bytes,
            sha256: sha256.to_owned(),
            tipo_de_conteudo: "application/json".to_owned(),
            versao: None,
        }
    }

    fn manifesto(parte: ParteRaw) -> ManifestoRaw {
        ManifestoRaw {
            fonte: FONTE.to_owned(),
            recurso: RECURSO.to_owned(),
            id_execucao: "e1".to_owned(),
            url_inicial: "https://origem/estados".to_owned(),
            iniciada_em: Utc::now(),
            concluida_em: Utc::now(),
            partes: vec![parte],
        }
    }

    fn json<T: serde::Serialize>(valor: &T) -> Vec<u8> {
        serde_json::to_vec(valor).unwrap()
    }

    /// Semeia um recurso íntegro e devolve o armazenamento.
    fn integro() -> ArmazenamentoEmMemoria {
        let ponteiro = UltimaVersao {
            versao: Some("v1".to_owned()),
            manifesto: CHAVE_MANIFESTO.to_owned(),
            iniciada_em: None,
        };
        let manifesto = manifesto(parte(CONTEUDO.len() as u64, "sha-ignorado"));
        ArmazenamentoEmMemoria::com([
            (
                LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
                json(&ponteiro),
            ),
            (CHAVE_MANIFESTO.to_owned(), json(&manifesto)),
            (CHAVE_PARTE.to_owned(), CONTEUDO.to_vec()),
        ])
    }

    #[tokio::test]
    async fn sem_ponteiro_resulta_em_none() {
        let armazenamento = ArmazenamentoEmMemoria::default();
        let estado = estado_na_raw(&armazenamento, FONTE, RECURSO).await.unwrap();
        assert_eq!(estado, None);
    }

    #[tokio::test]
    async fn manifesto_integro_e_completo() {
        let estado = estado_na_raw(&integro(), FONTE, RECURSO)
            .await
            .unwrap()
            .unwrap();
        assert!(estado.completa);
        assert!(estado.integra);
        assert_eq!(estado.versao.as_deref(), Some("v1"));
        assert_eq!(estado.tamanho, Some(CONTEUDO.len() as u64));
    }

    #[tokio::test]
    async fn ponteiro_sem_manifesto_e_incompleto() {
        let ponteiro = UltimaVersao {
            versao: Some("v1".to_owned()),
            manifesto: CHAVE_MANIFESTO.to_owned(),
            iniciada_em: None,
        };
        let armazenamento = ArmazenamentoEmMemoria::com([(
            LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
            json(&ponteiro),
        )]);

        let estado = estado_na_raw(&armazenamento, FONTE, RECURSO)
            .await
            .unwrap()
            .unwrap();
        assert!(!estado.completa);
        assert!(!estado.integra);
    }

    #[tokio::test]
    async fn manifesto_sem_partes_e_incompleto() {
        // Um manifesto válido, porém vazio, não descreve dado nenhum: incompleto.
        let ponteiro = UltimaVersao {
            versao: Some("v1".to_owned()),
            manifesto: CHAVE_MANIFESTO.to_owned(),
            iniciada_em: None,
        };
        let mut manifesto = manifesto(parte(CONTEUDO.len() as u64, "sha-ignorado"));
        manifesto.partes.clear();
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
                json(&ponteiro),
            ),
            (CHAVE_MANIFESTO.to_owned(), json(&manifesto)),
        ]);

        let estado = estado_na_raw(&armazenamento, FONTE, RECURSO)
            .await
            .unwrap()
            .unwrap();
        assert!(!estado.completa);
        assert!(!estado.integra);
    }

    #[tokio::test]
    async fn ponteiro_ilegivel_nao_trava_o_recurso() {
        // JSON corrompido no ponteiro vira estado incompleto, não erro: a próxima
        // ingestão sobrescreve em vez de ficar bloqueada.
        let armazenamento = ArmazenamentoEmMemoria::com([(
            LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
            b"{ nao e json".to_vec(),
        )]);

        let estado = estado_na_raw(&armazenamento, FONTE, RECURSO)
            .await
            .unwrap()
            .unwrap();
        assert!(!estado.completa);
        assert!(!estado.integra);
    }

    #[tokio::test]
    async fn manifesto_ilegivel_nao_trava_o_recurso() {
        let ponteiro = UltimaVersao {
            versao: Some("v1".to_owned()),
            manifesto: CHAVE_MANIFESTO.to_owned(),
            iniciada_em: None,
        };
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
                json(&ponteiro),
            ),
            (CHAVE_MANIFESTO.to_owned(), b"}}} invalido".to_vec()),
        ]);

        let estado = estado_na_raw(&armazenamento, FONTE, RECURSO)
            .await
            .unwrap()
            .unwrap();
        assert!(!estado.completa);
        assert_eq!(estado.versao.as_deref(), Some("v1"));
    }

    #[tokio::test]
    async fn manifesto_vigente_ignora_ponteiro_ilegivel() {
        let armazenamento = ArmazenamentoEmMemoria::com([(
            LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
            b"xpto".to_vec(),
        )]);

        let manifesto = manifesto_vigente(&armazenamento, FONTE, RECURSO)
            .await
            .unwrap();
        assert!(manifesto.is_none());
    }

    #[tokio::test]
    async fn parte_truncada_nao_e_integra() {
        // O manifesto diz que a parte tem mais bytes do que o conteúdo gravado.
        let ponteiro = UltimaVersao {
            versao: Some("v1".to_owned()),
            manifesto: CHAVE_MANIFESTO.to_owned(),
            iniciada_em: None,
        };
        let manifesto = manifesto(parte(CONTEUDO.len() as u64 + 10, "sha-ignorado"));
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
                json(&ponteiro),
            ),
            (CHAVE_MANIFESTO.to_owned(), json(&manifesto)),
            (CHAVE_PARTE.to_owned(), CONTEUDO.to_vec()),
        ]);

        let estado = estado_na_raw(&armazenamento, FONTE, RECURSO)
            .await
            .unwrap()
            .unwrap();
        assert!(estado.completa);
        assert!(!estado.integra);
    }

    #[tokio::test]
    async fn ponteiro_antigo_sem_campo_versao_desserializa() {
        // Formato gravado hoje pela TarefaDeArquivo: versao como string simples.
        let ponteiro = serde_json::json!({ "versao": "v1", "manifesto": CHAVE_MANIFESTO });
        let manifesto = manifesto(parte(CONTEUDO.len() as u64, "sha-ignorado"));
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
                json(&ponteiro),
            ),
            (CHAVE_MANIFESTO.to_owned(), json(&manifesto)),
            (CHAVE_PARTE.to_owned(), CONTEUDO.to_vec()),
        ]);

        let estado = estado_na_raw(&armazenamento, FONTE, RECURSO)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(estado.versao.as_deref(), Some("v1"));
        assert!(estado.integra);
    }

    fn ponteiro(manifesto: &str, iniciada_em: chrono::DateTime<Utc>) -> UltimaVersao {
        UltimaVersao {
            versao: None,
            manifesto: manifesto.to_owned(),
            iniciada_em: Some(iniciada_em),
        }
    }

    async fn manifesto_do_ponteiro(armazenamento: &ArmazenamentoEmMemoria) -> String {
        let bytes = armazenamento
            .ler(&LocalizacaoRaw::ultima_versao(FONTE, RECURSO))
            .await
            .unwrap()
            .unwrap();
        serde_json::from_slice::<UltimaVersao>(&bytes)
            .unwrap()
            .manifesto
    }

    #[tokio::test]
    async fn publicar_ponteiro_grava_quando_ausente() {
        let armazenamento = ArmazenamentoEmMemoria::default();
        let gravou = publicar_ponteiro(&armazenamento, FONTE, RECURSO, &ponteiro("m1", Utc::now()))
            .await
            .unwrap();
        assert!(gravou);
        assert_eq!(manifesto_do_ponteiro(&armazenamento).await, "m1");
    }

    #[tokio::test]
    async fn publicar_ponteiro_nao_deixa_rodada_antiga_sobrescrever_a_nova() {
        let armazenamento = ArmazenamentoEmMemoria::default();
        let nova = Utc::now();
        let antiga = nova - chrono::TimeDelta::seconds(60);

        // A rodada mais nova publica primeiro; a mais antiga termina depois.
        publicar_ponteiro(&armazenamento, FONTE, RECURSO, &ponteiro("nova", nova))
            .await
            .unwrap();
        let gravou = publicar_ponteiro(&armazenamento, FONTE, RECURSO, &ponteiro("antiga", antiga))
            .await
            .unwrap();

        assert!(!gravou);
        assert_eq!(manifesto_do_ponteiro(&armazenamento).await, "nova");
    }

    #[tokio::test]
    async fn publicar_ponteiro_preserva_primeiro_manifesto_em_empate_de_inicio() {
        let armazenamento = ArmazenamentoEmMemoria::default();
        let inicio = Utc::now();
        assert!(
            publicar_ponteiro(
                &armazenamento,
                FONTE,
                RECURSO,
                &ponteiro("primeiro", inicio)
            )
            .await
            .unwrap()
        );
        assert!(
            !publicar_ponteiro(&armazenamento, FONTE, RECURSO, &ponteiro("segundo", inicio))
                .await
                .unwrap()
        );
        assert_eq!(manifesto_do_ponteiro(&armazenamento).await, "primeiro");
    }

    #[tokio::test]
    async fn publicar_ponteiro_sobrescreve_quando_a_nova_e_mais_recente() {
        let armazenamento = ArmazenamentoEmMemoria::default();
        let antiga = Utc::now() - chrono::TimeDelta::seconds(60);

        publicar_ponteiro(&armazenamento, FONTE, RECURSO, &ponteiro("antiga", antiga))
            .await
            .unwrap();
        let gravou = publicar_ponteiro(
            &armazenamento,
            FONTE,
            RECURSO,
            &ponteiro("nova", Utc::now()),
        )
        .await
        .unwrap();

        assert!(gravou);
        assert_eq!(manifesto_do_ponteiro(&armazenamento).await, "nova");
    }

    #[tokio::test]
    async fn publicar_ponteiro_cas_rejeita_escrita_quando_ponteiro_muda_entre_leitura_e_gravacao() {
        use std::sync::atomic::{AtomicBool, Ordering as AtOrd};

        struct CasComIntrusao {
            inner: ArmazenamentoEmMemoria,
            intruso: AtomicBool,
        }

        #[async_trait]
        impl ArmazenamentoRaw for CasComIntrusao {
            async fn preparar(&self) -> Result<(), ErroDeIngestao> {
                Ok(())
            }
            async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao> {
                self.inner.gravar(objeto).await
            }
            async fn gravar_fluxo(
                &self,
                _: &str,
                _: &str,
                _: &mut (dyn AsyncRead + Send + Unpin),
            ) -> Result<(), ErroDeIngestao> {
                unreachable!()
            }
            async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
                self.inner.ler(chave).await
            }
            async fn gravar_condicional(
                &self,
                objeto: ObjetoRaw,
                conteudo_vigente: Option<&[u8]>,
            ) -> Result<bool, ErroDeIngestao> {
                if self.intruso.swap(false, AtOrd::Relaxed) {
                    let intruso = UltimaVersao {
                        versao: None,
                        manifesto: "intruso".to_owned(),
                        iniciada_em: Some(Utc::now() + chrono::TimeDelta::seconds(600)),
                    };
                    self.inner
                        .gravar(ObjetoRaw {
                            chave: objeto.chave.clone(),
                            conteudo: serde_json::to_vec_pretty(&intruso).unwrap(),
                            tipo_de_conteudo: "application/json".into(),
                        })
                        .await?;
                }
                let atual = self.inner.ler(&objeto.chave).await?;
                if atual.as_deref() == conteudo_vigente {
                    self.inner.gravar(objeto).await?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }

        let armazenamento = CasComIntrusao {
            inner: ArmazenamentoEmMemoria::default(),
            intruso: AtomicBool::new(true),
        };
        let gravou = publicar_ponteiro(
            &armazenamento,
            FONTE,
            RECURSO,
            &ponteiro("vitima", Utc::now()),
        )
        .await
        .unwrap();
        assert!(!gravou, "CAS deve rejeitar quando o ponteiro mudou");
        assert_eq!(manifesto_do_ponteiro(&armazenamento.inner).await, "intruso");
    }

    #[tokio::test]
    async fn publicar_ponteiro_sem_data_substitui_ponteiro_legado_sem_data() {
        // Dois ponteiros sem `iniciada_em` (formato legado): a gravação atual
        // prevalece em vez de ficar presa no antigo.
        let armazenamento = ArmazenamentoEmMemoria::default();
        let legado = UltimaVersao {
            versao: None,
            manifesto: "legado".to_owned(),
            iniciada_em: None,
        };
        let novo = UltimaVersao {
            versao: None,
            manifesto: "novo".to_owned(),
            iniciada_em: None,
        };

        publicar_ponteiro(&armazenamento, FONTE, RECURSO, &legado)
            .await
            .unwrap();
        let gravou = publicar_ponteiro(&armazenamento, FONTE, RECURSO, &novo)
            .await
            .unwrap();

        assert!(gravou);
        assert_eq!(manifesto_do_ponteiro(&armazenamento).await, "novo");
    }
}
