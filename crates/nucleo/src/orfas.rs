//! Recuperação de partes Raw órfãs: objetos gravados durante ingestões que não
//! concluíram (erro ambíguo no manifesto, cancelamento abrupto, ou rollback
//! incompleto). A limpeza só remove objetos comprovadamente desprotegidos — sem
//! manifesto — e nunca atinge partes de ingestões vigentes.
//!
//! Ver issue #269 e PR #268 (revisão).

use std::collections::HashSet;
use std::time::Duration;

use tracing::{info, warn};

use crate::erros::ErroDeIngestao;
use crate::portas::{ArmazenamentoRaw, FalhaDeRemocao};
use crate::raw::ManifestoRaw;

/// Concorrência padrão para remoção de órfãs.
const CONCORRENCIA: usize = 4;
/// Deadline padrão para remoção de órfãs.
const DEADLINE: Duration = Duration::from_secs(30);

/// Verifica se o manifesto de uma execução realmente foi gravado e, se não,
/// remove as partes que ficariam desprotegidas.
///
/// Cenário: a gravação do manifesto recebeu erro de rede, mas o servidor pode
/// ter feito o commit. Esta função verifica o estado real antes de apagar.
///
/// Devolve `true` se o manifesto existe (partes protegidas), `false` se não
/// existia e as partes foram limpas.
pub async fn verificar_manifesto_ou_limpar(
    armazenamento: &dyn ArmazenamentoRaw,
    chave_manifesto: &str,
    partes_novas: &[String],
) -> Result<bool, ErroDeIngestao> {
    // Verifica se o manifesto realmente existe no armazenamento.
    match armazenamento.ler(chave_manifesto).await? {
        Some(bytes) => {
            // O manifesto existe — verificar se é válido e referencia estas partes.
            if serde_json::from_slice::<ManifestoRaw>(&bytes).is_ok() {
                info!(
                    manifesto = %chave_manifesto,
                    "manifesto confirmado após erro ambíguo; partes protegidas"
                );
                return Ok(true);
            }
            // Manifesto existe mas é ilegível — partes ficam desprotegidas.
            warn!(
                manifesto = %chave_manifesto,
                "manifesto ilegível após erro ambíguo; removendo partes órfãs"
            );
        }
        None => {
            info!(
                manifesto = %chave_manifesto,
                "manifesto ausente após erro ambíguo; removendo partes órfãs"
            );
        }
    }

    let falhas = armazenamento
        .remover_lote(partes_novas, CONCORRENCIA, DEADLINE)
        .await;
    for falha in &falhas {
        warn!(
            chave = %falha.chave,
            detalhe = %falha.detalhe,
            "falha ao remover parte órfã após verificação de manifesto"
        );
    }

    Ok(false)
}

