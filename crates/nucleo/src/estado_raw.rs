//! Monta o [`EstadoNaRaw`] de um recurso: localiza o manifesto vigente pelo
//! ponteiro `_ultima_versao.json`, confere completude e integridade das partes.
//!
//! Diferente de [`crate::decisao`], que é regra pura, aqui há I/O, sempre pelo
//! porto [`ArmazenamentoRaw`]. O resultado alimenta [`crate::decisao::decidir`].

use crate::decisao::EstadoNaRaw;
use crate::erros::ErroDeIngestao;
use crate::portas::{ArmazenamentoRaw, ObjetoRaw};
use crate::raw::{LocalizacaoRaw, ManifestoRaw, UltimaVersao};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

type Trava = Arc<tokio::sync::Mutex<()>>;
static TRAVAS: OnceLock<Mutex<HashMap<String, Trava>>> = OnceLock::new();
const TENTATIVAS_CAS_PUBLICACAO: u8 = 64;
const PRAZO_CAS_PUBLICACAO: Duration = Duration::from_secs(30);
const PRAZO_AQUISICAO_TRAVA: Duration = Duration::from_secs(30);
const DURACAO_TRAVA_DISTRIBUIDA: chrono::Duration = chrono::Duration::minutes(5);
const ESPERA_TRAVA_DISTRIBUIDA: Duration = Duration::from_millis(25);
static SEQUENCIA_TRAVA_DISTRIBUIDA: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EstadoDaTrava {
    token: String,
    vence_em: DateTime<Utc>,
    liberada: bool,
}

struct TravaDistribuida {
    armazenamento: Arc<dyn ArmazenamentoRaw>,
    chave: String,
    token: String,
    liberada: bool,
}

fn erro_prazo_aquisicao_trava(chave: &str) -> ErroDeIngestao {
    ErroDeIngestao::Armazenamento {
        chave: chave.to_owned(),
        detalhe: "prazo de 30s excedido aguardando publicação concorrente; tente executar a tarefa novamente".into(),
    }
}

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

async fn adquirir_trava_distribuida_compartilhada(
    armazenamento: Arc<dyn ArmazenamentoRaw>,
    fonte: &str,
    recurso: &str,
) -> Result<TravaDistribuida, ErroDeIngestao> {
    let (chave, token) = adquirir_dados_trava(armazenamento.as_ref(), fonte, recurso).await?;
    Ok(TravaDistribuida {
        armazenamento,
        chave,
        token,
        liberada: false,
    })
}

async fn adquirir_dados_trava(
    armazenamento: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
) -> Result<(String, String), ErroDeIngestao> {
    let chave = format!("{fonte}/{recurso}/_publicacao.lock.json");
    let token = format!(
        "{}-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        SEQUENCIA_TRAVA_DISTRIBUIDA.fetch_add(1, Ordering::Relaxed)
    );
    let prazo = tokio::time::Instant::now() + PRAZO_AQUISICAO_TRAVA;

    loop {
        let atual = tokio::time::timeout_at(prazo, armazenamento.ler_controle(&chave))
            .await
            .map_err(|_| erro_prazo_aquisicao_trava(&chave))??;
        let pode_adquirir = atual.as_ref().is_none_or(|bytes| {
            serde_json::from_slice::<EstadoDaTrava>(bytes)
                .is_ok_and(|trava| trava.liberada || trava.vence_em <= Utc::now())
        }) || atual
            .as_ref()
            .is_some_and(|bytes| serde_json::from_slice::<EstadoDaTrava>(bytes).is_err());

        if pode_adquirir {
            let nova = EstadoDaTrava {
                token: token.clone(),
                vence_em: Utc::now() + DURACAO_TRAVA_DISTRIBUIDA,
                liberada: false,
            };
            let gravou = tokio::time::timeout_at(
                prazo,
                armazenamento.gravar_condicional(
                    ObjetoRaw {
                        chave: chave.clone(),
                        conteudo: serde_json::to_vec(&nova).map_err(|erro| {
                            ErroDeIngestao::Manifesto {
                                recurso: recurso.to_owned(),
                                detalhe: erro.to_string(),
                            }
                        })?,
                        tipo_de_conteudo: "application/json".to_owned(),
                    },
                    atual.as_deref(),
                ),
            )
            .await
            .map_err(|_| erro_prazo_aquisicao_trava(&chave))??;
            if gravou {
                return Ok((chave, token));
            }
        }

        if tokio::time::Instant::now() >= prazo {
            return Err(erro_prazo_aquisicao_trava(&chave));
        }
        tokio::time::sleep(ESPERA_TRAVA_DISTRIBUIDA).await;
    }
}

