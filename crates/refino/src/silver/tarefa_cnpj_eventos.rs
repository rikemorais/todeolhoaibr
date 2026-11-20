//! Tarefa Silver que compara dois snapshots Bronze e produz eventos de diff.

use std::io::Read;
use std::sync::Arc;

use arrow::datatypes::SchemaRef;
use async_trait::async_trait;
use chrono::Utc;
use futures::StreamExt;
use sha2::{Digest, Sha256};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw,
    ParteRaw, ResumoDaTarefa, Tarefa, UltimaVersao, bloquear_publicacao, estado_na_raw,
    publicar_ponteiro_sob_guarda,
};

use crate::{
    EscritorParquetTipado, SnapshotBronze,
    executor_sql::{ConfiguracaoExecutorSql, executar_sql_duplo, preparar_snapshot_duplo},
};

const TIPO_PARQUET: &str = "application/vnd.apache.parquet";

/// Tarefa que compara dois snapshots Bronze consecutivos e publica eventos de diff.
pub struct TarefaDeSilverCnpjEventos {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot_vigente: Arc<SnapshotBronze>,
    snapshot_anterior: Arc<SnapshotBronze>,
    fonte: String,
    recurso: String,
    recurso_destino: String,
    regra: String,
    sql: &'static str,
    esquema: SchemaRef,
    executor: ConfiguracaoExecutorSql,
}

