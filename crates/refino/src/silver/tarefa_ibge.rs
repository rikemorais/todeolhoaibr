//! Publicação dos agregados IBGE na camada Silver.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa,
};

use super::{
    ibge_agregados::esquema_ibge_agregados, publicacao::ConfiguracaoDePublicacao,
    tarefa_sql::TarefaDeSilverSql,
};
use crate::SnapshotBronze;

const REGRA: &str = "ibge-agregado-v2";
const SQL: &str = include_str!("../../sql/silver/ibge_agregados.sql");

/// Tarefa Silver para um recurso de agregados IBGE.
pub struct TarefaDeSilverIbge {
    tarefa_sql: TarefaDeSilverSql,
}

impl TarefaDeSilverIbge {
    /// Cria uma tarefa para o recurso e snapshot Bronze selecionados.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Self {
        let recurso = recurso.into();
        Self {
            tarefa_sql: TarefaDeSilverSql::nova(
                bronze,
                snapshot,
                "ibge",
                recurso.clone(),
                None,
                REGRA,
                SQL,
                esquema_ibge_agregados(),
            ),
        }
    }

    /// Indica se a versão Silver precisa acompanhar o snapshot Bronze atual.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        self.tarefa_sql.pendente(silver).await
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverIbge {
    fn nome(&self) -> &str {
        self.tarefa_sql.nome()
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        self.tarefa_sql.executar(contexto).await
    }
}

/// Transforma e publica um recurso IBGE a partir do snapshot recebido.
pub async fn publicar_ibge_agregados(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    super::publicacao::publicar_transformacao_sql(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "ibge",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_ibge_agregados()),
            iniciada_em,
            id_execucao,
        },
        SQL,
        &crate::executor_sql::ConfiguracaoExecutorSql::default(),
    )
    .await
}

#[cfg(test)]
mod testes {
    use std::collections::{HashMap, HashSet};

    use arrow::array::{Array, Date32Array, StringArray};
    use async_trait::async_trait;
    use bytes::Bytes;
    use chrono::Utc;
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use sha2::{Digest, Sha256};
    use todeolho_nucleo::{
        ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw, UltimaVersao,
    };
    use tokio::io::{AsyncRead, AsyncReadExt};

    use super::*;
    use crate::SnapshotBronze;

    const TIPO_PARQUET: &str = "application/vnd.apache.parquet";

    #[derive(Default)]
    struct ArmazenamentoEmMemoria {
        objetos: std::sync::Mutex<HashMap<String, Vec<u8>>>,
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
            chave: &str,
            _tipo_de_conteudo: &str,
            fluxo: &mut (dyn AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            let mut conteudo = Vec::new();
            fluxo.read_to_end(&mut conteudo).await.map_err(|erro| {
                ErroDeIngestao::Armazenamento {
                    chave: chave.into(),
                    detalhe: erro.to_string(),
                }
            })?;
            self.objetos.lock().unwrap().insert(chave.into(), conteudo);
            Ok(())
        }

        async fn ler(&self, chave: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(self.objetos.lock().unwrap().get(chave).cloned())
        }
    }

    fn fixture_parquet() -> Vec<u8> {
        let colunas = [
            "variavel_id",
            "variavel",
            "unidade",
            "classificacoes",
            "nivel_id",
            "nivel_nome",
            "localidade_id",
            "localidade_nome",
            "periodo",
            "valor",
        ];
        let mut tabela = crate::Tabela::nova(colunas);
        tabela = tabela
            .com_linha([
                Some("1737".to_owned()),
                Some("IPCA".to_owned()),
                Some("%".to_owned()),
                None,
                Some("N1".to_owned()),
                Some("Brasil".to_owned()),
                Some("1".to_owned()),
                Some("Brasil".to_owned()),
                Some("2025".to_owned()),
                Some("4.56".to_owned()),
            ])
            .com_linha([
                Some("1737".to_owned()),
                Some("IPCA".to_owned()),
                Some("%".to_owned()),
                Some("cat=1".to_owned()),
                Some("N6".to_owned()),
                Some("Município".to_owned()),
                Some("001".to_owned()),
                Some("Cidade".to_owned()),
                Some("2025".to_owned()),
                Some("texto especial".to_owned()),
            ])
            .com_linha([
                Some("1737".to_owned()),
                Some("IPCA".to_owned()),
                Some("%".to_owned()),
                Some("cat=1".to_owned()),
                Some("N6".to_owned()),
                Some("Município".to_owned()),
                Some("002".to_owned()),
                Some("Outra".to_owned()),
                Some("2025".to_owned()),
                None,
            ]);
        tabela.para_parquet().unwrap()
    }

