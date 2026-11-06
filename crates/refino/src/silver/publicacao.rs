//! Fluxo comum de publicação de transformações Silver tipadas.

use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::io::Read;
use todeolho_nucleo::{
    ArmazenamentoRaw, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw,
    ResumoDaTarefa, UltimaVersao, estado_na_raw, publicar_ponteiro,
};

use crate::{EscritorParquetTipado, SnapshotBronze};

const TIPO_PARQUET: &str = "application/vnd.apache.parquet";

/// Identidade e contrato da tabela produzida por uma transformação Silver.
pub(crate) struct ConfiguracaoDePublicacao<'a> {
    pub fonte: &'a str,
    pub recurso: &'a str,
    /// Recurso de saída quando a transformação materializa uma relação derivada.
    pub recurso_destino: Option<&'a str>,
    pub regra: &'a str,
    pub esquema: Option<SchemaRef>,
    pub iniciada_em: DateTime<Utc>,
    pub id_execucao: &'a str,
}

/// Transforma as partes de um snapshot Bronze e publica o Parquet tipado no destino.
///
/// A publicação é idempotente por execução Bronze e versão da regra. Partes e
/// manifesto são gravados antes do ponteiro, e o ponteiro Bronze é revalidado
/// imediatamente antes da publicação para impedir que uma leitura obsoleta vire vigente.
pub(crate) async fn publicar_transformacao<F>(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    configuracao: ConfiguracaoDePublicacao<'_>,
    transformar: F,
) -> Result<ResumoDaTarefa, ErroDeIngestao>
where
    F: FnMut(&RecordBatch) -> Result<RecordBatch, String> + Send,
{
    publicar_transformacao_selecionada(
        bronze,
        silver,
        snapshot,
        configuracao,
        |_| true,
        transformar,
    )
    .await
}

/// Publica apenas as partes escolhidas do manifesto, mantendo o restante do fluxo comum.
pub(crate) async fn publicar_transformacao_selecionada<F, P>(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    configuracao: ConfiguracaoDePublicacao<'_>,
    selecionar: P,
    mut transformar: F,
) -> Result<ResumoDaTarefa, ErroDeIngestao>
where
    F: FnMut(&RecordBatch) -> Result<RecordBatch, String> + Send,
    P: Fn(&ParteRaw) -> bool + Send + Sync,
{
    let ConfiguracaoDePublicacao {
        fonte,
        recurso,
        recurso_destino,
        regra,
        esquema,
        iniciada_em,
        id_execucao,
    } = configuracao;
    let recurso_destino = recurso_destino.unwrap_or(recurso);
    if snapshot.manifesto().fonte != fonte || snapshot.manifesto().recurso != recurso {
        return Err(erro_manifesto(
            recurso,
            format!("snapshot não pertence ao recurso {fonte} solicitado"),
        ));
    }
    let versao_silver = format!("{}|regra={regra}", snapshot.manifesto().id_execucao);
    if let Some(estado) = estado_na_raw(silver, fonte, recurso_destino).await?
        && estado.completa
        && estado.integra
        && estado.versao.as_deref() == Some(versao_silver.as_str())
    {
        return Ok(ResumoDaTarefa {
            reaproveitada: true,
            ..ResumoDaTarefa::default()
        });
    }

    let entradas: Vec<_> = snapshot
        .manifesto()
        .partes
        .iter()
        .filter(|parte| selecionar(parte))
        .collect();
    if entradas.is_empty() {
        return Err(erro_manifesto(
            recurso,
            "nenhuma parte do manifesto corresponde ao contrato solicitado",
        ));
    }

    let local = LocalizacaoRaw::nova(fonte, recurso_destino, iniciada_em, id_execucao);
    let mut partes = Vec::new();
    let mut total_bytes = 0u64;
    for entrada in entradas {
        let baixada = snapshot
            .baixar_parte(bronze, &entrada.chave)
            .await
            .map_err(|erro| erro_manifesto(recurso, erro))?;
        let mut leitor = baixada
            .abrir_lotes(65_536)
            .map_err(|erro| erro_manifesto(recurso, erro))?;
        let arquivo =
            tempfile::NamedTempFile::new().map_err(|erro| erro_manifesto(recurso, erro))?;
        let mut escritor = match &esquema {
            Some(esquema) => Some(
                EscritorParquetTipado::novo_em(
                    arquivo
                        .reopen()
                        .map_err(|erro| erro_manifesto(recurso, erro))?,
                    esquema.clone(),
                )
                .map_err(|erro| erro_manifesto(recurso, erro))?,
            ),
            None => None,
        };
        for lote in &mut leitor {
            let registros = lote
                .map_err(|erro| erro_manifesto(recurso, erro))?
                .registros;
            let transformado =
                transformar(&registros).map_err(|erro| erro_manifesto(recurso, erro))?;
            if escritor.is_none() {
                let esquema_lote = esquema.clone().unwrap_or_else(|| transformado.schema());
                escritor = Some(
                    EscritorParquetTipado::novo_em(
                        arquivo
                            .reopen()
                            .map_err(|erro| erro_manifesto(recurso, erro))?,
                        esquema_lote,
                    )
                    .map_err(|erro| erro_manifesto(recurso, erro))?,
                );
            }
            escritor
                .as_mut()
                .expect("writer initialized from the first batch")
                .escrever_lote(&transformado)
                .map_err(|erro| erro_manifesto(recurso, erro))?;
        }
        let escritor = escritor.ok_or_else(|| {
            erro_manifesto(
                recurso,
                "parte Bronze sem lotes; não foi possível inferir o schema",
            )
        })?;
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
                "{}|regra={regra}",
                entrada.versao.as_deref().unwrap_or("desconhecida")
            )),
        });
    }

    let snapshot_atual = SnapshotBronze::vigente(bronze, fonte, recurso)
        .await
        .map_err(|erro| erro_manifesto(recurso, erro))?;
    if snapshot_atual
        .as_ref()
        .is_none_or(|atual| atual.chave_manifesto() != snapshot.chave_manifesto())
    {
        return Err(erro_manifesto(
            recurso,
            "ponteiro Bronze mudou durante a transformação; Silver não publicada",
        ));
    }

    let manifesto = ManifestoRaw {
        fonte: fonte.into(),
        recurso: recurso_destino.into(),
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
            conteudo: serde_json::to_vec_pretty(&manifesto)
                .map_err(|erro| erro_manifesto(recurso, erro))?,
            tipo_de_conteudo: "application/json".into(),
        })
        .await?;
    publicar_ponteiro(
        silver,
        fonte,
        recurso_destino,
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