impl TarefaDeSilverCnpjEventos {
    /// Cria a tarefa de diff entre dois snapshots Bronze consecutivos.
    #[allow(clippy::too_many_arguments)]
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot_vigente: Arc<SnapshotBronze>,
        snapshot_anterior: Arc<SnapshotBronze>,
        fonte: impl Into<String>,
        recurso: impl Into<String>,
        recurso_destino: impl Into<String>,
        regra: impl Into<String>,
        sql: &'static str,
        esquema: SchemaRef,
    ) -> Self {
        Self {
            bronze,
            snapshot_vigente,
            snapshot_anterior,
            fonte: fonte.into(),
            recurso: recurso.into(),
            recurso_destino: recurso_destino.into(),
            regra: regra.into(),
            sql,
            esquema,
            executor: ConfiguracaoExecutorSql::default(),
        }
    }

    fn versao_silver(&self) -> String {
        format!(
            "{}+{}|regra={}",
            self.snapshot_vigente.manifesto().id_execucao,
            self.snapshot_anterior.manifesto().id_execucao,
            self.regra
        )
    }

    /// Retorna `true` se a versão Silver ainda não corresponde ao par de manifestos.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = self.versao_silver();
        Ok(!matches!(
            estado_na_raw(silver, &self.fonte, &self.recurso_destino).await?,
            Some(ref estado)
                if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

fn erro_manifesto(recurso: &str, detalhe: impl std::fmt::Display) -> ErroDeIngestao {
    ErroDeIngestao::Armazenamento {
        chave: recurso.to_owned(),
        detalhe: detalhe.to_string(),
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverCnpjEventos {
    fn nome(&self) -> &str {
        &self.recurso_destino
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let versao_silver = self.versao_silver();
        if let Some(estado) = estado_na_raw(
            contexto.armazenamento.as_ref(),
            &self.fonte,
            &self.recurso_destino,
        )
        .await?
        .filter(|e| e.completa && e.integra && e.versao.as_deref() == Some(versao_silver.as_str()))
        {
            let _ = estado;
            return Ok(ResumoDaTarefa {
                reaproveitada: true,
                ..ResumoDaTarefa::default()
            });
        }

        let entrada = preparar_snapshot_duplo(
            self.bronze.as_ref(),
            &self.snapshot_vigente,
            &self.snapshot_anterior,
            self.executor.limite_entrada_bytes,
        )
        .await
        .map_err(|erro| erro_manifesto(&self.recurso, format!("preparação do diff: {erro}")))?;

        let diretorio_spill = entrada.diretorio_execucao().join("spill");
        std::fs::create_dir(&diretorio_spill)
            .map_err(|_| erro_manifesto(&self.recurso, "falha ao criar diretório de spill"))?;

        let sql = self
            .sql
            .replace(
                "{{MANIFESTO_VIGENTE}}",
                self.snapshot_vigente.chave_manifesto(),
            )
            .replace(
                "{{MANIFESTO_ANTERIOR}}",
                self.snapshot_anterior.chave_manifesto(),
            );

        let mut fluxo = executar_sql_duplo(
            entrada.diretorio_vigente(),
            entrada.diretorio_anterior(),
            &diretorio_spill,
            &sql,
            &self.executor,
        )
        .await
        .map_err(|erro| {
            erro_manifesto(
                &self.recurso,
                format!("falha ao planejar contrato SQL de eventos: {erro}"),
            )
        })?;

        let local = LocalizacaoRaw::nova(
            &self.fonte,
            &self.recurso_destino,
            contexto.iniciada_em,
            &contexto.id_execucao,
        );
        let arquivo = tempfile::Builder::new()
            .prefix("todeolho-silver-eventos-")
            .suffix(".parquet")
            .tempfile_in(entrada.diretorio_execucao())
            .map_err(|_| {
                erro_manifesto(&self.recurso, "falha ao criar Parquet Silver temporário")
            })?;
        let mut escritor = EscritorParquetTipado::novo_em(
            arquivo.reopen().map_err(|_| {
                erro_manifesto(&self.recurso, "falha ao abrir Parquet Silver temporário")
            })?,
            self.esquema.clone(),
        )
        .map_err(|_| erro_manifesto(&self.recurso, "falha ao iniciar escritor Parquet Silver"))?;

        while let Some(lote) = fluxo.next().await {
            let lote = lote.map_err(|erro| {
                erro_manifesto(
                    &self.recurso,
                    format!("falha na execução do contrato SQL de eventos: {erro}"),
                )
            })?;
            escritor.escrever_lote(&lote).map_err(|_| {
                erro_manifesto(&self.recurso, "resultado SQL diverge do schema de saída")
            })?;
        }
        drop(fluxo);
        escritor
            .fechar()
            .map_err(|_| erro_manifesto(&self.recurso, "falha ao finalizar Parquet Silver"))?;

        let bytes = arquivo
            .as_file()
            .metadata()
            .map_err(|_| erro_manifesto(&self.recurso, "falha ao medir Parquet Silver"))?
            .len();
        let mut hash = Sha256::new();
        let mut leitura = arquivo
            .reopen()
            .map_err(|_| erro_manifesto(&self.recurso, "falha ao ler Parquet Silver temporário"))?;
        let mut bloco = [0u8; 64 * 1024];
        loop {
            let lidos = leitura.read(&mut bloco).map_err(|_| {
                erro_manifesto(&self.recurso, "falha ao calcular hash do Parquet Silver")
            })?;
            if lidos == 0 {
                break;
            }
            hash.update(&bloco[..lidos]);
        }

        let chave = local.parte(1, "parquet");
        let mut fluxo_upload = tokio::fs::File::from_std(
            arquivo
                .reopen()
                .map_err(|_| erro_manifesto(&self.recurso, "falha ao reabrir Parquet Silver"))?,
        );
        contexto
            .armazenamento
            .gravar_fluxo(&chave, TIPO_PARQUET, &mut fluxo_upload)
            .await?;

        let guarda = bloquear_publicacao(&self.fonte, &self.recurso_destino).await;
        let manifesto = ManifestoRaw {
            fonte: self.fonte.clone(),
            recurso: self.recurso_destino.clone(),
            id_execucao: contexto.id_execucao.clone(),
            url_inicial: format!("bronze/{}", self.recurso),
            iniciada_em: contexto.iniciada_em,
            concluida_em: Utc::now(),
            partes: vec![ParteRaw {
                chave,
                url: self.recurso.clone(),
                bytes,
                sha256: hex::encode(hash.finalize()),
                tipo_de_conteudo: TIPO_PARQUET.into(),
                versao: Some(versao_silver.clone()),
            }],
        };
        contexto
            .armazenamento
            .gravar(ObjetoRaw {
                chave: local.manifesto(),
                conteudo: serde_json::to_vec_pretty(&manifesto)
                    .map_err(|erro| erro_manifesto(&self.recurso, erro))?,
                tipo_de_conteudo: "application/json".into(),
            })
            .await?;
        let publicou = publicar_ponteiro_sob_guarda(
            contexto.armazenamento.as_ref(),
            &self.fonte,
            &self.recurso_destino,
            &UltimaVersao {
                versao: Some(versao_silver),
                manifesto: local.manifesto(),
                iniciada_em: Some(contexto.iniciada_em),
            },
            &guarda,
        )
        .await?;
        if !publicou {
            return Err(erro_manifesto(
                &self.recurso,
                "ponteiro Silver mais recente preservado; esta execução não foi publicada",
            ));
        }
        Ok(ResumoDaTarefa {
            partes: 1,
            bytes,
            ..ResumoDaTarefa::default()
        })
    }
}
