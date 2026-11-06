//! Publicação dos agregados IBGE na camada Silver.

use std::io::Read;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw,
    ParteRaw, ResumoDaTarefa, Tarefa, UltimaVersao, estado_na_raw, publicar_ponteiro,
};

use super::ibge_agregados::{esquema_ibge_agregados, transformar_ibge_agregados};
use crate::{EscritorParquetTipado, SnapshotBronze};

const REGRA: &str = "ibge-agregado-v1";
const TIPO_PARQUET: &str = "application/vnd.apache.parquet";

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
    if snapshot.manifesto().fonte != "ibge" || snapshot.manifesto().recurso != recurso {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: "snapshot não pertence ao recurso IBGE solicitado".into(),
        });
    }
    let versao_silver = format!("{}|regra={REGRA}", snapshot.manifesto().id_execucao);
    if let Some(estado) = estado_na_raw(silver, "ibge", recurso).await?
        && estado.completa
        && estado.integra
        && estado.versao.as_deref() == Some(versao_silver.as_str())
    {
        return Ok(ResumoDaTarefa {
            reaproveitada: true,
            ..ResumoDaTarefa::default()
        });
    }

    let local = LocalizacaoRaw::nova("ibge", recurso, iniciada_em, id_execucao);
    let mut partes = Vec::new();
    let mut total_bytes = 0u64;
    for entrada in &snapshot.manifesto().partes {
        let baixada = snapshot
            .baixar_parte(bronze, &entrada.chave)
            .await
            .map_err(|erro| erro_manifesto(recurso, erro))?;
        let mut leitor = baixada
            .abrir_lotes(65_536)
            .map_err(|erro| erro_manifesto(recurso, erro))?;
        let arquivo =
            tempfile::NamedTempFile::new().map_err(|erro| erro_manifesto(recurso, erro))?;
        let mut escritor = EscritorParquetTipado::novo_em(
            arquivo
                .reopen()
                .map_err(|erro| erro_manifesto(recurso, erro))?,
            esquema_ibge_agregados(),
        )
        .map_err(|erro| erro_manifesto(recurso, erro))?;
        for lote in &mut leitor {
            let registros = lote
                .map_err(|erro| erro_manifesto(recurso, erro))?
                .registros;
            let transformado = transformar_ibge_agregados(&registros)
                .map_err(|erro| erro_manifesto(recurso, erro))?;
            escritor
                .escrever_lote(&transformado)
                .map_err(|erro| erro_manifesto(recurso, erro))?;
        }
        escritor
            .fechar()
            .map_err(|erro| erro_manifesto(recurso, erro))?;

        let bytes = arquivo
            .as_file()
            .metadata()
            .map_err(|erro| erro_manifesto(recurso, erro))?
            .len();
        total_bytes = total_bytes.saturating_add(bytes);
        let mut hash = Sha256::new();
        let mut leitura = arquivo
            .reopen()
            .map_err(|erro| erro_manifesto(recurso, erro))?;
        let mut bloco = [0u8; 64 * 1024];
        loop {
            let lidos = leitura
                .read(&mut bloco)
                .map_err(|erro| erro_manifesto(recurso, erro))?;
            if lidos == 0 {
                break;
            }
            hash.update(&bloco[..lidos]);
        }
        let chave = local.parte(partes.len() + 1, "parquet");
        let mut fluxo = tokio::fs::File::from_std(
            arquivo
                .reopen()
                .map_err(|erro| erro_manifesto(recurso, erro))?,
        );
        silver
            .gravar_fluxo(&chave, TIPO_PARQUET, &mut fluxo)
            .await?;
        partes.push(ParteRaw {
            chave,
            url: entrada.url.clone(),
            bytes,
            sha256: hex::encode(hash.finalize()),
            tipo_de_conteudo: TIPO_PARQUET.into(),
            versao: Some(format!(
                "{}|regra={REGRA}",
                entrada.versao.as_deref().unwrap_or("desconhecida")
            )),
        });
    }

    let snapshot_atual = SnapshotBronze::vigente(bronze, "ibge", recurso)
        .await
        .map_err(|erro| erro_manifesto(recurso, erro))?;
    if snapshot_atual
        .as_ref()
        .is_none_or(|atual| atual.chave_manifesto() != snapshot.chave_manifesto())
    {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: "ponteiro Bronze mudou durante a transformação; Silver não publicada".into(),
        });
    }

    let manifesto = ManifestoRaw {
        fonte: "ibge".into(),
        recurso: recurso.into(),
        id_execucao: id_execucao.into(),
        url_inicial: format!("bronze/{recurso}"),
        iniciada_em,
        concluida_em: Utc::now(),
        partes,
    };
    let quantidade = manifesto.partes.len();
    silver
        .gravar(ObjetoRaw {
            chave: local.manifesto(),
            conteudo: serde_json::to_vec_pretty(&manifesto).map_err(|erro| {
                ErroDeIngestao::Manifesto {
                    recurso: recurso.into(),
                    detalhe: erro.to_string(),
                }
            })?,
            tipo_de_conteudo: "application/json".into(),
        })
        .await?;
    publicar_ponteiro(
        silver,
        "ibge",
        recurso,
        &UltimaVersao {
            versao: Some(versao_silver),
            manifesto: local.manifesto(),
            iniciada_em: Some(iniciada_em),
        },
    )
    .await?;
    Ok(ResumoDaTarefa {
        partes: quantidade,
        bytes: total_bytes,
        ..ResumoDaTarefa::default()
    })
}

fn erro_manifesto(recurso: &str, erro: impl std::fmt::Display) -> ErroDeIngestao {
    ErroDeIngestao::Manifesto {
        recurso: recurso.to_owned(),
        detalhe: erro.to_string(),
    }
}
