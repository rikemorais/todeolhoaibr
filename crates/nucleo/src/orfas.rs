//! Recuperação de partes Raw órfãs: objetos gravados durante ingestões que não
//! concluíram (erro ambíguo no manifesto, cancelamento abrupto, ou rollback
//! incompleto). A limpeza só remove objetos comprovadamente desprotegidos — sem
//! manifesto — e nunca atinge partes de ingestões vigentes.
//!
//! Ver issue #269 e PR #278 (revisão).

use std::collections::HashSet;
use std::time::Duration;

use tracing::{info, warn};

use crate::erros::ErroDeIngestao;
use crate::portas::{ArmazenamentoRaw, FalhaDeRemocao};
use crate::raw::ManifestoRaw;

const CONCORRENCIA: usize = 4;
const DEADLINE: Duration = Duration::from_secs(30);

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

/// Varre um prefixo no armazenamento e remove objetos que não pertencem a
/// nenhum manifesto vigente nem à execução corrente.
///
/// `id_execucao_atual` exclui partes da ingestão em andamento para evitar
/// colisão quando duas execuções do mesmo recurso se sobrepõem.
///
/// Se um manifesto listado não puder ser lido ou desserializado, todas as
/// chaves sob a mesma execução são consideradas protegidas (princípio
/// conservador: na dúvida, não apagar).
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

    // Protege tudo da execução corrente.
    for chave in &chaves {
        if chave.contains(&format!("execucao={id_execucao_atual}/")) {
            protegidas.insert(chave.clone());
        }
    }

    for chave in &chaves {
        if chave.ends_with("/_manifesto.json") {
            protegidas.insert(chave.clone());
            match armazenamento.ler(chave).await {
                Ok(Some(bytes)) => match serde_json::from_slice::<ManifestoRaw>(&bytes) {
                    Ok(manifesto) => {
                        for parte in &manifesto.partes {
                            protegidas.insert(parte.chave.clone());
                        }
                    }
                    Err(_) => {
                        // Manifesto ilegível: proteger TODAS as chaves sob a mesma
                        // execução para não apagar dados referenciados.
                        warn!(
                            manifesto = %chave,
                            "manifesto ilegível; protegendo toda a execução"
                        );
                        let prefixo_exec = prefixo_da_execucao(chave);
                        for outra in &chaves {
                            if outra.starts_with(&prefixo_exec) {
                                protegidas.insert(outra.clone());
                            }
                        }
                    }
                },
                Ok(None) | Err(_) => {
                    // Listou mas não conseguiu ler: proteger a execução inteira.
                    warn!(
                        manifesto = %chave,
                        "falha ao ler manifesto listado; protegendo toda a execução"
                    );
                    let prefixo_exec = prefixo_da_execucao(chave);
                    for outra in &chaves {
                        if outra.starts_with(&prefixo_exec) {
                            protegidas.insert(outra.clone());
                        }
                    }
                }
            }
        }
        if chave.ends_with("/_ultima_versao.json") {
            protegidas.insert(chave.clone());
        }
    }

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

    let falhas = remover_e_logar(armazenamento, &orfas, "varredura").await;
    Ok((total, falhas))
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
