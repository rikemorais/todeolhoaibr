//! Monta o [`EstadoNaRaw`] de um recurso: localiza o manifesto vigente pelo
//! ponteiro `_ultima_versao.json`, confere completude e integridade das partes.
//!
//! Diferente de [`crate::decisao`], que é regra pura, aqui há I/O, sempre pelo
//! porto [`ArmazenamentoRaw`]. O resultado alimenta [`crate::decisao::decidir`].

use sha2::{Digest, Sha256};

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
    let ponteiro: UltimaVersao =
        serde_json::from_slice(&bytes).map_err(|erro| erro_de_manifesto(recurso, erro))?;

    let Some(bytes_manifesto) = armazenamento.ler(&ponteiro.manifesto).await? else {
        return Ok(Some(EstadoNaRaw {
            completa: false,
            integra: false,
            versao: ponteiro.versao,
            tamanho: None,
        }));
    };
    let manifesto: ManifestoRaw = serde_json::from_slice(&bytes_manifesto)
        .map_err(|erro| erro_de_manifesto(recurso, erro))?;

    let tamanho = manifesto.partes.iter().map(|parte| parte.bytes).sum();
    let integra = partes_integras(armazenamento, &manifesto).await?;

    Ok(Some(EstadoNaRaw {
        completa: true,
        integra,
        versao: ponteiro.versao,
        tamanho: Some(tamanho),
    }))
}

/// Confere que cada parte do manifesto existe e bate em bytes e sha256.
async fn partes_integras(
    armazenamento: &dyn ArmazenamentoRaw,
    manifesto: &ManifestoRaw,
) -> Result<bool, ErroDeIngestao> {
    for parte in &manifesto.partes {
        let Some(conteudo) = armazenamento.ler(&parte.chave).await? else {
            return Ok(false);
        };
        if conteudo.len() as u64 != parte.bytes {
            return Ok(false);
        }
        if hex::encode(Sha256::digest(&conteudo)) != parte.sha256 {
            return Ok(false);
        }
    }
    Ok(true)
}

fn erro_de_manifesto(recurso: &str, erro: serde_json::Error) -> ErroDeIngestao {
    ErroDeIngestao::Manifesto {
        recurso: recurso.to_owned(),
        detalhe: erro.to_string(),
    }
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

    fn sha256(conteudo: &[u8]) -> String {
        hex::encode(Sha256::digest(conteudo))
    }

    /// Semeia um recurso íntegro e devolve o armazenamento.
    fn integro() -> ArmazenamentoEmMemoria {
        let ponteiro = UltimaVersao {
            versao: Some("v1".to_owned()),
            manifesto: CHAVE_MANIFESTO.to_owned(),
        };
        let manifesto = manifesto(parte(CONTEUDO.len() as u64, &sha256(CONTEUDO)));
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
    async fn parte_truncada_nao_e_integra() {
        // O manifesto diz que a parte tem mais bytes do que o conteúdo gravado.
        let ponteiro = UltimaVersao {
            versao: Some("v1".to_owned()),
            manifesto: CHAVE_MANIFESTO.to_owned(),
        };
        let manifesto = manifesto(parte(CONTEUDO.len() as u64 + 10, &sha256(CONTEUDO)));
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
    async fn hash_divergente_nao_e_integro() {
        // Mesmo tamanho, mas sha256 do manifesto não corresponde ao conteúdo.
        let ponteiro = UltimaVersao {
            versao: Some("v1".to_owned()),
            manifesto: CHAVE_MANIFESTO.to_owned(),
        };
        let manifesto = manifesto(parte(CONTEUDO.len() as u64, &sha256(b"outro conteudo")));
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
        assert!(!estado.integra);
    }

    #[tokio::test]
    async fn ponteiro_antigo_sem_campo_versao_desserializa() {
        // Formato gravado hoje pela TarefaDeArquivo: versao como string simples.
        let ponteiro = serde_json::json!({ "versao": "v1", "manifesto": CHAVE_MANIFESTO });
        let manifesto = manifesto(parte(CONTEUDO.len() as u64, &sha256(CONTEUDO)));
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
