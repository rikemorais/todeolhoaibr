//! Publicação dos agregados IBGE na camada Silver.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    ibge_agregados::{esquema_ibge_agregados, transformar_ibge_agregados},
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao},
};
use crate::SnapshotBronze;

const REGRA: &str = "ibge-agregado-v1";

/// Tarefa Silver para um recurso de agregados IBGE.
pub struct TarefaDeSilverIbge {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
}

impl TarefaDeSilverIbge {
    /// Cria uma tarefa para o recurso e snapshot Bronze selecionados.
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

    /// Indica se a versão Silver precisa acompanhar o snapshot Bronze atual.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "ibge", &self.recurso).await?,
            Some(ref estado)
                if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverIbge {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_ibge_agregados(
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

/// Transforma e publica um recurso IBGE a partir do snapshot recebido.
pub async fn publicar_ibge_agregados(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "ibge",
            recurso,
            regra: REGRA,
            esquema: esquema_ibge_agregados(),
            iniciada_em,
            id_execucao,
        },
        |lote| transformar_ibge_agregados(lote).map_err(|erro| erro.to_string()),
    )
    .await
}

#[cfg(test)]
mod testes {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use chrono::Utc;
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
        let linha = [
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
        ];
        let mut tabela = crate::Tabela::nova(colunas);
        tabela = tabela.com_linha(linha);
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
