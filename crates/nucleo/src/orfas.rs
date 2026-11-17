//! Recuperação de snapshots Raw sem publicação concluída. Snapshots marcados para
//! remoção passam por uma janela de segurança para preservar leitores que já tenham
//! carregado o ponteiro antes de uma publicação ser substituída.
//!
//! Ver issue #269 e PR #278 (revisão).

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::erros::ErroDeIngestao;
use crate::portas::{ArmazenamentoRaw, FalhaDeRemocao, ObjetoRaw};
use crate::raw::{ManifestoRaw, UltimaVersao};

const CONCORRENCIA: usize = 4;
const DEADLINE: Duration = Duration::from_secs(30);
const GRACA_SNAPSHOT: chrono::Duration = chrono::Duration::hours(24);

#[derive(Debug, Serialize, Deserialize)]
struct LimpezaPendente {
    manifesto: String,
    solicitada_em: DateTime<Utc>,
}

/// Agenda a remoção tardia de um snapshot cuja publicação foi perdida ou ficou ambígua.
/// A varredura espera a janela de segurança e confere novamente o ponteiro antes de apagar.
pub async fn agendar_limpeza_snapshot(
    armazenamento: &dyn ArmazenamentoRaw,
    chave_manifesto: &str,
) -> Result<(), ErroDeIngestao> {
    let chave = chave_da_limpeza(chave_manifesto);
    let conteudo = serde_json::to_vec(&LimpezaPendente {
        manifesto: chave_manifesto.to_owned(),
        solicitada_em: Utc::now(),
    })
    .map_err(|erro| ErroDeIngestao::Manifesto {
        recurso: chave_manifesto.to_owned(),
        detalhe: erro.to_string(),
    })?;
    armazenamento
        .gravar(ObjetoRaw {
            chave,
            conteudo,
            tipo_de_conteudo: "application/json".to_owned(),
        })
        .await
}

fn chave_da_limpeza(chave_manifesto: &str) -> String {
    chave_manifesto.strip_suffix("_manifesto.json").map_or_else(
        || format!("{chave_manifesto}.limpeza_pendente.json"),
        |prefixo| format!("{prefixo}_limpeza_pendente.json"),
    )
}

/// Resultado da verificação de manifesto após erro ambíguo.
#[derive(Debug)]
pub enum ResultadoVerificacao {
    /// Manifesto existe e é válido — partes protegidas, nenhuma remoção feita.
    ManifestoConfirmado,
    /// Manifesto ausente — partes removidas (falhas individuais no vetor).
    ManifestoAusente(Vec<FalhaDeRemocao>),
    /// Manifesto presente mas ilegível — partes removidas por precaução.
    ManifestoIlegivel(Vec<FalhaDeRemocao>),
}

/// Verifica se o manifesto de uma execução realmente foi gravado e, se não,
/// remove as partes que ficariam desprotegidas.
///
/// Cenário: a gravação do manifesto recebeu erro de rede, mas o servidor pode
/// ter feito o commit. Esta função verifica o estado real antes de apagar.
pub async fn verificar_manifesto_ou_limpar(
    armazenamento: &dyn ArmazenamentoRaw,
    chave_manifesto: &str,
    partes_novas: &[String],
) -> Result<ResultadoVerificacao, ErroDeIngestao> {
    match armazenamento.ler(chave_manifesto).await? {
        Some(bytes) => {
            if serde_json::from_slice::<ManifestoRaw>(&bytes).is_ok() {
                info!(
                    manifesto = %chave_manifesto,
                    "manifesto confirmado após erro ambíguo; partes protegidas"
                );
                return Ok(ResultadoVerificacao::ManifestoConfirmado);
            }
            warn!(
                manifesto = %chave_manifesto,
                "manifesto ilegível após erro ambíguo; removendo partes órfãs"
            );
            let falhas =
                remover_e_logar(armazenamento, partes_novas, "verificação de manifesto").await;
            Ok(ResultadoVerificacao::ManifestoIlegivel(falhas))
        }
        None => {
            info!(
                manifesto = %chave_manifesto,
                "manifesto ausente após erro ambíguo; removendo partes órfãs"
            );
            let falhas =
                remover_e_logar(armazenamento, partes_novas, "verificação de manifesto").await;
            Ok(ResultadoVerificacao::ManifestoAusente(falhas))
        }
    }
}

