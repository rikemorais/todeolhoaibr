//! Tarefa Silver para as votações anuais do Senado.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao},
    senado_votacoes::transformar_senado_votacoes,
};
use crate::SnapshotBronze;

const REGRA: &str = "senado-votacao-v1";

/// Tarefa para publicar um recurso anual de votações do Senado.
pub struct TarefaDeSilverSenadoVotacoes {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
}

impl TarefaDeSilverSenadoVotacoes {
    /// Cria a tarefa para o snapshot Bronze de um ano.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Self {
        Self {
            bronze,
            snapshot,
            recurso: recurso.into(),
        }
    }

    /// Indica se a Silver acompanha o snapshot e a regra de transformação atuais.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "senado", &self.recurso).await?,
            Some(ref estado) if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverSenadoVotacoes {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_senado_votacoes(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            &self.recurso,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Valida a chave da sessão e publica a estrutura original, mantendo os votos aninhados.
pub async fn publicar_senado_votacoes(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    if !recurso.starts_with("votacoes/ano=") {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: "recurso de votações sem ano no caminho".into(),
        });
    }
    let mut codigos_vistos = HashSet::new();
    publicar_transformacao(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "senado",
            recurso,
            regra: REGRA,
            esquema: None,
            iniciada_em,
            id_execucao,
        },
        |lote| {
            transformar_senado_votacoes(lote, &mut codigos_vistos).map_err(|erro| erro.to_string())
        },
    )
    .await
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;

    use arrow::{array::StringArray, record_batch::RecordBatch};
    use async_trait::async_trait;
    use chrono::Utc;
    use sha2::{Digest, Sha256};
    use todeolho_nucleo::{LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw, UltimaVersao};
    use tokio::io::{AsyncRead, AsyncReadExt};

    use super::*;

    #[derive(Default)]
    struct Memoria(std::sync::Mutex<HashMap<String, Vec<u8>>>);

    #[async_trait]
    impl ArmazenamentoRaw for Memoria {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }
        async fn gravar(&self, objeto: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            self.0.lock().unwrap().insert(objeto.chave, objeto.conteudo);
            Ok(())
        }
        async fn gravar_fluxo(
            &self,
            chave: &str,
            _: &str,
            fluxo: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            let mut bytes = Vec::new();
            fluxo
                .read_to_end(&mut bytes)
                .await
                .map_err(|erro| ErroDeIngestao::Armazenamento {
                    chave: chave.into(),
                    detalhe: erro.to_string(),
                })?;
            self.0.lock().unwrap().insert(chave.into(), bytes);
            Ok(())
        }
        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.0.lock().unwrap().get(chave).cloned())
        }
    }

    async fn fonte(ids: &[&str]) -> (Memoria, Arc<SnapshotBronze>) {
        let bronze = Memoria::default();
        let inicio = Utc::now();
        let recurso = "votacoes/ano=2024";
        let local = LocalizacaoRaw::nova("senado", recurso, inicio, "bronze-1");
        let datas = vec!["2024-02-01"; ids.len()];
        let votos = vec![r#"[{"codigoParlamentar":"7","voto":"Sim"}]"#; ids.len()];
        let lote = RecordBatch::try_from_iter(vec![
            (
                "codigoSessaoVotacao",
                Arc::new(StringArray::from(ids.to_vec())) as _,
            ),
            ("dataSessao", Arc::new(StringArray::from(datas)) as _),
            ("votos", Arc::new(StringArray::from(votos)) as _),
        ])
        .unwrap();
        let mut bytes = Vec::new();
        let mut writer =
            parquet::arrow::ArrowWriter::try_new(&mut bytes, lote.schema(), None).unwrap();
        writer.write(&lote).unwrap();
        writer.close().unwrap();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "votacao?ano=2024".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "senado".into(),
            recurso: recurso.into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "https://legis.senado.leg.br/dadosabertos/votacao?ano=2024".into(),
            iniciada_em: inicio,
            concluida_em: Utc::now(),
            partes: vec![parte.clone()],
        };
        for objeto in [
            ObjetoRaw {
                chave: parte.chave,
                conteudo: bytes,
                tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            },
            ObjetoRaw {
                chave: local.manifesto(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            },
            ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao("senado", recurso),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some("bronze-1".into()),
                    manifesto: local.manifesto(),
                    iniciada_em: Some(inicio),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            },
        ] {
            bronze.gravar(objeto).await.unwrap();
        }
        let snapshot = SnapshotBronze::vigente(&bronze, "senado", recurso)
            .await
            .unwrap()
            .unwrap();
        (bronze, Arc::new(snapshot))
    }

    #[tokio::test]
    async fn publica_e_reaproveita_sessao_preservando_votos() {
        let (bronze, snapshot) = fonte(&["S1"]).await;
        let silver = Memoria::default();
        let recurso = "votacoes/ano=2024";
        let tarefa = TarefaDeSilverSenadoVotacoes::nova(
            Arc::new(Memoria::default()),
            snapshot.clone(),
            recurso,
        );
        assert!(tarefa.pendente(&silver).await.unwrap());
        let resumo =
            publicar_senado_votacoes(&bronze, &silver, &snapshot, recurso, Utc::now(), "silver-1")
                .await
                .unwrap();
        assert_eq!(resumo.partes, 1);
        assert!(!tarefa.pendente(&silver).await.unwrap());
        assert!(
            publicar_senado_votacoes(&bronze, &silver, &snapshot, recurso, Utc::now(), "silver-2",)
                .await
                .unwrap()
                .reaproveitada
        );
    }

    #[tokio::test]
    async fn nao_publica_recurso_que_nao_seja_votacao_anual() {
        let (bronze, snapshot) = fonte(&["S1"]).await;
        assert!(
            publicar_senado_votacoes(
                &bronze,
                &Memoria::default(),
                &snapshot,
                "votacoes/recente",
                Utc::now(),
                "silver-1",
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn rejeita_duplicata_no_recurso_e_anula_publicacao() {
        let (bronze, snapshot) = fonte(&["S1", "S1"]).await;
        let silver = Memoria::default();
        assert!(
            publicar_senado_votacoes(
                &bronze,
                &silver,
                &snapshot,
                "votacoes/ano=2024",
                Utc::now(),
                "silver-1",
            )
            .await
            .is_err()
        );
        assert!(
            silver
                .ler(&LocalizacaoRaw::ultima_versao(
                    "senado",
                    "votacoes/ano=2024"
                ))
                .await
                .unwrap()
                .is_none()
        );
    }
}