    async fn snapshot_ibge(bronze: &ArmazenamentoEmMemoria) -> SnapshotBronze {
        let recurso = "ipca/1737";
        let iniciada_em = Utc::now();
        let local = LocalizacaoRaw::nova("ibge", recurso, iniciada_em, "bronze-1");
        let bytes = fixture_parquet();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "https://servicodados.ibge.gov.br/api/v3/agregados/1737".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: TIPO_PARQUET.into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "ibge".into(),
            recurso: recurso.into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "https://servicodados.ibge.gov.br/api/v3/agregados/1737".into(),
            iniciada_em,
            concluida_em: Utc::now(),
            partes: vec![parte.clone()],
        };
        bronze
            .gravar(ObjetoRaw {
                chave: parte.chave,
                conteudo: bytes,
                tipo_de_conteudo: TIPO_PARQUET.into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: local.manifesto(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao("ibge", recurso),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some("bronze-1".into()),
                    manifesto: local.manifesto(),
                    iniciada_em: Some(iniciada_em),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        SnapshotBronze::vigente(bronze, "ibge", recurso)
            .await
            .unwrap()
            .unwrap()
    }

    async fn snapshot_bcb(
        bronze: &ArmazenamentoEmMemoria,
        recurso: &str,
        id_execucao: &str,
        linhas: &[[Option<String>; 2]],
    ) -> SnapshotBronze {
        let iniciada_em = Utc::now();
        let local = LocalizacaoRaw::nova("bcb", recurso, iniciada_em, id_execucao);
        let mut tabela = crate::Tabela::nova(["data", "valor"]);
        for linha in linhas {
            tabela = tabela.com_linha(linha.clone());
        }
        let bytes = tabela.para_parquet().unwrap();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "fixture-bcb".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: TIPO_PARQUET.into(),
            versao: Some(id_execucao.into()),
        };
        bronze
            .gravar(ObjetoRaw {
                chave: parte.chave.clone(),
                conteudo: bytes,
                tipo_de_conteudo: TIPO_PARQUET.into(),
            })
            .await
            .unwrap();
        let manifesto = ManifestoRaw {
            fonte: "bcb".into(),
            recurso: recurso.into(),
            id_execucao: id_execucao.into(),
            url_inicial: "fixture-bcb".into(),
            iniciada_em,
            concluida_em: Utc::now(),
            partes: vec![parte],
        };
        bronze
            .gravar(ObjetoRaw {
                chave: local.manifesto(),
                conteudo: serde_json::to_vec(&manifesto).unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        bronze
            .gravar(ObjetoRaw {
                chave: LocalizacaoRaw::ultima_versao("bcb", recurso),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some(id_execucao.into()),
                    manifesto: local.manifesto(),
                    iniciada_em: Some(iniciada_em),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            })
            .await
            .unwrap();
        SnapshotBronze::vigente(bronze, "bcb", recurso)
            .await
            .unwrap()
            .unwrap()
    }

    async fn lote_silver(
        armazenamento: &ArmazenamentoEmMemoria,
        fonte: &str,
        recurso: &str,
    ) -> arrow::record_batch::RecordBatch {
        let bytes_ponteiro = armazenamento
            .ler(&LocalizacaoRaw::ultima_versao(fonte, recurso))
            .await
            .unwrap()
            .unwrap();
        let ponteiro: UltimaVersao = serde_json::from_slice(&bytes_ponteiro).unwrap();
        let bytes_manifesto = armazenamento
            .ler(&ponteiro.manifesto)
            .await
            .unwrap()
            .unwrap();
        let manifesto: ManifestoRaw = serde_json::from_slice(&bytes_manifesto).unwrap();
        let bytes_parquet = armazenamento
            .ler(&manifesto.partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        ParquetRecordBatchReaderBuilder::try_new(Bytes::from(bytes_parquet))
            .unwrap()
            .build()
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn publica_parquet_tipado_e_reaproveita_versao_vigente() {
        let bronze = ArmazenamentoEmMemoria::default();
        let silver = ArmazenamentoEmMemoria::default();
        bronze.preparar().await.unwrap();
        silver.preparar().await.unwrap();
        let snapshot = Arc::new(snapshot_ibge(&bronze).await);
        let tarefa = TarefaDeSilverIbge::nova(
            Arc::new(ArmazenamentoEmMemoria::default()),
            Arc::clone(&snapshot),
            "ipca/1737",
        );
        assert_eq!(tarefa.nome(), "ipca/1737");
        assert!(tarefa.pendente(&silver).await.unwrap());

        let primeiro = publicar_ibge_agregados(
            &bronze,
            &silver,
            &snapshot,
            "ipca/1737",
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap();
        assert_eq!(primeiro.partes, 1);
        assert!(primeiro.bytes > 0);
        let bytes_bronze = fixture_parquet();
        let entrada_legada = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(bytes_bronze))
            .unwrap()
            .build()
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let esperado =
            super::super::ibge_agregados::transformar_ibge_agregados(&entrada_legada).unwrap();
        let publicado = lote_silver(&silver, "ibge", "ipca/1737").await;
        assert_eq!(publicado, esperado);
        let chaves = (0..publicado.num_rows())
            .map(|linha| {
                [0, 3, 4, 6, 8]
                    .into_iter()
                    .map(|coluna| {
                        let valores = publicado
                            .column(coluna)
                            .as_any()
                            .downcast_ref::<StringArray>()
                            .unwrap();
                        (!valores.is_null(linha)).then(|| valores.value(linha).to_owned())
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<HashSet<_>>();
        assert_eq!(chaves.len(), publicado.num_rows());
        assert!(!tarefa.pendente(&silver).await.unwrap());

        let reaproveitado = publicar_ibge_agregados(
            &bronze,
            &silver,
            &snapshot,
            "ipca/1737",
            Utc::now(),
            "silver-2",
        )
        .await
        .unwrap();
        assert!(reaproveitado.reaproveitada);
    }

    #[tokio::test]
    async fn contrato_sql_bcb_preserva_resultado_da_transformacao_arrow_legada() {
        let bronze = ArmazenamentoEmMemoria::default();
        let silver = ArmazenamentoEmMemoria::default();
        let recurso = "series/fixture";
        let snapshot = snapshot_bcb(
            &bronze,
            recurso,
            "bronze-bcb-1",
            &[
                [Some("01/06/1989".into()), Some("19.68".into())],
                [Some("02/06/1989".into()), Some("-0.125".into())],
            ],
        )
        .await;
        let bytes_bronze = bronze
            .ler(&snapshot.manifesto().partes[0].chave)
            .await
            .unwrap()
            .unwrap();
        let entrada = ParquetRecordBatchReaderBuilder::try_new(Bytes::from(bytes_bronze))
            .unwrap()
            .build()
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let esperado = super::super::bcb::transformar_bcb_sgs(&entrada).unwrap();

        crate::publicar_bcb_sgs(
            &bronze,
            &silver,
            &snapshot,
            recurso,
            Utc::now(),
            "silver-bcb-1",
        )
        .await
        .unwrap();

        let publicado = lote_silver(&silver, "bcb", recurso).await;
        assert_eq!(publicado, esperado);
        let datas = publicado
            .column(0)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        let chaves = datas.values().iter().copied().collect::<HashSet<_>>();
        assert_eq!(chaves.len(), publicado.num_rows());
    }

    #[tokio::test]
    async fn contrato_sql_bcb_rejeita_data_e_decimal_invalidos_sem_publicar() {
        for (id, linhas) in [
            (
                "bronze-data-invalida",
                vec![[Some("31/02/2024".into()), Some("1.25".into())]],
            ),
            (
                "bronze-decimal-invalido",
                vec![[Some("01/06/1989".into()), Some("1.1234567".into())]],
            ),
            ("bronze-data-nula", vec![[None, Some("1.25".into())]]),
            ("bronze-valor-nulo", vec![[Some("01/06/1989".into()), None]]),
        ] {
            let bronze = ArmazenamentoEmMemoria::default();
            let silver = ArmazenamentoEmMemoria::default();
            let recurso = "series/fixture";
            let snapshot = snapshot_bcb(&bronze, recurso, id, &linhas).await;
            let resultado = crate::publicar_bcb_sgs(
                &bronze,
                &silver,
                &snapshot,
                recurso,
                Utc::now(),
                "silver-invalida",
            )
            .await;

            assert!(resultado.is_err(), "a fixture {id} deveria ser rejeitada");
            assert!(
                silver
                    .ler(&LocalizacaoRaw::ultima_versao("bcb", recurso))
                    .await
                    .unwrap()
                    .is_none(),
                "a fixture {id} não pode publicar o ponteiro"
            );
        }
    }

    #[tokio::test]
    async fn rejeita_snapshot_de_outro_recurso() {
        let bronze = ArmazenamentoEmMemoria::default();
        let silver = ArmazenamentoEmMemoria::default();
        let snapshot = snapshot_ibge(&bronze).await;
        let erro = publicar_ibge_agregados(
            &bronze,
            &silver,
            &snapshot,
            "inpc/1736",
            Utc::now(),
            "silver-erro",
        )
        .await
        .unwrap_err();
        assert!(erro.to_string().contains("snapshot não pertence"));
    }

    #[tokio::test]
    async fn falha_se_a_parte_do_snapshot_desaparecer() {
        let bronze = ArmazenamentoEmMemoria::default();
        let silver = ArmazenamentoEmMemoria::default();
        let snapshot = snapshot_ibge(&bronze).await;
        let chave_parte = snapshot.manifesto().partes[0].chave.clone();
        bronze.objetos.lock().unwrap().remove(&chave_parte);
        let erro = publicar_ibge_agregados(
            &bronze,
            &silver,
            &snapshot,
            "ipca/1737",
            Utc::now(),
            "silver-parte-ausente",
        )
        .await
        .unwrap_err();
        assert!(erro.to_string().contains("parte"));
    }
}