/// Varre um prefixo e remove objetos não referenciados, incluindo snapshots
/// explicitamente agendados quando sua janela de segurança já expirou.
///
/// `id_execucao_atual` exclui partes da ingestão em andamento para evitar
/// colisão quando duas execuções do mesmo recurso se sobrepõem.
///
/// Snapshots com marcador permanecem protegidos por 24 horas e enquanto forem
/// apontados por `_ultima_versao.json`. Se um manifesto ou ponteiro listado não
/// puder ser lido, a execução é protegida por inteiro.
pub async fn limpar_orfas_do_recurso(
    armazenamento: &dyn ArmazenamentoRaw,
    prefixo_recurso: &str,
    id_execucao_atual: &str,
) -> Result<(usize, Vec<FalhaDeRemocao>), ErroDeIngestao> {
    let chaves = armazenamento.listar_prefixo(prefixo_recurso).await?;
    if chaves.is_empty() {
        return Ok((0, Vec::new()));
    }

    let mut protegidas = HashSet::new();
    let mut manifestos_vigentes = HashSet::new();

    for chave in &chaves {
        if chave.ends_with("/_publicacao.lock.json") {
            protegidas.insert(chave.clone());
        }
        if chave.ends_with("/_ultima_versao.json") {
            protegidas.insert(chave.clone());
            match armazenamento.ler(chave).await {
                Ok(Some(bytes)) => match serde_json::from_slice::<UltimaVersao>(&bytes) {
                    Ok(ponteiro) => {
                        manifestos_vigentes.insert(ponteiro.manifesto);
                    }
                    Err(_) => proteger_execucao(chave, &chaves, &mut protegidas),
                },
                Ok(None) | Err(_) => proteger_execucao(chave, &chaves, &mut protegidas),
            }
        }
    }

    let mut limpezas_pendentes = HashMap::new();
    for chave in &chaves {
        if !chave.ends_with("/_limpeza_pendente.json") {
            continue;
        }
        match armazenamento.ler(chave).await {
            Ok(Some(bytes)) => match serde_json::from_slice::<LimpezaPendente>(&bytes) {
                Ok(limpeza) if chave_da_limpeza(&limpeza.manifesto) == *chave => {
                    limpezas_pendentes.insert(limpeza.manifesto.clone(), (chave, limpeza));
                }
                _ => proteger_execucao(chave, &chaves, &mut protegidas),
            },
            Ok(None) | Err(_) => proteger_execucao(chave, &chaves, &mut protegidas),
        }
    }

    for chave in &chaves {
        if chave.contains(&format!("execucao={id_execucao_atual}/")) {
            protegidas.insert(chave.clone());
        }
    }

    for chave in &chaves {
        if !chave.ends_with("/_manifesto.json") {
            continue;
        }
        let limpeza_expirada = limpezas_pendentes.get(chave).is_some_and(|(_, limpeza)| {
            !manifestos_vigentes.contains(chave)
                && Utc::now().signed_duration_since(limpeza.solicitada_em) >= GRACA_SNAPSHOT
        });
        let deve_proteger = manifestos_vigentes.contains(chave) || !limpeza_expirada;
        match armazenamento.ler(chave).await {
            Ok(Some(bytes)) => match serde_json::from_slice::<ManifestoRaw>(&bytes) {
                Ok(manifesto) if deve_proteger => {
                    protegidas.insert(chave.clone());
                    for parte in &manifesto.partes {
                        protegidas.insert(parte.chave.clone());
                    }
                }
                Ok(_) => {}
                Err(_) => proteger_execucao(chave, &chaves, &mut protegidas),
            },
            Ok(None) | Err(_) => proteger_execucao(chave, &chaves, &mut protegidas),
        }
    }

    let mut limpezas_expiradas = Vec::new();
    for (manifesto, (chave_limpeza, limpeza)) in &limpezas_pendentes {
        let expirada = Utc::now().signed_duration_since(limpeza.solicitada_em) >= GRACA_SNAPSHOT;
        protegidas.insert((*chave_limpeza).clone());
        if expirada && !manifestos_vigentes.contains(manifesto) {
            limpezas_expiradas.push(((*chave_limpeza).clone(), prefixo_da_execucao(manifesto)));
        } else {
            proteger_execucao(manifesto, &chaves, &mut protegidas);
        }
    }

    let orfas: Vec<String> = chaves
        .into_iter()
        .filter(|chave| !protegidas.contains(chave))
        .collect();

    let mut total = orfas.len();
    let mut falhas = if orfas.is_empty() {
        Vec::new()
    } else {
        info!(
            prefixo = %prefixo_recurso,
            orfas = total,
            "encontradas partes órfãs; iniciando limpeza"
        );
        remover_e_logar(armazenamento, &orfas, "varredura").await
    };

    // O marcador só sai depois que todos os objetos daquela execução foram
    // removidos. Se uma remoção falhar, ele permite que a próxima varredura
    // tente novamente em vez de deixar um manifesto válido órfão protegido para sempre.
    for (chave_limpeza, prefixo_execucao) in limpezas_expiradas {
        if falhas
            .iter()
            .any(|falha| falha.chave.starts_with(&prefixo_execucao))
        {
            continue;
        }
        let falhas_marcador = remover_e_logar(
            armazenamento,
            &[chave_limpeza],
            "marcador de limpeza concluída",
        )
        .await;
        if falhas_marcador.is_empty() {
            total += 1;
        }
        falhas.extend(falhas_marcador);
    }
    Ok((total, falhas))
}

