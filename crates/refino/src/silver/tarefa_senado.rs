//! Tarefa Silver de parlamentares do Senado por legislatura.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql},
    senado_parlamentares::esquema_senado_parlamentares,
};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

const REGRA: &str = "senado-parlamentar-v2";
const SQL: &str = include_str!("../../sql/silver/senado_parlamentares.sql");

/// Tarefa Silver para a listagem de uma legislatura do Senado.
pub struct TarefaDeSilverSenadoParlamentares {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    legislatura: u16,
}

impl TarefaDeSilverSenadoParlamentares {
    /// Cria a tarefa a partir do snapshot Bronze e da legislatura do recurso.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
        legislatura: u16,
    ) -> Self {
        Self {
            bronze,
            snapshot,
            recurso: recurso.into(),
            legislatura,
        }
    }

    /// Indica se a versão Silver está pendente para o snapshot Bronze recebido.
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
impl Tarefa for TarefaDeSilverSenadoParlamentares {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_senado_parlamentares(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            &self.recurso,
            self.legislatura,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Publica a transformação de parlamentares usando o fluxo Silver compartilhado.
pub async fn publicar_senado_parlamentares(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    legislatura: u16,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    let sql = SQL.replace("{{LEGISLATURA}}", &legislatura.to_string());
    publicar_transformacao_sql(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "senado",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_senado_parlamentares()),
            iniciada_em,
            id_execucao,
        },
        &sql,
        &ConfiguracaoExecutorSql::default(),
    )
    .await
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;

    use arrow::{array::StringArray, record_batch::RecordBatch};
    use async_trait::async_trait;
    use bytes::Bytes;
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

    async fn fonte(identificacoes: &[Option<&str>]) -> (Memoria, Arc<SnapshotBronze>) {
        let bronze = Memoria::default();
        let inicio = Utc::now();
        let recurso = "parlamentares/legislatura=57";
        let local = LocalizacaoRaw::nova("senado", recurso, inicio, "bronze-1");
        let lote = RecordBatch::try_from_iter(vec![
            (
                "IdentificacaoParlamentar",
                Arc::new(StringArray::from(identificacoes.to_vec())) as _,
            ),
            (
                "Mandato",
                Arc::new(StringArray::from(vec![
                    Some("mandato original");
                    identificacoes.len()
                ])) as _,
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
            url: "parlamentares/legislatura=57.json".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "senado".into(),
            recurso: recurso.into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "https://legis.senado.leg.br/dadosabertos/parlamentares".into(),
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
    async fn sql_preserva_identificadores_textuais_e_objeto_original() {
        let (bronze, snapshot) = fonte(&[
            Some(r#"{"CodigoParlamentar":"00825","NomeParlamentar":"A"}"#),
            Some(r#"{"CodigoParlamentar":123,"NomeParlamentar":"B"}"#),
        ])
        .await;
        let silver = Memoria::default();
        publicar_senado_parlamentares(
            &bronze,
            &silver,
            &snapshot,
            "parlamentares/legislatura=57",
            57,
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(
            &silver
                .ler(&LocalizacaoRaw::ultima_versao(
                    "senado",
                    "parlamentares/legislatura=57",
                ))
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let bytes = silver
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let saida = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(
            Bytes::from(bytes),
        )
        .unwrap()
        .build()
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
        let codigos = saida
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(codigos.value(0), "00825");
        assert_eq!(codigos.value(1), "123");
        assert_eq!(saida.num_rows(), 2);
    }

    #[tokio::test]
    async fn codigo_parlamentar_ausente_impede_publicacao() {
        for json in [
            r#"{"NomeParlamentar":"A"}"#,
            r#"{"IdentificacaoParlamentar":{"CodigoParlamentar":false}}"#,
        ] {
            let (bronze, snapshot) = fonte(&[Some(json)]).await;
            let silver = Memoria::default();
            assert!(
                publicar_senado_parlamentares(
                    &bronze,
                    &silver,
                    &snapshot,
                    "parlamentares/legislatura=57",
                    57,
                    Utc::now(),
                    "silver-1",
                )
                .await
                .is_err(),
                "JSON aceito inesperadamente: {json}"
            );
            assert!(
                silver
                    .ler(&LocalizacaoRaw::ultima_versao(
                        "senado",
                        "parlamentares/legislatura=57",
                    ))
                    .await
                    .unwrap()
                    .is_none(),
                "publicou versão para JSON inválido: {json}"
            );
        }
    }
}
