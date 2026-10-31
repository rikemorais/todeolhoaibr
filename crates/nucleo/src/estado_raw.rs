//! Monta o [`EstadoNaRaw`] de um recurso: localiza o manifesto vigente pelo
//! ponteiro `_ultima_versao.json`, confere completude e integridade das partes.
//!
//! Diferente de [`crate::decisao`], que é regra pura, aqui há I/O, sempre pelo
//! porto [`ArmazenamentoRaw`]. O resultado alimenta [`crate::decisao::decidir`].

use crate::decisao::EstadoNaRaw;
use crate::erros::ErroDeIngestao;
use crate::portas::ArmazenamentoRaw;
use crate::raw::{LocalizacaoRaw, ManifestoRaw, UltimaVersao};

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
}
