//! Tarefa Silver dos conjuntos anuais de votações da Câmara.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    camara_votacoes::{ConjuntoCamaraVotacoes, validar_camara_votacoes},
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao},
};
use crate::SnapshotBronze;

/// Tarefa Silver que valida e publica um recurso anual de votações.
pub struct TarefaDeSilverCamaraVotacoes {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    conjunto: ConjuntoCamaraVotacoes,
}

impl TarefaDeSilverCamaraVotacoes {
    /// Cria a tarefa para um recurso Bronze anual.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Option<Self> {
        let recurso = recurso.into();
        let conjunto = ConjuntoCamaraVotacoes::de_recurso(&recurso)?;
        Some(Self {
            bronze,
            snapshot,
            recurso,
            conjunto,
        })
    }

    fn regra(&self) -> String {
        format!("camara-{}-v1", self.conjunto.nome())
    }

    /// Indica se a Silver acompanha o snapshot Bronze e a regra atuais.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!(
            "{}|regra={}",
            self.snapshot.manifesto().id_execucao,
            self.regra()
        );
        Ok(!matches!(
            estado_na_raw(silver, "camara", &self.recurso).await?,
            Some(ref estado) if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverCamaraVotacoes {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_camara_votacoes(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            &self.recurso,
            self.conjunto,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Valida a chave anual e preserva todas as colunas e linhas da origem.
pub async fn publicar_camara_votacoes(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    conjunto: ConjuntoCamaraVotacoes,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    if ConjuntoCamaraVotacoes::de_recurso(recurso) != Some(conjunto) {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: "conjunto não corresponde ao recurso Bronze".into(),
        });
    }
    let regra = format!("camara-{}-v1", conjunto.nome());
    let mut chaves = HashSet::new();
    publicar_transformacao(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "camara",
            recurso,
            recurso_destino: None,
            regra: &regra,
            esquema: None,
            iniciada_em,
            id_execucao,
        },
        |lote| {
            validar_camara_votacoes(conjunto, lote, &mut chaves).map_err(|erro| erro.to_string())
        },
    )
    .await
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;

    use arrow::array::StringArray;
    use arrow::record_batch::RecordBatch;
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

    async fn fonte_com_parquet(ids_deputado: &[&str]) -> (Memoria, Arc<SnapshotBronze>) {
        let bronze = Memoria::default();
        let inicio = Utc::now();
        let recurso = "votacoes/votacoesVotos/ano=2024";
        let local = LocalizacaoRaw::nova("camara", recurso, inicio, "bronze-1");
        let votacoes = vec!["V1"; ids_deputado.len()];
        let votos = vec!["Sim"; ids_deputado.len()];
        let lote = RecordBatch::try_from_iter(vec![
            ("idVotacao", Arc::new(StringArray::from(votacoes)) as _),
            (
                "deputado_id",
                Arc::new(StringArray::from(ids_deputado.to_vec())) as _,
            ),
            ("tipoVoto", Arc::new(StringArray::from(votos)) as _),
        ])
        .unwrap();
        let mut bytes = Vec::new();
        let mut writer =
            parquet::arrow::ArrowWriter::try_new(&mut bytes, lote.schema(), None).unwrap();
        writer.write(&lote).unwrap();
        writer.close().unwrap();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "votacoesVotos-2024.csv".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "camara".into(), recurso: recurso.into(), id_execucao: "bronze-1".into(),
            url_inicial: "https://dadosabertos.camara.leg.br/arquivos/votacoesVotos/csv/votacoesVotos-2024.csv".into(),
            iniciada_em: inicio, concluida_em: Utc::now(), partes: vec![parte.clone()],
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
                chave: LocalizacaoRaw::ultima_versao("camara", recurso),
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
        let snapshot = SnapshotBronze::vigente(&bronze, "camara", recurso)
            .await
            .unwrap()
            .unwrap();
        (bronze, Arc::new(snapshot))
    }

    #[tokio::test]
    async fn publica_schema_de_origem_e_recusa_chave_duplicada() {
        let (bronze, snapshot) = fonte_com_parquet(&["D1"]).await;
        let silver = Memoria::default();
        let recurso = "votacoes/votacoesVotos/ano=2024";
        let publicado = publicar_camara_votacoes(
            &bronze,
            &silver,
            &snapshot,
            recurso,
            ConjuntoCamaraVotacoes::VotacoesVotos,
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap();
        assert_eq!(publicado.partes, 1);
        let tarefa = TarefaDeSilverCamaraVotacoes::nova(
            Arc::new(Memoria::default()),
            snapshot.clone(),
            recurso,
        )
        .unwrap();
        assert!(!tarefa.pendente(&silver).await.unwrap());
        assert!(
            publicar_camara_votacoes(
                &bronze,
                &silver,
                &snapshot,
                recurso,
                ConjuntoCamaraVotacoes::VotacoesVotos,
                Utc::now(),
                "silver-2",
            )
            .await
            .is_ok()
        );

        let (bronze_duplicada, snapshot_duplicado) = fonte_com_parquet(&["D1", "D1"]).await;
        let resultado = publicar_camara_votacoes(
            &bronze_duplicada,
            &Memoria::default(),
            &snapshot_duplicado,
            recurso,
            ConjuntoCamaraVotacoes::VotacoesVotos,
            Utc::now(),
            "silver-3",
        )
        .await;
        assert!(resultado.is_err());
    }
}
