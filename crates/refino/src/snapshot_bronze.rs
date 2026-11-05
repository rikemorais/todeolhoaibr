//! Seleção e verificação de uma parte do manifesto bronze vigente.
//!
//! O snapshot fixa o ponteiro lido no início. A tarefa silver futura deverá
//! conferir novamente o ponteiro antes de publicar sua própria versão.

use std::io::Read;

use sha2::{Digest, Sha256};
use thiserror::Error;
use todeolho_nucleo::{
    ArmazenamentoRaw, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ParteRaw, UltimaVersao,
};

use crate::{ErroDeRefino, LeitorDeParteBronze};

/// Erros ao selecionar ou verificar uma parte bronze para a silver.
#[derive(Debug, Error)]
pub enum ErroDeSnapshotBronze {
    /// Falha ao acessar o bucket bronze.
    #[error(transparent)]
    Armazenamento(#[from] ErroDeIngestao),
    /// Falha ao criar ou ler o arquivo temporário.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Falha ao abrir o Parquet verificado.
    #[error(transparent)]
    Refino(#[from] ErroDeRefino),
    /// Ponteiro, manifesto ou parte inconsistentes.
    #[error("snapshot bronze inválido: {0}")]
    Invalido(String),
}

/// Manifesto selecionado pelo ponteiro vigente de um recurso bronze.
#[derive(Debug)]
pub struct SnapshotBronze {
    chave_manifesto: String,
    manifesto: ManifestoRaw,
}

impl SnapshotBronze {
    /// Consulta o ponteiro vigente e valida o manifesto apontado.
    ///
    /// `None` significa que ainda não há bronze publicado para o recurso.
    /// Ponteiro ou manifesto ausente após a leitura do ponteiro é um erro.
    pub async fn vigente(
        armazenamento: &dyn ArmazenamentoRaw,
        fonte: &str,
        recurso: &str,
    ) -> Result<Option<Self>, ErroDeSnapshotBronze> {
        let chave_ponteiro = LocalizacaoRaw::ultima_versao(fonte, recurso);
        let Some(bytes) = armazenamento.ler(&chave_ponteiro).await? else {
            return Ok(None);
        };
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes)
            .map_err(|erro| ErroDeSnapshotBronze::Invalido(format!("{chave_ponteiro}: {erro}")))?;
        if ponteiro.manifesto.is_empty() {
            return Err(ErroDeSnapshotBronze::Invalido(format!(
                "{chave_ponteiro}: chave de manifesto vazia"
            )));
        }
        let bytes_manifesto = armazenamento
            .ler(&ponteiro.manifesto)
            .await?
            .ok_or_else(|| {
                ErroDeSnapshotBronze::Invalido(format!(
                    "manifesto {} apontado por {chave_ponteiro} não encontrado",
                    ponteiro.manifesto
                ))
            })?;
        let manifesto: ManifestoRaw = serde_json::from_slice(&bytes_manifesto).map_err(|erro| {
            ErroDeSnapshotBronze::Invalido(format!("{}: {erro}", ponteiro.manifesto))
        })?;
        if manifesto.fonte != fonte || manifesto.recurso != recurso {
            return Err(ErroDeSnapshotBronze::Invalido(format!(
                "{} pertence a {}/{}, esperado {fonte}/{recurso}",
                ponteiro.manifesto, manifesto.fonte, manifesto.recurso
            )));
        }
        if manifesto.partes.is_empty() {
            return Err(ErroDeSnapshotBronze::Invalido(format!(
                "{} não contém partes",
                ponteiro.manifesto
            )));
        }
        Ok(Some(Self {
            chave_manifesto: ponteiro.manifesto,
            manifesto,
        }))
    }

    /// Chave do manifesto que identifica esta versão bronze.
    pub fn chave_manifesto(&self) -> &str {
        &self.chave_manifesto
    }

    /// Conteúdo do manifesto para selecionar tabelas e partes.
    pub fn manifesto(&self) -> &ManifestoRaw {
        &self.manifesto
    }

    /// Baixa uma parte listada no manifesto e valida tamanho e SHA-256.
    ///
    /// O download e a verificação usam arquivo temporário e memória limitada.
    pub async fn baixar_parte(
        &self,
        armazenamento: &dyn ArmazenamentoRaw,
        chave_parte: &str,
    ) -> Result<ParteBronzeBaixada, ErroDeSnapshotBronze> {
        let parte = self
            .manifesto
            .partes
            .iter()
            .find(|parte| parte.chave == chave_parte)
            .ok_or_else(|| {
                ErroDeSnapshotBronze::Invalido(format!(
                    "parte {chave_parte} não pertence a {}",
                    self.chave_manifesto
                ))
            })?;
        let arquivo = tempfile::NamedTempFile::new()?;
        let mut escrita = tokio::fs::File::from_std(arquivo.reopen()?);
        let existe = armazenamento.ler_fluxo(chave_parte, &mut escrita).await?;
        tokio::io::AsyncWriteExt::flush(&mut escrita).await?;
        drop(escrita);
        if !existe {
            return Err(ErroDeSnapshotBronze::Invalido(format!(
                "parte {chave_parte} do manifesto não encontrada"
            )));
        }
        let tamanho = arquivo.as_file().metadata()?.len();
        if tamanho != parte.bytes {
            return Err(ErroDeSnapshotBronze::Invalido(format!(
                "parte {chave_parte}: {tamanho} bytes, manifesto declara {}",
                parte.bytes
            )));
        }
        let mut leitura = arquivo.reopen()?;
        let mut digest = Sha256::new();
        let mut bloco = [0u8; 64 * 1024];
        loop {
            let lidos = leitura.read(&mut bloco)?;
            if lidos == 0 {
                break;
            }
            digest.update(&bloco[..lidos]);
        }
        let hash = hex::encode(digest.finalize());
        if hash != parte.sha256 {
            return Err(ErroDeSnapshotBronze::Invalido(format!(
                "parte {chave_parte}: SHA-256 diverge do manifesto"
            )));
        }
        Ok(ParteBronzeBaixada {
            arquivo,
            chave_manifesto: self.chave_manifesto.clone(),
            parte: parte.clone(),
        })
    }
}

/// Arquivo temporário verificado contra uma parte do snapshot bronze.
pub struct ParteBronzeBaixada {
    arquivo: tempfile::NamedTempFile,
    chave_manifesto: String,
    parte: ParteRaw,
}

impl ParteBronzeBaixada {
    /// Abre a parte verificada para leitura sequencial em lotes.
    ///
    /// Mantenha este valor vivo até terminar de consumir o leitor.
    pub fn abrir_lotes(&self, tamanho_lote: usize) -> Result<LeitorDeParteBronze, ErroDeRefino> {
        LeitorDeParteBronze::abrir(
            self.arquivo.path(),
            &self.chave_manifesto,
            &self.parte,
            tamanho_lote,
        )
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use todeolho_nucleo::ObjetoRaw;
    use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

    use super::*;
    use crate::Tabela;

    struct ArmazenamentoFalso {
        objetos: BTreeMap<String, Vec<u8>>,
        leituras_em_fluxo: AtomicUsize,
    }

    #[async_trait]
    impl ArmazenamentoRaw for ArmazenamentoFalso {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }

        async fn gravar(&self, _: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            unreachable!()
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
            Ok(self.objetos.get(chave).cloned())
        }

        async fn ler_fluxo(
            &self,
            chave: &str,
            escrita: &mut (dyn AsyncWrite + Send + Unpin),
        ) -> Result<bool, ErroDeIngestao> {
            self.leituras_em_fluxo.fetch_add(1, Ordering::Relaxed);
            let Some(bytes) = self.objetos.get(chave) else {
                return Ok(false);
            };
            for bloco in bytes.chunks(17) {
                escrita
                    .write_all(bloco)
                    .await
                    .map_err(|erro| ErroDeIngestao::Armazenamento {
                        chave: chave.to_owned(),
                        detalhe: erro.to_string(),
                    })?;
            }
            Ok(true)
        }
    }