fn proteger_execucao(chave: &str, chaves: &[String], protegidas: &mut HashSet<String>) {
    warn!(manifesto = %chave, "estado do snapshot incerto; protegendo a execução inteira");
    let prefixo_exec = prefixo_da_execucao(chave);
    for outra in chaves {
        if outra.starts_with(&prefixo_exec) {
            protegidas.insert(outra.clone());
        }
    }
}

/// Extrai o prefixo da execução de uma chave de manifesto.
/// Ex.: `ibge/estados/data=.../execucao=abc/_manifesto.json` → `ibge/estados/data=.../execucao=abc/`
fn prefixo_da_execucao(chave_manifesto: &str) -> String {
    match chave_manifesto.rfind('/') {
        Some(pos) => chave_manifesto[..=pos].to_owned(),
        None => chave_manifesto.to_owned(),
    }
}

async fn remover_e_logar(
    armazenamento: &dyn ArmazenamentoRaw,
    chaves: &[String],
    contexto: &str,
) -> Vec<FalhaDeRemocao> {
    let falhas = armazenamento
        .remover_lote(chaves, CONCORRENCIA, DEADLINE)
        .await;
    for falha in &falhas {
        warn!(
            chave = %falha.chave,
            detalhe = %falha.detalhe,
            contexto = %contexto,
            "falha ao remover parte órfã"
        );
    }
    falhas
}

#[cfg(test)]
mod testes {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use tokio::io::AsyncRead;

    use crate::portas::ObjetoRaw;
    use crate::raw::{ManifestoRaw, ParteRaw, UltimaVersao};

    #[derive(Default)]
    struct ArmazenamentoEmMemoria {
        objetos: Mutex<HashMap<String, Vec<u8>>>,
        falha_remocao: Mutex<Option<String>>,
    }

