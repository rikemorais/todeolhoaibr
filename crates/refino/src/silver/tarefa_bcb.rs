//! Publicação do contrato BCB/SGS na camada Silver.

use std::io::Read;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use todeolho_nucleo::{
    ArmazenamentoRaw, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw,
    UltimaVersao, estado_na_raw, publicar_ponteiro,
};

use super::bcb::{esquema_bcb_sgs, transformar_bcb_sgs};
use crate::{EscritorParquetTipado, SnapshotBronze};

const REGRA: &str = "bcb-sgs-v1";
const TIPO_PARQUET: &str = "application/vnd.apache.parquet";

/// Transforma e publica uma versão Silver BCB a partir do snapshot Bronze recebido.
///
/// O manifesto e o ponteiro Silver só são escritos depois de todas as partes.
pub async fn publicar_bcb_sgs(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<usize, ErroDeIngestao> {
    if snapshot.manifesto().fonte != "bcb" || snapshot.manifesto().recurso != recurso {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: "snapshot não pertence ao recurso BCB solicitado".into(),
        });
    }
    let versao_silver = format!("{}|regra={REGRA}", snapshot.manifesto().id_execucao);
    if let Some(estado) = estado_na_raw(silver, "bcb", recurso).await?
        && estado.completa
        && estado.integra
        && estado.versao.as_deref() == Some(versao_silver.as_str())
    {
        return Ok(0);
    }
    let local = LocalizacaoRaw::nova("bcb", recurso, iniciada_em, id_execucao);
    let mut partes = Vec::new();
    for entrada in &snapshot.manifesto().partes {
        let baixada = snapshot
            .baixar_parte(bronze, &entrada.chave)
            .await
            .map_err(|erro| ErroDeIngestao::Manifesto {
                recurso: recurso.to_owned(),
                detalhe: erro.to_string(),
            })?;
        let mut leitor = baixada
            .abrir_lotes(65_536)
            .map_err(|erro| ErroDeIngestao::Manifesto {
                recurso: recurso.to_owned(),
                detalhe: erro.to_string(),
            })?;
        let arquivo = tempfile::NamedTempFile::new().map_err(|erro| ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: erro.to_string(),
        })?;
        let mut escritor = EscritorParquetTipado::novo_em(
            arquivo.reopen().map_err(|erro| ErroDeIngestao::Manifesto {
                recurso: recurso.to_owned(),
                detalhe: erro.to_string(),
            })?,
            esquema_bcb_sgs(),
        )
        .map_err(|erro| ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: erro.to_string(),
        })?;
        for lote in &mut leitor {
            let transformado = transformar_bcb_sgs(
                &lote
                    .map_err(|erro| ErroDeIngestao::Manifesto {
                        recurso: recurso.to_owned(),
                        detalhe: erro.to_string(),
                    })?
                    .registros,
            )
            .map_err(|erro| ErroDeIngestao::Manifesto {
                recurso: recurso.to_owned(),
                detalhe: erro.to_string(),
            })?;
            escritor
                .escrever_lote(&transformado)
                .map_err(|erro| ErroDeIngestao::Manifesto {
                    recurso: recurso.to_owned(),
                    detalhe: erro.to_string(),
                })?;
        }
        escritor
            .fechar()
            .map_err(|erro| ErroDeIngestao::Manifesto {
                recurso: recurso.to_owned(),
                detalhe: erro.to_string(),
            })?;
        let bytes = arquivo
            .as_file()
            .metadata()
            .map_err(|erro| ErroDeIngestao::Manifesto {
                recurso: recurso.to_owned(),
                detalhe: erro.to_string(),
            })?
            .len();
        let mut hash = Sha256::new();
        let mut leitura = arquivo.reopen().map_err(|erro| ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: erro.to_string(),
        })?;
        let mut bloco = [0u8; 64 * 1024];
        loop {
            let lidos = leitura
                .read(&mut bloco)
                .map_err(|erro| ErroDeIngestao::Manifesto {
                    recurso: recurso.to_owned(),
                    detalhe: erro.to_string(),
                })?;
            if lidos == 0 {
                break;
            }
            hash.update(&bloco[..lidos]);
        }
        let chave = local.parte(partes.len() + 1, "parquet");
        let mut fluxo = tokio::fs::File::from_std(arquivo.reopen().map_err(|erro| {
            ErroDeIngestao::Manifesto {
                recurso: recurso.to_owned(),
                detalhe: erro.to_string(),
            }
        })?);
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
    let snapshot_atual = SnapshotBronze::vigente(bronze, "bcb", recurso)
        .await
        .map_err(|erro| ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: erro.to_string(),
        })?;
    if snapshot_atual
        .as_ref()
        .is_none_or(|atual| atual.chave_manifesto() != snapshot.chave_manifesto())
    {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: "ponteiro Bronze mudou durante a transformação; Silver não publicada".into(),
        });
    }
    let agora = Utc::now();
    let manifesto = ManifestoRaw {
        fonte: "bcb".into(),
        recurso: recurso.into(),
        id_execucao: id_execucao.into(),
        url_inicial: format!("bronze/{recurso}"),
        iniciada_em,
        concluida_em: agora,
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
    let ponteiro = UltimaVersao {
        versao: Some(versao_silver),
        manifesto: local.manifesto(),
        iniciada_em: Some(iniciada_em),
    };
    publicar_ponteiro(silver, "bcb", recurso, &ponteiro).await?;
    Ok(quantidade)
}