impl TravaDistribuida {
    async fn renovar(&self) -> Result<bool, ErroDeIngestao> {
        let Some(bytes) = self.armazenamento.ler_controle(&self.chave).await? else {
            return Ok(false);
        };
        let Ok(mut atual) = serde_json::from_slice::<EstadoDaTrava>(&bytes) else {
            return Ok(false);
        };
        if atual.token != self.token || atual.liberada || atual.vence_em <= Utc::now() {
            return Ok(false);
        }
        atual.vence_em = Utc::now() + DURACAO_TRAVA_DISTRIBUIDA;
        self.armazenamento
            .gravar_condicional(
                ObjetoRaw {
                    chave: self.chave.clone(),
                    conteudo: serde_json::to_vec(&atual).map_err(|erro| {
                        ErroDeIngestao::Manifesto {
                            recurso: self.chave.clone(),
                            detalhe: erro.to_string(),
                        }
                    })?,
                    tipo_de_conteudo: "application/json".to_owned(),
                },
                Some(&bytes),
            )
            .await
    }

    async fn liberar(mut self) {
        let resultado =
            liberar_trava_distribuida(self.armazenamento.as_ref(), &self.chave, &self.token).await;
        self.liberada = true;
        if let Err(erro) = resultado {
            tracing::warn!(%erro, chave = %self.chave, "falha ao liberar trava distribuída; ela expira automaticamente");
        }
    }
}

impl Drop for TravaDistribuida {
    fn drop(&mut self) {
        if self.liberada {
            return;
        }
        let (armazenamento, chave, token) = (
            Arc::clone(&self.armazenamento),
            self.chave.clone(),
            self.token.clone(),
        );
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                if let Err(erro) =
                    liberar_trava_distribuida(armazenamento.as_ref(), &chave, &token).await
                {
                    tracing::warn!(%erro, chave = %chave, "falha ao liberar trava distribuída após cancelamento");
                }
            });
        }
    }
}

async fn liberar_trava_distribuida(
    armazenamento: &dyn ArmazenamentoRaw,
    chave: &str,
    token: &str,
) -> Result<(), ErroDeIngestao> {
    let Some(bytes) = armazenamento.ler_controle(chave).await? else {
        return Ok(());
    };
    let Ok(mut atual) = serde_json::from_slice::<EstadoDaTrava>(&bytes) else {
        return Ok(());
    };
    if atual.token != token || atual.liberada {
        return Ok(());
    }
    atual.liberada = true;
    atual.vence_em = Utc::now();
    armazenamento
        .gravar_condicional(
            ObjetoRaw {
                chave: chave.to_owned(),
                conteudo: serde_json::to_vec(&atual).map_err(|erro| ErroDeIngestao::Manifesto {
                    recurso: chave.to_owned(),
                    detalhe: erro.to_string(),
                })?,
                tipo_de_conteudo: "application/json".to_owned(),
            },
            Some(&bytes),
        )
        .await?;
    Ok(())
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
    Ok(estado_na_raw_com_origem(armazenamento, fonte, recurso)
        .await?
        .map(|(estado, _)| estado))
}

