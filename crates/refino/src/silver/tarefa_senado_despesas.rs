//! Tarefa Silver para a série anual de despesas CEAPS do Senado.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql_com_json_string_original},
    senado_despesas::esquema_senado_despesas,
};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

#[cfg(test)]
use super::senado_despesas::transformar_senado_despesas;

const REGRA: &str = "senado-despesa-ceaps-v4";
const SQL: &str = include_str!("../../sql/silver/senado_despesas.sql");

/// Tarefa de transformação de um recurso anual de despesas do Senado.
pub struct TarefaDeSilverSenadoDespesas {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    ano: u16,
}

impl TarefaDeSilverSenadoDespesas {
    /// Cria a tarefa para um recurso CEAPS que contém o ano no caminho.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Option<Self> {
        let recurso = recurso.into();
        let ano = ano_do_recurso(&recurso)?;
        Some(Self {
            bronze,
            snapshot,
            recurso,
            ano,
        })
    }

    /// Indica se a Silver precisa acompanhar o snapshot Bronze e a regra atuais.
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
impl Tarefa for TarefaDeSilverSenadoDespesas {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_senado_despesas(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            &self.recurso,
            self.ano,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Publica despesas tipadas e valida códigos de documento sem descartar linhas incompletas.
pub async fn publicar_senado_despesas(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    ano: u16,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    if ano_do_recurso(recurso) != Some(ano) {
        return Err(erro_recurso(recurso));
    }
    let sql = SQL.replace("{{ANO_RECURSO}}", &ano.to_string());
    publicar_transformacao_sql_com_json_string_original(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "senado",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_senado_despesas()),
            iniciada_em,
            id_execucao,
        },
        &sql,
        &ConfiguracaoExecutorSql::default(),
    )
    .await
}

/// Extrai o ano de um recurso `despesas/ano=AAAA`.
fn ano_do_recurso(recurso: &str) -> Option<u16> {
    let ano = recurso.strip_prefix("despesas/ano=")?;
    if ano.len() != 4 || !ano.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    ano.parse().ok()
}

fn erro_recurso(recurso: &str) -> ErroDeIngestao {
    ErroDeIngestao::Manifesto {
        recurso: recurso.to_owned(),
        detalhe: "recurso de despesas deve conter um ano válido".into(),
    }
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;

    use arrow::array::StringArray;
    use arrow::record_batch::RecordBatch;
    use async_trait::async_trait;
    use bytes::Bytes;
    use chrono::Utc;
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
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

    async fn snapshot() -> (Memoria, Arc<SnapshotBronze>) {
        let bronze = Memoria::default();
        let inicio = Utc::now();
        let recurso = "despesas/ano=2024";
        let local = LocalizacaoRaw::nova("senado", recurso, inicio, "bronze-1");

        let lote = RecordBatch::try_from_iter(vec![
            (
                "ANO",
                Arc::new(StringArray::from(vec![Some("2024"); 4])) as _,
            ),
            (
                "MES",
                Arc::new(StringArray::from(vec![
                    Some("02"),
                    Some("03"),
                    None,
                    Some("04"),
                ])) as _,
            ),
            (
                "SENADOR",
                Arc::new(StringArray::from(vec![Some("SENADORA EXEMPLO"); 4])) as _,
            ),
            (
                "TIPO_DESPESA",
                Arc::new(StringArray::from(vec![Some("Passagens"); 4])) as _,
            ),
            (
                "CNPJ_CPF",
                Arc::new(StringArray::from(vec![Some("00123456000199"); 4])) as _,
            ),
            (
                "FORNECEDOR",
                Arc::new(StringArray::from(vec![Some("Fornecedor"); 4])) as _,
            ),
            (
                "DOCUMENTO",
                Arc::new(StringArray::from(vec![Some("000123"); 4])) as _,
            ),
            (
                "DATA",
                Arc::new(StringArray::from(vec![
                    Some("04/02/2024"),
                    Some("10/03/2024"),
                    Some("data deslocada"),
                    Some("31/13/2024"),
                ])) as _,
            ),
            (
                "DETALHAMENTO",
                Arc::new(StringArray::from(vec![Some("Trecho"); 4])) as _,
            ),
            (
                "VALOR_REEMBOLSADO",
                Arc::new(StringArray::from(vec![
                    Some("1\r\n675,50"),
                    Some("-42,25"),
                    Some("2014675"),
                    Some(",95"),
                ])) as _,
            ),
            (
                "COD_DOCUMENTO",
                Arc::new(StringArray::from(vec![
                    Some("00045"),
                    Some("00046"),
                    None,
                    Some("00047"),
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
            url: "despesa_ceaps_2024.csv".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "senado".into(),
            recurso: recurso.into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "https://www.senado.leg.br/transparencia/LAI/verba/despesa_ceaps_2024.csv"
                .into(),
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
    async fn publica_recurso_anual_e_reaproveita_versao_atual() {
        let (bronze, snapshot) = snapshot().await;
        let parte_bronze = &snapshot.manifesto().partes[0];
        let lote_bronze = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(
            bronze.ler(&parte_bronze.chave).await.unwrap().unwrap(),
        ))
        .unwrap()
        .build()
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
        let esperado =
            transformar_senado_despesas(&lote_bronze, 2024, &mut Default::default()).unwrap();
        let silver = Memoria::default();
        let tarefa = TarefaDeSilverSenadoDespesas::nova(
            Arc::new(Memoria::default()),
            Arc::clone(&snapshot),
            "despesas/ano=2024",
        )
        .unwrap();
        assert!(tarefa.pendente(&silver).await.unwrap());
        let resumo = publicar_senado_despesas(
            &bronze,
            &silver,
            &snapshot,
            "despesas/ano=2024",
            2024,
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap();
        assert_eq!(resumo.partes, 1);
        let ponteiro: UltimaVersao = serde_json::from_slice(
            &silver
                .ler(&LocalizacaoRaw::ultima_versao(
                    "senado",
                    "despesas/ano=2024",
                ))
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let manifesto: ManifestoRaw =
            serde_json::from_slice(&silver.ler(&ponteiro.manifesto).await.unwrap().unwrap())
                .unwrap();
        let saida = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(
            silver
                .ler(&manifesto.partes[0].chave)
                .await
                .unwrap()
                .unwrap(),
        ))
        .unwrap()
        .build()
        .unwrap()
        .next()
        .unwrap()
        .unwrap();
        assert_eq!(saida, esperado);
        assert!(!tarefa.pendente(&silver).await.unwrap());
        assert!(
            publicar_senado_despesas(
                &bronze,
                &silver,
                &snapshot,
                "despesas/ano=2024",
                2024,
                Utc::now(),
                "silver-2",
            )
            .await
            .unwrap()
            .reaproveitada
        );
    }

    #[tokio::test]
    async fn rejeita_ano_divergente_e_recurso_sem_ano() {
        let (bronze, snapshot) = snapshot().await;
        let erro = publicar_senado_despesas(
            &bronze,
            &Memoria::default(),
            &snapshot,
            "despesas/ano=2024",
            2023,
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap_err();
        assert!(matches!(erro, ErroDeIngestao::Manifesto { .. }));
        assert!(ano_do_recurso("despesas/ano=x").is_none());
        assert!(ano_do_recurso("despesas/ano=2024/extra").is_none());
    }
}