/// Varre um prefixo no armazenamento e remove objetos que não pertencem a
/// nenhum manifesto vigente.
///
/// Funciona assim:
/// 1. Lista todas as chaves sob `prefixo_recurso` (ex.: `fonte/recurso/`).
/// 2. Para cada manifesto encontrado, coleta as partes referenciadas.
/// 3. Qualquer chave que não seja um manifesto, um ponteiro, e que não esteja
///    nas partes de nenhum manifesto, é uma órfã e será removida.
///
/// Devolve o número de órfãs encontradas (mesmo que nem todas tenham sido
/// removidas com sucesso).
pub async fn limpar_orfas_do_recurso(
    armazenamento: &dyn ArmazenamentoRaw,
    prefixo_recurso: &str,
) -> Result<(usize, Vec<FalhaDeRemocao>), ErroDeIngestao> {
    let chaves = armazenamento.listar_prefixo(prefixo_recurso).await?;
    if chaves.is_empty() {
        return Ok((0, Vec::new()));
    }

    // Coleta todas as chaves protegidas: manifestos, ponteiros e partes referenciadas.
    let mut protegidas = HashSet::new();
    for chave in &chaves {
        if chave.ends_with("/_manifesto.json") {
            protegidas.insert(chave.clone());
            // Lê o manifesto para encontrar partes referenciadas.
            if let Ok(Some(bytes)) = armazenamento.ler(chave).await
                && let Ok(manifesto) = serde_json::from_slice::<ManifestoRaw>(&bytes)
            {
                for parte in &manifesto.partes {
                    protegidas.insert(parte.chave.clone());
                }
            }
        }
        if chave.ends_with("/_ultima_versao.json") {
            protegidas.insert(chave.clone());
        }
    }

    // Órfãs: chaves que existem mas não são protegidas.
    let orfas: Vec<String> = chaves
        .into_iter()
        .filter(|chave| !protegidas.contains(chave))
        .collect();

    if orfas.is_empty() {
        return Ok((0, Vec::new()));
    }

    let total = orfas.len();
    info!(
        prefixo = %prefixo_recurso,
        orfas = total,
        "encontradas partes órfãs; iniciando limpeza"
    );

    let falhas = armazenamento
        .remover_lote(&orfas, CONCORRENCIA, DEADLINE)
        .await;
    for falha in &falhas {
        warn!(
            chave = %falha.chave,
            detalhe = %falha.detalhe,
            "falha ao remover parte órfã durante varredura"
        );
    }

    Ok((total, falhas))
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
    }

    impl ArmazenamentoEmMemoria {
        fn com(objetos: impl IntoIterator<Item = (String, Vec<u8>)>) -> Self {
            Self {
                objetos: Mutex::new(objetos.into_iter().collect()),
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

        let protegido = verificar_manifesto_ou_limpar(
            &armazenamento,
            "_manifesto.json",
            &["p1".to_owned(), "p2".to_owned()],
        )
        .await
        .unwrap();

        assert!(protegido);
        assert!(armazenamento.contem("p1"));
        assert!(armazenamento.contem("p2"));
    }

    #[tokio::test]
    async fn manifesto_ausente_limpa_partes() {
        let armazenamento = ArmazenamentoEmMemoria::com([
            ("p1".to_owned(), b"dados1".to_vec()),
            ("p2".to_owned(), b"dados2".to_vec()),
        ]);

        let protegido = verificar_manifesto_ou_limpar(
            &armazenamento,
            "_manifesto.json",
            &["p1".to_owned(), "p2".to_owned()],
        )
        .await
        .unwrap();

        assert!(!protegido);
        assert!(!armazenamento.contem("p1"));
        assert!(!armazenamento.contem("p2"));
    }

    #[tokio::test]
    async fn manifesto_ilegivel_limpa_partes() {
        let armazenamento = ArmazenamentoEmMemoria::com([
            ("_manifesto.json".to_owned(), b"nao e json".to_vec()),
            ("p1".to_owned(), b"dados".to_vec()),
        ]);

        let protegido =
            verificar_manifesto_ou_limpar(&armazenamento, "_manifesto.json", &["p1".to_owned()])
                .await
                .unwrap();

        assert!(!protegido);
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

        let (total, falhas) = limpar_orfas_do_recurso(&armazenamento, "ibge/estados/")
            .await
            .unwrap();
        assert_eq!(total, 0);
        assert!(falhas.is_empty());
        assert!(armazenamento.contem("ibge/estados/exec1/parte-00001.json"));
    }

    #[tokio::test]
    async fn remove_orfas_sem_manifesto() {
        // Duas partes de uma execução que não tem manifesto (cancelada).
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

        let (total, falhas) = limpar_orfas_do_recurso(&armazenamento, "ibge/estados/")
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert!(falhas.is_empty());
        assert!(!armazenamento.contem("ibge/estados/exec2/parte-00001.json"));
        assert!(!armazenamento.contem("ibge/estados/exec2/parte-00002.json"));
    }

    #[tokio::test]
    async fn preserva_partes_de_execucao_vigente_e_remove_as_orfas() {
        // Exec1: completa com manifesto. Exec2: cancelada, sem manifesto.
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
            // Órfãs de exec2 (sem manifesto):
            (
                "ibge/estados/exec2/parte-00001.json".to_owned(),
                b"orfa1".to_vec(),
            ),
            (
                "ibge/estados/exec2/parte-00002.json".to_owned(),
                b"orfa2".to_vec(),
            ),
        ]);

        let (total, falhas) = limpar_orfas_do_recurso(&armazenamento, "ibge/estados/")
            .await
            .unwrap();
        assert_eq!(total, 2);
        assert!(falhas.is_empty());
        // Vigentes preservadas:
        assert!(armazenamento.contem("ibge/estados/exec1/_manifesto.json"));
        assert!(armazenamento.contem("ibge/estados/exec1/parte-00001.json"));
        // Órfãs removidas:
        assert!(!armazenamento.contem("ibge/estados/exec2/parte-00001.json"));
        assert!(!armazenamento.contem("ibge/estados/exec2/parte-00002.json"));
    }
}
