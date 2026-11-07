//! Tarefa Silver para os lançamentos anuais da CEAP da Câmara.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    camara_despesas::esquema_camara_despesas,
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql_com_json_string_original},
};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

const REGRA: &str = "camara-despesa-ceap-v3";
const SQL: &str = include_str!("../../sql/silver/camara_despesas.sql");

/// Tarefa Silver para um recurso anual de despesas da Câmara.
pub struct TarefaDeSilverCamaraDespesas {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    ano: u16,
}

impl TarefaDeSilverCamaraDespesas {
    /// Cria a tarefa se o caminho do recurso declarar um ano válido.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Option<Self> {
        let recurso = recurso.into();
        let ano = recurso
            .split('/')
            .find_map(|segmento| segmento.strip_prefix("ano="))?
            .parse::<u16>()
            .ok()?;
        Some(Self {
            bronze,
            snapshot,
            recurso,
            ano,
        })
    }

    /// Indica se a Silver acompanha o snapshot Bronze e a regra atuais.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "camara", &self.recurso).await?,
            Some(ref estado)
                if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverCamaraDespesas {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_camara_despesas(
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

/// Publica despesas tipadas, mantendo todos os lançamentos e os campos originais.
pub async fn publicar_camara_despesas(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    ano: u16,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    if snapshot.manifesto().fonte != "camara"
        || snapshot.manifesto().recurso != recurso
        || !recurso.starts_with("despesas/ano=")
        || recurso
            .split('/')
            .find_map(|segmento| segmento.strip_prefix("ano="))
            .and_then(|texto| texto.parse::<u16>().ok())
            != Some(ano)
    {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: "snapshot não corresponde ao recurso anual de despesas da Câmara".into(),
        });
    }
    let sql = SQL.replace("{{ANO_RECURSO}}", &ano.to_string());
    publicar_transformacao_sql_com_json_string_original(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "camara",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_camara_despesas()),
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

    use arrow::array::StringArray;
    use arrow::record_batch::RecordBatch;
    use async_trait::async_trait;
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

    fn parquet() -> Vec<u8> {
        let colunas = [
            ("txNomeParlamentar", "Deputado Exemplo"),
            ("nuDeputadoId", "00123"),
            ("ideDocumento", "00045"),
            ("txtCNPJCPF", "00123456000199"),
            ("datEmissao", "2024-03-07T00:00:00"),
            ("datPagamentoRestituicao", ""),
            ("vlrDocumento", "1234.56789"),
            ("vlrGlosa", "0"),
            ("vlrLiquido", "1234.56789"),
            ("vlrRestituicao", ""),
        ];
        let lote = RecordBatch::try_from_iter(colunas.into_iter().map(|(nome, valor)| {
            (
                nome,
                std::sync::Arc::new(StringArray::from(vec![valor])) as _,
            )
        }))
        .unwrap();
        let mut bytes = Vec::new();
        let mut escritor =
            parquet::arrow::ArrowWriter::try_new(&mut bytes, lote.schema(), None).unwrap();
        escritor.write(&lote).unwrap();
        escritor.close().unwrap();
        bytes
    }

    async fn snapshot(bronze: &Memoria) -> SnapshotBronze {
        let iniciada_em = Utc::now();
        let recurso = "despesas/ano=2024";
        let local = LocalizacaoRaw::nova("camara", recurso, iniciada_em, "bronze-1");
        let bytes = parquet();
        let parte = ParteRaw {
            chave: local.parte(1, "parquet"),
            url: "https://www.camara.leg.br/cotas/Ano-2024.csv.zip".into(),
            bytes: bytes.len() as u64,
            sha256: hex::encode(Sha256::digest(&bytes)),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: Some("raw-1".into()),
        };
        let manifesto = ManifestoRaw {
            fonte: "camara".into(),
            recurso: recurso.into(),
            id_execucao: "bronze-1".into(),
            url_inicial: "https://www.camara.leg.br/cotas/Ano-2024.csv.zip".into(),
            iniciada_em,
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
                chave: LocalizacaoRaw::ultima_versao("camara", recurso),
                conteudo: serde_json::to_vec(&UltimaVersao {
                    versao: Some("bronze-1".into()),
                    manifesto: local.manifesto(),
                    iniciada_em: Some(iniciada_em),
                })
                .unwrap(),
                tipo_de_conteudo: "application/json".into(),
            },
        ] {
            bronze.gravar(objeto).await.unwrap();
        }
        SnapshotBronze::vigente(bronze, "camara", recurso)
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn publica_e_reaproveita_o_recurso_anual_vigente() {
        let bronze = Memoria::default();
        let silver = Memoria::default();
        let snapshot = Arc::new(snapshot(&bronze).await);
        let recurso = "despesas/ano=2024";
        let tarefa = TarefaDeSilverCamaraDespesas::nova(
            Arc::new(Memoria::default()),
            Arc::clone(&snapshot),
            recurso,
        )
        .unwrap();

        assert!(tarefa.pendente(&silver).await.unwrap());
        let resultado = publicar_camara_despesas(
            &bronze,
            &silver,
            &snapshot,
            recurso,
            2024,
            Utc::now(),
            "silver-1",
        )
        .await
        .unwrap();

        assert_eq!(resultado.partes, 1);
        assert!(!tarefa.pendente(&silver).await.unwrap());
    }

    #[tokio::test]
    async fn recusa_recurso_que_nao_corresponde_ao_snapshot_ou_ano() {
        let bronze = Memoria::default();
        let silver = Memoria::default();
        let atual = snapshot(&bronze).await;

        assert!(
            TarefaDeSilverCamaraDespesas::nova(
                Arc::new(Memoria::default()),
                Arc::new(snapshot(&bronze).await),
                "despesas/sem-ano",
            )
            .is_none()
        );
        assert!(
            publicar_camara_despesas(
                &bronze,
                &silver,
                &atual,
                "despesas/ano=2023",
                2023,
                Utc::now(),
                "silver-erro",
            )
            .await
            .is_err()
        );
    }
}
