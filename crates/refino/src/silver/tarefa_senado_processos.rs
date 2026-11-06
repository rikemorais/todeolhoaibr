//! Tarefa Silver para o catálogo anual de processos do Senado.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao},
    senado_processos::{esquema_senado_processos, transformar_senado_processos},
};
use crate::SnapshotBronze;

const REGRA: &str = "senado-processo-v1";

/// Tarefa para publicar um recurso anual do catálogo de processos do Senado.
pub struct TarefaDeSilverSenadoProcessos {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
}

impl TarefaDeSilverSenadoProcessos {
    /// Cria a tarefa para um snapshot Bronze de processos.
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

    /// Indica se a Silver acompanha o snapshot Bronze e a regra atuais.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "senado", &self.recurso).await?,
            Some(ref estado)
                if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverSenadoProcessos {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_senado_processos(
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

/// Publica a listagem de processos com os campos de origem preservados como JSON.
pub async fn publicar_senado_processos(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    let Some(ano) = recurso.strip_prefix("processos/ano=") else {
        return Err(erro_recurso(recurso));
    };
    if ano.len() != 4 || !ano.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(erro_recurso(recurso));
    }
    let ano = ano.parse::<u16>().map_err(|_| erro_recurso(recurso))?;
    let mut ids_vistos = HashSet::new();
    publicar_transformacao(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "senado",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_senado_processos()),
            iniciada_em,
            id_execucao,
        },
        |lote| {
            transformar_senado_processos(lote, ano, &mut ids_vistos)
                .map_err(|erro| erro.to_string())
        },
    )
    .await
}

fn erro_recurso(recurso: &str) -> ErroDeIngestao {
    ErroDeIngestao::Manifesto {
        recurso: recurso.to_owned(),
        detalhe: "recurso de processos deve conter um ano válido".into(),
    }
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;

    use arrow::{array::StringArray, record_batch::RecordBatch};
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

    async fn fonte() -> (Memoria, Arc<SnapshotBronze>) {
        let bronze = Memoria::default();
        let inicio = Utc::now();
        let recurso = "processos/ano=2024";
        let local = LocalizacaoRaw::nova("senado", recurso, inicio, "bronze-1");
        let lote = RecordBatch::try_from_iter(vec![
            ("id", Arc::new(StringArray::from(vec![Some("00825")])) as _),
            (
                "ementa",
                Arc::new(StringArray::from(vec![Some("Proposta")])) as _,
            ),
        ])
        .unwrap();
        let mut bytes = Vec::new();
        let mut writer =
            parquet::arrow::ArrowWriter::try_new(&mut bytes, lote.schema(), None).unwrap();
        writer.write(&lote).unwrap();
        writer.close().unwrap();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "processo?ano=2024".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "senado".into(),
            recurso: recurso.into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "https://legis.senado.leg.br/dadosabertos/processo?ano=2024".into(),
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
    async fn publica_e_reaproveita_o_catalogo_do_ano() {
        let (bronze, snapshot) = fonte().await;
        let silver = Memoria::default();
        let recurso = "processos/ano=2024";
        let tarefa = TarefaDeSilverSenadoProcessos::nova(
            Arc::new(Memoria::default()),
            snapshot.clone(),
            recurso,
        );
        assert!(tarefa.pendente(&silver).await.unwrap());
        let resumo =
            publicar_senado_processos(&bronze, &silver, &snapshot, recurso, Utc::now(), "silver-1")
                .await
                .unwrap();
        assert_eq!(resumo.partes, 1);
        assert!(!tarefa.pendente(&silver).await.unwrap());
        assert!(
            publicar_senado_processos(&bronze, &silver, &snapshot, recurso, Utc::now(), "silver-2")
                .await
                .unwrap()
                .reaproveitada
        );
    }

    #[tokio::test]
    async fn recusa_recurso_fora_do_padrao_anual() {
        let (bronze, snapshot) = fonte().await;
        assert!(
            publicar_senado_processos(
                &bronze,
                &Memoria::default(),
                &snapshot,
                "processos/recente",
                Utc::now(),
                "silver-1"
            )
            .await
            .is_err()
        );
    }
}