    impl ArmazenamentoEmMemoria {
        fn com(objetos: impl IntoIterator<Item = (String, Vec<u8>)>) -> Self {
            Self {
                objetos: Mutex::new(objetos.into_iter().collect()),
                falha_remocao: Mutex::new(None),
            }
        }

        fn contem(&self, chave: &str) -> bool {
            self.objetos.lock().unwrap().contains_key(chave)
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
            unreachable!()
        }

        async fn remover(&self, chave: &str) -> Result<(), ErroDeIngestao> {
            if self.falha_remocao.lock().unwrap().as_deref() == Some(chave) {
                return Err(ErroDeIngestao::Armazenamento {
                    chave: chave.to_owned(),
                    detalhe: "falha simulada".to_owned(),
                });
            }
            self.objetos.lock().unwrap().remove(chave);
            Ok(())
        }

        async fn listar_prefixo(&self, prefixo: &str) -> Result<Vec<String>, ErroDeIngestao> {
            Ok(self
                .objetos
                .lock()
                .unwrap()
                .keys()
                .filter(|chave| chave.starts_with(prefixo))
                .cloned()
                .collect())
        }

        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.objetos.lock().unwrap().get(chave).cloned())
        }
    }

    fn json<T: serde::Serialize>(valor: &T) -> Vec<u8> {
        serde_json::to_vec(valor).unwrap()
    }

    fn parte(chave: &str) -> ParteRaw {
        ParteRaw {
            chave: chave.to_owned(),
            url: "https://origem/dados".to_owned(),
            bytes: 100,
            sha256: "abc".to_owned(),
            tipo_de_conteudo: "application/json".to_owned(),
            versao: None,
        }
    }

    fn manifesto_com_partes(partes: Vec<ParteRaw>) -> ManifestoRaw {
        ManifestoRaw {
            fonte: "ibge".to_owned(),
            recurso: "estados".to_owned(),
            id_execucao: "e1".to_owned(),
            url_inicial: "https://origem/dados".to_owned(),
            iniciada_em: chrono::Utc::now(),
            concluida_em: chrono::Utc::now(),
            partes,
        }
    }

    // --- verificar_manifesto_ou_limpar ---

    #[tokio::test]
    async fn manifesto_presente_protege_partes() {
        let manifesto = manifesto_com_partes(vec![parte("p1"), parte("p2")]);
        let armazenamento = ArmazenamentoEmMemoria::com([
            ("_manifesto.json".to_owned(), json(&manifesto)),
            ("p1".to_owned(), b"dados1".to_vec()),
            ("p2".to_owned(), b"dados2".to_vec()),
        ]);

        let resultado = verificar_manifesto_ou_limpar(
            &armazenamento,
            "_manifesto.json",
            &["p1".to_owned(), "p2".to_owned()],
        )
        .await
        .unwrap();

        assert!(matches!(
            resultado,
            ResultadoVerificacao::ManifestoConfirmado
        ));
        assert!(armazenamento.contem("p1"));
        assert!(armazenamento.contem("p2"));
    }

    #[tokio::test]
    async fn manifesto_ausente_limpa_partes() {
        let armazenamento = ArmazenamentoEmMemoria::com([
            ("p1".to_owned(), b"dados1".to_vec()),
            ("p2".to_owned(), b"dados2".to_vec()),
        ]);

        let resultado = verificar_manifesto_ou_limpar(
            &armazenamento,
            "_manifesto.json",
            &["p1".to_owned(), "p2".to_owned()],
        )
        .await
        .unwrap();

        assert!(matches!(resultado, ResultadoVerificacao::ManifestoAusente(ref f) if f.is_empty()));
        assert!(!armazenamento.contem("p1"));
        assert!(!armazenamento.contem("p2"));
    }

    #[tokio::test]
    async fn manifesto_ilegivel_limpa_partes_e_sinaliza() {
        let armazenamento = ArmazenamentoEmMemoria::com([
            ("_manifesto.json".to_owned(), b"nao e json".to_vec()),
            ("p1".to_owned(), b"dados".to_vec()),
        ]);

        let resultado =
            verificar_manifesto_ou_limpar(&armazenamento, "_manifesto.json", &["p1".to_owned()])
                .await
                .unwrap();

        assert!(
            matches!(resultado, ResultadoVerificacao::ManifestoIlegivel(ref f) if f.is_empty())
        );
        assert!(!armazenamento.contem("p1"));
    }

    // --- limpar_orfas_do_recurso ---

    #[tokio::test]
    async fn nenhuma_orfa_quando_tudo_referenciado() {
        let manifesto = manifesto_com_partes(vec![parte("ibge/estados/exec1/parte-00001.json")]);
        let ponteiro = UltimaVersao {
            versao: None,
            manifesto: "ibge/estados/exec1/_manifesto.json".to_owned(),
            iniciada_em: None,
        };
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                "ibge/estados/_ultima_versao.json".to_owned(),
                json(&ponteiro),
            ),
            (
                "ibge/estados/exec1/_manifesto.json".to_owned(),
                json(&manifesto),
            ),
            (
                "ibge/estados/exec1/parte-00001.json".to_owned(),
                b"dados".to_vec(),
            ),
        ]);

        let (total, falhas) = limpar_orfas_do_recurso(&armazenamento, "ibge/estados/", "corrente")
            .await
            .unwrap();
        assert_eq!(total, 0);
        assert!(falhas.is_empty());
        assert!(armazenamento.contem("ibge/estados/exec1/parte-00001.json"));
    }

    #[tokio::test]
    async fn snapshot_perdedor_fica_protegido_durante_a_graca() {
        let manifesto_vigente =
            manifesto_com_partes(vec![parte("ibge/estados/exec-atual/parte.json")]);
        let manifesto_perdedor =
            manifesto_com_partes(vec![parte("ibge/estados/exec-perdedor/parte.json")]);
        let chave_manifesto_perdedor = "ibge/estados/exec-perdedor/_manifesto.json";
        let limpeza = LimpezaPendente {
            manifesto: chave_manifesto_perdedor.to_owned(),
            solicitada_em: Utc::now(),
        };
        let ponteiro = UltimaVersao {
            versao: None,
            manifesto: "ibge/estados/exec-atual/_manifesto.json".to_owned(),
            iniciada_em: None,
        };
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                "ibge/estados/_ultima_versao.json".to_owned(),
                json(&ponteiro),
            ),
            (
                "ibge/estados/exec-atual/_manifesto.json".to_owned(),
                json(&manifesto_vigente),
            ),
            (
                "ibge/estados/exec-atual/parte.json".to_owned(),
                b"atual".to_vec(),
            ),
            (
                chave_manifesto_perdedor.to_owned(),
                json(&manifesto_perdedor),
            ),
            (
                "ibge/estados/exec-perdedor/parte.json".to_owned(),
                b"perdedor".to_vec(),
            ),
            (chave_da_limpeza(chave_manifesto_perdedor), json(&limpeza)),
        ]);

        let (removidas, falhas) =
            limpar_orfas_do_recurso(&armazenamento, "ibge/estados/", "corrente")
                .await
                .unwrap();

        assert_eq!(removidas, 0);
        assert!(falhas.is_empty());
        assert!(armazenamento.contem(chave_manifesto_perdedor));
        assert!(armazenamento.contem("ibge/estados/exec-perdedor/parte.json"));
        assert!(armazenamento.contem(&chave_da_limpeza(chave_manifesto_perdedor)));
    }

    #[tokio::test]
    async fn snapshot_perdedor_e_removido_depois_da_graca() {
        let manifesto_vigente =
            manifesto_com_partes(vec![parte("ibge/estados/exec-atual/parte.json")]);
        let manifesto_perdedor =
            manifesto_com_partes(vec![parte("ibge/estados/exec-perdedor/parte.json")]);
        let chave_manifesto_perdedor = "ibge/estados/exec-perdedor/_manifesto.json";
        let limpeza = LimpezaPendente {
            manifesto: chave_manifesto_perdedor.to_owned(),
            solicitada_em: Utc::now() - GRACA_SNAPSHOT - chrono::Duration::seconds(1),
        };
        let ponteiro = UltimaVersao {
            versao: None,
            manifesto: "ibge/estados/exec-atual/_manifesto.json".to_owned(),
            iniciada_em: None,
        };
        let chave_limpeza = chave_da_limpeza(chave_manifesto_perdedor);
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                "ibge/estados/_ultima_versao.json".to_owned(),
                json(&ponteiro),
            ),
            (
                "ibge/estados/exec-atual/_manifesto.json".to_owned(),
                json(&manifesto_vigente),
            ),
            (
                "ibge/estados/exec-atual/parte.json".to_owned(),
                b"atual".to_vec(),
            ),
            (
                chave_manifesto_perdedor.to_owned(),
                json(&manifesto_perdedor),
            ),
            (
                "ibge/estados/exec-perdedor/parte.json".to_owned(),
                b"perdedor".to_vec(),
            ),
            (chave_limpeza.clone(), json(&limpeza)),
        ]);

        let (removidas, falhas) =
            limpar_orfas_do_recurso(&armazenamento, "ibge/estados/", "corrente")
                .await
                .unwrap();

        assert_eq!(removidas, 3);
        assert!(falhas.is_empty());
        assert!(!armazenamento.contem(chave_manifesto_perdedor));
        assert!(!armazenamento.contem("ibge/estados/exec-perdedor/parte.json"));
        assert!(!armazenamento.contem(&chave_limpeza));
        assert!(armazenamento.contem("ibge/estados/exec-atual/parte.json"));
    }

    #[tokio::test]
    async fn marcador_permanece_quando_a_remocao_do_snapshot_falha() {
        let prefixo = "ibge/estados/execucao=e1/";
        let manifesto_chave = format!("{prefixo}_manifesto.json");
        let marcador_chave = format!("{prefixo}_limpeza_pendente.json");
        let parte_chave = format!("{prefixo}parte-00001.json");
        let mut manifesto = manifesto_com_partes(vec![parte(&parte_chave)]);
        manifesto.id_execucao = "e1".to_owned();
        let marcador = LimpezaPendente {
            manifesto: manifesto_chave.clone(),
            solicitada_em: Utc::now() - GRACA_SNAPSHOT - chrono::Duration::seconds(1),
        };
        let armazenamento = ArmazenamentoEmMemoria::com([
            (manifesto_chave.clone(), json(&manifesto)),
            (marcador_chave.clone(), json(&marcador)),
            (parte_chave.clone(), b"dados".to_vec()),
        ]);
        *armazenamento.falha_remocao.lock().unwrap() = Some(manifesto_chave.clone());

        let (_, falhas) = limpar_orfas_do_recurso(&armazenamento, "ibge/estados/", "outra")
            .await
            .unwrap();

        assert_eq!(falhas.len(), 1);
        assert!(armazenamento.contem(&marcador_chave));
        assert!(armazenamento.contem(&manifesto_chave));

        *armazenamento.falha_remocao.lock().unwrap() = None;
        limpar_orfas_do_recurso(&armazenamento, "ibge/estados/", "outra")
            .await
            .unwrap();
        assert!(!armazenamento.contem(&marcador_chave));
        assert!(!armazenamento.contem(&manifesto_chave));
        assert!(!armazenamento.contem(&parte_chave));
    }

    #[tokio::test]
    async fn remove_orfas_sem_manifesto() {
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                "ibge/estados/exec2/parte-00001.json".to_owned(),
                b"dados1".to_vec(),
            ),
            (
                "ibge/estados/exec2/parte-00002.json".to_owned(),
                b"dados2".to_vec(),
            ),
        ]);

        let (total, falhas) = limpar_orfas_do_recurso(&armazenamento, "ibge/estados/", "corrente")
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert!(falhas.is_empty());
        assert!(!armazenamento.contem("ibge/estados/exec2/parte-00001.json"));
        assert!(!armazenamento.contem("ibge/estados/exec2/parte-00002.json"));
    }

    #[tokio::test]
    async fn preserva_partes_de_execucao_vigente_e_remove_as_orfas() {
        let manifesto = manifesto_com_partes(vec![parte("ibge/estados/exec1/parte-00001.json")]);
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                "ibge/estados/exec1/_manifesto.json".to_owned(),
                json(&manifesto),
            ),
            (
                "ibge/estados/exec1/parte-00001.json".to_owned(),
                b"vigente".to_vec(),
            ),
            (
                "ibge/estados/exec2/parte-00001.json".to_owned(),
                b"orfa1".to_vec(),
            ),
            (
                "ibge/estados/exec2/parte-00002.json".to_owned(),
                b"orfa2".to_vec(),
            ),
        ]);

        let (total, falhas) = limpar_orfas_do_recurso(&armazenamento, "ibge/estados/", "corrente")
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert!(falhas.is_empty());
        assert!(armazenamento.contem("ibge/estados/exec1/_manifesto.json"));
        assert!(armazenamento.contem("ibge/estados/exec1/parte-00001.json"));
        assert!(!armazenamento.contem("ibge/estados/exec2/parte-00001.json"));
        assert!(!armazenamento.contem("ibge/estados/exec2/parte-00002.json"));
    }

    #[tokio::test]
    async fn execucao_atual_nunca_e_removida() {
        // Partes da execução corrente ainda sem manifesto — não são órfãs.
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                "ibge/estados/execucao=corrente/parte-00001.json".to_owned(),
                b"em andamento".to_vec(),
            ),
            (
                "ibge/estados/execucao=velha/parte-00001.json".to_owned(),
                b"orfa".to_vec(),
            ),
        ]);

        let (total, falhas) = limpar_orfas_do_recurso(&armazenamento, "ibge/estados/", "corrente")
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert!(falhas.is_empty());
        assert!(armazenamento.contem("ibge/estados/execucao=corrente/parte-00001.json"));
        assert!(!armazenamento.contem("ibge/estados/execucao=velha/parte-00001.json"));
    }

    #[tokio::test]
    async fn manifesto_ilegivel_protege_execucao_inteira() {
        // Manifesto presente mas corrompido — todas as partes sob essa execução
        // ficam protegidas (princípio conservador).
        let armazenamento = ArmazenamentoEmMemoria::com([
            (
                "ibge/estados/exec1/_manifesto.json".to_owned(),
                b"corrompido".to_vec(),
            ),
            (
                "ibge/estados/exec1/parte-00001.json".to_owned(),
                b"referenciada?".to_vec(),
            ),
            // Órfã real (outra execução):
            (
                "ibge/estados/exec2/parte-00001.json".to_owned(),
                b"orfa".to_vec(),
            ),
        ]);

        let (total, falhas) = limpar_orfas_do_recurso(&armazenamento, "ibge/estados/", "corrente")
            .await
            .unwrap();
        assert_eq!(total, 1);
        assert!(falhas.is_empty());
        // Protegidas apesar do manifesto ilegível:
        assert!(armazenamento.contem("ibge/estados/exec1/_manifesto.json"));
        assert!(armazenamento.contem("ibge/estados/exec1/parte-00001.json"));
        // Órfã removida:
        assert!(!armazenamento.contem("ibge/estados/exec2/parte-00001.json"));
    }
}