    fn fixture() -> (ArmazenamentoFalso, String) {
        let parte_chave = "tse/prestacao-contas/ano=2024/execucao=abc/parte-00001.parquet";
        let manifesto_chave = "tse/prestacao-contas/ano=2024/execucao=abc/_manifesto.json";
        let parquet = Tabela::nova(["SQ_DESPESA", "VR_DESPESA"])
            .com_linha([Some("10".into()), Some("100".into())])
            .com_linha([Some("10".into()), Some("200".into())])
            .para_parquet()
            .unwrap();
        let instante = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let manifesto = ManifestoRaw {
            fonte: "tse".into(),
            recurso: "prestacao-contas".into(),
            id_execucao: "abc".into(),
            url_inicial: "raw/tse/prestacao-contas".into(),
            iniciada_em: instante,
            concluida_em: instante,
            partes: vec![ParteRaw {
                chave: parte_chave.into(),
                url: "despesas_contratadas_candidatos_2024_BRASIL".into(),
                bytes: parquet.len() as u64,
                sha256: hex::encode(Sha256::digest(&parquet)),
                tipo_de_conteudo: "application/vnd.apache.parquet".into(),
                versao: None,
            }],
        };
        let ponteiro = UltimaVersao {
            versao: None,
            manifesto: manifesto_chave.into(),
            iniciada_em: Some(instante),
        };
        let objetos = BTreeMap::from([
            (
                LocalizacaoRaw::ultima_versao("tse", "prestacao-contas"),
                serde_json::to_vec(&ponteiro).unwrap(),
            ),
            (
                manifesto_chave.into(),
                serde_json::to_vec(&manifesto).unwrap(),
            ),
            (parte_chave.into(), parquet),
        ]);
        (
            ArmazenamentoFalso {
                objetos,
                leituras_em_fluxo: AtomicUsize::new(0),
            },
            parte_chave.into(),
        )
    }