/// Lê o estado de um recurso na Raw e a URL gravada no manifesto vigente.
///
/// Esta função preserva a URL sem alterar o contrato público de `EstadoNaRaw`.
pub async fn estado_na_raw_com_origem(
    armazenamento: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
) -> Result<Option<(EstadoNaRaw, Option<String>)>, ErroDeIngestao> {
    let chave_ponteiro = LocalizacaoRaw::ultima_versao(fonte, recurso);
    let Some(bytes) = armazenamento.ler_controle(&chave_ponteiro).await? else {
        return Ok(None);
    };
    // Ponteiro ilegível não pode travar o recurso: trata como estado incompleto
    // para que a decisão reingira e o sobrescreva (ver issue #13).
    let Ok(ponteiro) = serde_json::from_slice::<UltimaVersao>(&bytes) else {
        return Ok(Some((EstadoNaRaw::incompleto(None), None)));
    };

    let Some(bytes_manifesto) = armazenamento.ler(&ponteiro.manifesto).await? else {
        return Ok(Some((EstadoNaRaw::incompleto(ponteiro.versao), None)));
    };
    // Manifesto ilegível recebe o mesmo tratamento do ponteiro corrompido.
    let Ok(manifesto) = serde_json::from_slice::<ManifestoRaw>(&bytes_manifesto) else {
        return Ok(Some((EstadoNaRaw::incompleto(ponteiro.versao), None)));
    };

    // Manifesto sem partes não descreve dado nenhum: incompleto, para reingerir.
    if manifesto.partes.is_empty() {
        return Ok(Some((EstadoNaRaw::incompleto(ponteiro.versao), None)));
    }

    let tamanho = manifesto.partes.iter().map(|parte| parte.bytes).sum();
    let integra = partes_integras(armazenamento, &manifesto).await?;

    let origem = Some(manifesto.url_inicial.clone());
    Ok(Some((
        EstadoNaRaw {
            completa: true,
            integra,
            versao: ponteiro.versao,
            tamanho: Some(tamanho),
        },
        origem,
    )))
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
    let Some(bytes) = armazenamento.ler_controle(&chave_ponteiro).await? else {
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
/// Devolve `true` se o ponteiro desta execução está vigente, e `false` se
/// preservou um ponteiro mais novo. Conflitos CAS persistentes e timeouts
/// devolvem erro com indicação para repetir a tarefa.
///
/// Serializa publicações dentro do processo e usa CAS para o ponteiro. Para
/// armazenamento compartilhado entre processos, use
/// [`publicar_ponteiro_compartilhado`], que também mantém a trava distribuída.
pub async fn publicar_ponteiro(
    armazenamento: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
    nova: &UltimaVersao,
) -> Result<bool, ErroDeIngestao> {
    let guarda = bloquear_publicacao(fonte, recurso).await;
    publicar_ponteiro_sob_guarda(armazenamento, fonte, recurso, nova, &guarda).await
}

/// Publica um ponteiro usando a trava distribuída e libera-a também quando a
/// tarefa chamadora é cancelada. Use esta variante para storage compartilhado.
pub async fn publicar_ponteiro_compartilhado(
    armazenamento: Arc<dyn ArmazenamentoRaw>,
    fonte: &str,
    recurso: &str,
    nova: &UltimaVersao,
) -> Result<bool, ErroDeIngestao> {
    let trava =
        adquirir_trava_distribuida_compartilhada(Arc::clone(&armazenamento), fonte, recurso)
            .await?;
    let guarda = bloquear_publicacao(fonte, recurso).await;
    let resultado =
        publicar_ponteiro_sob_guarda(armazenamento.as_ref(), fonte, recurso, nova, &guarda).await;
    trava.liberar().await;
    resultado
}

/// Confere o manifesto Raw vigente e publica Bronze sob a mesma trava distribuída
/// usada pelos publicadores Raw. Assim, outro processo não troca a origem entre a
/// conferência final e a gravação do ponteiro Bronze.
pub async fn publicar_ponteiro_bronze_se_raw_vigente(
    raw: Arc<dyn ArmazenamentoRaw>,
    bronze: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
    id_execucao_raw: &str,
    ponteiro_bronze: &UltimaVersao,
) -> Result<bool, ErroDeIngestao> {
    let trava = adquirir_trava_distribuida_compartilhada(Arc::clone(&raw), fonte, recurso).await?;
    let guarda = bloquear_publicacao(fonte, recurso).await;
    let resultado = async {
        let vigente = manifesto_vigente(raw.as_ref(), fonte, recurso).await?;
        if vigente
            .as_ref()
            .is_none_or(|manifesto| manifesto.id_execucao != id_execucao_raw)
        {
            return Ok(false);
        }
        if !trava.renovar().await? {
            return Ok(false);
        }
        let vigente = manifesto_vigente(raw.as_ref(), fonte, recurso).await?;
        if vigente
            .as_ref()
            .is_none_or(|manifesto| manifesto.id_execucao != id_execucao_raw)
        {
            return Ok(false);
        }
        publicar_ponteiro_bronze_sob_guarda(bronze, fonte, recurso, ponteiro_bronze, &guarda).await
    }
    .await;
    trava.liberar().await;
    resultado
}

/// Publica após o chamador adquirir a guarda do recurso Bronze correspondente.
/// Usado quando conferência Bronze e troca Silver precisam ser indivisíveis no processo.
pub async fn publicar_ponteiro_sob_guarda(
    armazenamento: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
    nova: &UltimaVersao,
    guarda: &GuardaPublicacao,
) -> Result<bool, ErroDeIngestao> {
    publicar_ponteiro_com_regra_de_versao(armazenamento, fonte, recurso, nova, guarda, false).await
}

async fn publicar_ponteiro_bronze_sob_guarda(
    armazenamento: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
    nova: &UltimaVersao,
    guarda: &GuardaPublicacao,
) -> Result<bool, ErroDeIngestao> {
    publicar_ponteiro_com_regra_de_versao(armazenamento, fonte, recurso, nova, guarda, true).await
}

async fn publicar_ponteiro_com_regra_de_versao(
    armazenamento: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
    nova: &UltimaVersao,
    _guarda: &GuardaPublicacao,
    substituir_mesma_versao: bool,
) -> Result<bool, ErroDeIngestao> {
    let chave = LocalizacaoRaw::ultima_versao(fonte, recurso);
    let conteudo = serde_json::to_vec_pretty(nova).map_err(|erro| ErroDeIngestao::Manifesto {
        recurso: recurso.to_owned(),
        detalhe: erro.to_string(),
    })?;

    let publicar = async {
        // Um CAS pode perder para outra execução que publicou depois da leitura.
        // Releia e reavalie a recência: uma execução antiga não bloqueia esta;
        // uma mais nova vence; conflitos persistentes falham com indicação de retry.
        for tentativa in 0..TENTATIVAS_CAS_PUBLICACAO {
            let bytes_vigente = armazenamento.ler_controle(&chave).await?;
            if let Some(ref bytes) = bytes_vigente
                && let Ok(vigente) = serde_json::from_slice::<UltimaVersao>(bytes)
            {
                if vigente.manifesto == nova.manifesto && vigente.iniciada_em == nova.iniciada_em {
                    return Ok(true);
                }
                if !(substituir_mesma_versao && vigente.versao == nova.versao)
                    && ponteiro_e_mais_recente(&vigente, nova)
                {
                    return Ok(false);
                }
            }

            let gravou = armazenamento
                .gravar_condicional(
                    ObjetoRaw {
                        chave: chave.clone(),
                        conteudo: conteudo.clone(),
                        tipo_de_conteudo: "application/json".to_owned(),
                    },
                    bytes_vigente.as_deref(),
                )
                .await?;
            if gravou {
                return Ok(true);
            }
            if tentativa + 1 < TENTATIVAS_CAS_PUBLICACAO {
                let espera = Duration::from_millis(1_u64 << tentativa.min(4));
                tokio::time::sleep(espera).await;
            }
        }

        let bytes_vigente = armazenamento.ler_controle(&chave).await?;
        if let Some(bytes) = bytes_vigente
            && let Ok(vigente) = serde_json::from_slice::<UltimaVersao>(&bytes)
        {
            if vigente.manifesto == nova.manifesto && vigente.iniciada_em == nova.iniciada_em {
                return Ok(true);
            }
            if !(substituir_mesma_versao && vigente.versao == nova.versao)
                && ponteiro_e_mais_recente(&vigente, nova)
            {
                return Ok(false);
            }
        }

        Err(ErroDeIngestao::Armazenamento {
            chave: chave.clone(),
            detalhe: format!(
                "publicação CAS conflitou {TENTATIVAS_CAS_PUBLICACAO} vezes; tente executar a tarefa novamente"
            ),
        })
    };

    let resultado = match tokio::time::timeout(PRAZO_CAS_PUBLICACAO, publicar).await {
        Ok(resultado) => resultado,
        Err(_) => Err(ErroDeIngestao::Armazenamento {
            chave,
            detalhe: format!(
                "prazo de {}s excedido ao publicar ponteiro Raw; tente executar a tarefa novamente",
                PRAZO_CAS_PUBLICACAO.as_secs()
            ),
        }),
    };
    if resultado.is_err()
        && let Ok(Some(bytes)) = armazenamento
            .ler_controle(&LocalizacaoRaw::ultima_versao(fonte, recurso))
            .await
        && serde_json::from_slice::<UltimaVersao>(&bytes).is_ok_and(|vigente| {
            vigente.manifesto == nova.manifesto && vigente.iniciada_em == nova.iniciada_em
        })
    {
        return Ok(true);
    }
    resultado
}

fn ponteiro_e_mais_recente(vigente: &UltimaVersao, nova: &UltimaVersao) -> bool {
    vigente.iniciada_em > nova.iniciada_em
        || (vigente.iniciada_em.is_some()
            && vigente.iniciada_em == nova.iniciada_em
            && vigente.manifesto != nova.manifesto)
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

        async fn gravar_condicional(
            &self,
            objeto: ObjetoRaw,
            conteudo_vigente: Option<&[u8]>,
        ) -> Result<bool, ErroDeIngestao> {
            let mut objetos = self.objetos.lock().unwrap();
            if objetos.get(&objeto.chave).map(Vec::as_slice) != conteudo_vigente {
                return Ok(false);
            }
            objetos.insert(objeto.chave, objeto.conteudo);
            Ok(true)
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
    async fn publicar_bronze_sob_trava_raw_exige_a_mesma_execucao() {
        let raw: Arc<dyn ArmazenamentoRaw> = Arc::new(integro());
        let bronze = ArmazenamentoEmMemoria::default();
        let ponteiro_bronze = UltimaVersao {
            versao: Some("e1".into()),
            manifesto: "bronze/estados/_manifesto.json".into(),
            iniciada_em: Some(Utc::now()),
        };

        assert!(
            !publicar_ponteiro_bronze_se_raw_vigente(
                Arc::clone(&raw),
                &bronze,
                FONTE,
                RECURSO,
                "outra-execucao",
                &ponteiro_bronze,
            )
            .await
            .unwrap()
        );
        assert!(
            estado_na_raw(&bronze, FONTE, RECURSO)
                .await
                .unwrap()
                .is_none()
        );

        assert!(
            publicar_ponteiro_bronze_se_raw_vigente(
                Arc::clone(&raw),
                &bronze,
                FONTE,
                RECURSO,
                "e1",
                &ponteiro_bronze,
            )
            .await
            .unwrap()
        );
        assert_eq!(
            manifesto_do_ponteiro(&bronze).await,
            "bronze/estados/_manifesto.json"
        );

        bronze
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao(FONTE, RECURSO),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some("e1".into()),
                    manifesto: "bronze/estados/manifesto-antigo.json".into(),
                    iniciada_em: Some(Utc::now() + chrono::Duration::hours(1)),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        let reconstruido = UltimaVersao {
            versao: Some("e1".into()),
            manifesto: "bronze/estados/manifesto-reconstruido.json".into(),
            iniciada_em: Some(Utc::now()),
        };
        assert!(
            publicar_ponteiro_bronze_se_raw_vigente(
                Arc::clone(&raw),
                &bronze,
                FONTE,
                RECURSO,
                "e1",
                &reconstruido,
            )
            .await
            .unwrap()
        );
        assert_eq!(
            manifesto_do_ponteiro(&bronze).await,
            "bronze/estados/manifesto-reconstruido.json"
        );
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
                if objeto.chave.ends_with("/_ultima_versao.json")
                    && self.intruso.swap(false, AtOrd::Relaxed)
                {
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
    async fn publicar_ponteiro_persiste_apos_mais_de_oito_conflitos_com_rodadas_antigas() {
        use std::sync::atomic::{AtomicUsize, Ordering as AtOrd};

        struct CasComRodadaAntiga {
            inner: ArmazenamentoEmMemoria,
            conflitos: AtomicUsize,
        }

        #[async_trait]
        impl ArmazenamentoRaw for CasComRodadaAntiga {
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
                let houve_conflito = objeto.chave.ends_with("/_ultima_versao.json")
                    && self.conflitos.load(AtOrd::Relaxed) > 0;
                if houve_conflito {
                    self.conflitos.fetch_sub(1, AtOrd::Relaxed);
                    let intruso = UltimaVersao {
                        versao: None,
                        manifesto: "rodada-antiga".to_owned(),
                        iniciada_em: Some(Utc::now() - chrono::TimeDelta::seconds(600)),
                    };
                    self.inner
                        .gravar(ObjetoRaw {
                            chave: objeto.chave.clone(),
                            conteudo: serde_json::to_vec_pretty(&intruso).unwrap(),
                            tipo_de_conteudo: "application/json".into(),
                        })
                        .await?;
                    return Ok(false);
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

        let armazenamento = CasComRodadaAntiga {
            inner: ArmazenamentoEmMemoria::default(),
            conflitos: AtomicUsize::new(10),
        };
        let gravou = publicar_ponteiro(
            &armazenamento,
            FONTE,
            RECURSO,
            &ponteiro("nova", Utc::now()),
        )
        .await
        .unwrap();

        assert!(
            gravou,
            "a rodada atual deve persistir até vencer conflitos com rodadas antigas"
        );
        assert_eq!(armazenamento.conflitos.load(AtOrd::Relaxed), 0);
        assert_eq!(manifesto_do_ponteiro(&armazenamento.inner).await, "nova");
    }

    #[tokio::test]
    async fn conflitos_cas_persistentes_devolvem_erro_repetivel() {
        struct CasSempreEmConflito(ArmazenamentoEmMemoria);

        #[async_trait]
        impl ArmazenamentoRaw for CasSempreEmConflito {
            async fn preparar(&self) -> Result<(), ErroDeIngestao> {
                Ok(())
            }
            async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao> {
                self.0.gravar(objeto).await
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
                self.0.ler(chave).await
            }
            async fn gravar_condicional(
                &self,
                objeto: ObjetoRaw,
                _: Option<&[u8]>,
            ) -> Result<bool, ErroDeIngestao> {
                if objeto.chave.ends_with("/_publicacao.lock.json") {
                    self.0.gravar(objeto).await?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }

        let armazenamento = CasSempreEmConflito(ArmazenamentoEmMemoria::default());
        let erro = publicar_ponteiro(
            &armazenamento,
            FONTE,
            RECURSO,
            &ponteiro("nova", Utc::now()),
        )
        .await
        .unwrap_err();

        assert!(
            erro.to_string()
                .contains("tente executar a tarefa novamente")
        );
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