    #[tokio::test]
    async fn baixa_parte_do_manifesto_vigente_e_preserva_linhas() {
        let (armazenamento, chave) = fixture();
        let snapshot = SnapshotBronze::vigente(&armazenamento, "tse", "prestacao-contas")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.manifesto().partes.len(), 1);
        let parte = snapshot.baixar_parte(&armazenamento, &chave).await.unwrap();
        let lote = parte
            .abrir_lotes(1)
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert_eq!(lote.len(), 2);
        assert_eq!(lote[0].referencia(0).unwrap().ordinal, 1);
        assert_eq!(lote[1].referencia(0).unwrap().ordinal, 2);
        assert_eq!(lote[0].origem.manifesto, snapshot.chave_manifesto());
        assert_eq!(lote[0].origem.parte, chave);
        assert_eq!(armazenamento.leituras_em_fluxo.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn rejeita_parte_fora_do_manifesto_sem_baixar() {
        let (armazenamento, _) = fixture();
        let snapshot = SnapshotBronze::vigente(&armazenamento, "tse", "prestacao-contas")
            .await
            .unwrap()
            .unwrap();
        assert!(
            snapshot
                .baixar_parte(&armazenamento, "outra.parquet")
                .await
                .is_err()
        );
        assert_eq!(armazenamento.leituras_em_fluxo.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn rejeita_parte_corrompida_e_manifesto_de_outro_recurso() {
        let (mut armazenamento, chave) = fixture();
        armazenamento.objetos.get_mut(&chave).unwrap()[5] ^= 1;
        let snapshot = SnapshotBronze::vigente(&armazenamento, "tse", "prestacao-contas")
            .await
            .unwrap()
            .unwrap();
        assert!(snapshot.baixar_parte(&armazenamento, &chave).await.is_err());

        let chave_ponteiro = LocalizacaoRaw::ultima_versao("tse", "outro-recurso");
        let ponteiro = armazenamento
            .objetos
            .get(&LocalizacaoRaw::ultima_versao("tse", "prestacao-contas"))
            .unwrap()
            .clone();
        armazenamento.objetos.insert(chave_ponteiro, ponteiro);
        assert!(
            SnapshotBronze::vigente(&armazenamento, "tse", "outro-recurso")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn rejeita_parte_ausente_ou_com_tamanho_diferente() {
        let (mut armazenamento, chave) = fixture();
        let snapshot = SnapshotBronze::vigente(&armazenamento, "tse", "prestacao-contas")
            .await
            .unwrap()
            .unwrap();
        armazenamento.objetos.get_mut(&chave).unwrap().pop();
        assert!(snapshot.baixar_parte(&armazenamento, &chave).await.is_err());
        armazenamento.objetos.remove(&chave);
        assert!(snapshot.baixar_parte(&armazenamento, &chave).await.is_err());
    }

    #[tokio::test]
    async fn distingue_ausencia_de_ponteiro_invalido() {
        let (mut armazenamento, _) = fixture();
        assert!(
            SnapshotBronze::vigente(&armazenamento, "tse", "ausente")
                .await
                .unwrap()
                .is_none()
        );
        armazenamento.objetos.insert(
            LocalizacaoRaw::ultima_versao("tse", "ausente"),
            b"nao e json".to_vec(),
        );
        assert!(
            SnapshotBronze::vigente(&armazenamento, "tse", "ausente")
                .await
                .is_err()
        );
    }
}
