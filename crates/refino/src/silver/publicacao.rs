//! Fluxo comum de publicação de transformações Silver tipadas.

use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;

use arrow::array::{Array, AsArray};
use arrow::compute::{filter_record_batch, is_not_null, is_null};
use arrow::datatypes::{SchemaRef, UInt64Type};
use arrow::record_batch::RecordBatch;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use todeolho_nucleo::{
    ArmazenamentoRaw, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw,
    ResumoDaTarefa, UltimaVersao, bloquear_publicacao, estado_na_raw, publicar_ponteiro_sob_guarda,
};

use super::diagnostico::{
    AcumuladorDeRejeicoes, COL_REJEICAO_MOTIVO, chave_quarentena, verificar_limites,
};
use crate::{
    EscritorParquetTipado, SnapshotBronze,
    executor_sql::{
        COL_MANIFESTO, COL_ORDINAL, COL_PARTE, ConfiguracaoExecutorSql, ErroExecutorSql,
        FiltrosSqlEmendas, executar_sql_com_filtros, preparar_snapshot,
        preparar_snapshot_com_json_original, preparar_snapshot_com_json_string_original,
    },
};

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

struct OpcoesDeTransformacao<'a> {
    json_original_aninhado: Option<bool>,
    fonte_bronze: Option<&'a str>,
    filtros_emendas: Option<&'a FiltrosSqlEmendas>,
    limites_rejeicao: HashMap<String, u64>,
}

/// Executa um contrato SQL sobre o snapshot inteiro e publica sua saída em fluxo.
pub(crate) async fn publicar_transformacao_sql(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    configuracao: ConfiguracaoDePublicacao<'_>,
    sql: &str,
    executor: &ConfiguracaoExecutorSql,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao_sql_interno(
        bronze,
        silver,
        snapshot,
        configuracao,
        sql,
        executor,
        OpcoesDeTransformacao {
            json_original_aninhado: None,
            fonte_bronze: None,
            filtros_emendas: None,
            limites_rejeicao: HashMap::new(),
        },
    )
    .await
}

/// Executa SQL quando a fonte Bronze tem nome diferente da fonte Silver publicada.
pub(crate) async fn publicar_transformacao_sql_com_fonte_bronze(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    fonte_bronze: &str,
    configuracao: ConfiguracaoDePublicacao<'_>,
    sql: &str,
    executor: &ConfiguracaoExecutorSql,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao_sql_interno(
        bronze,
        silver,
        snapshot,
        configuracao,
        sql,
        executor,
        OpcoesDeTransformacao {
            json_original_aninhado: None,
            fonte_bronze: Some(fonte_bronze),
            filtros_emendas: None,
            limites_rejeicao: HashMap::new(),
        },
    )
    .await
}

/// Executa um contrato SQL preservando também os campos dinâmicos da linha Bronze em JSON.
pub(crate) async fn publicar_transformacao_sql_com_json_original(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    configuracao: ConfiguracaoDePublicacao<'_>,
    sql: &str,
    executor: &ConfiguracaoExecutorSql,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao_sql_interno(
        bronze,
        silver,
        snapshot,
        configuracao,
        sql,
        executor,
        OpcoesDeTransformacao {
            json_original_aninhado: Some(true),
            fonte_bronze: None,
            filtros_emendas: None,
            limites_rejeicao: HashMap::new(),
        },
    )
    .await
}

/// Executa SQL e preserva todos os valores da linha original como texto JSON.
pub(crate) async fn publicar_transformacao_sql_com_json_string_original(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    configuracao: ConfiguracaoDePublicacao<'_>,
    sql: &str,
    executor: &ConfiguracaoExecutorSql,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao_sql_interno(
        bronze,
        silver,
        snapshot,
        configuracao,
        sql,
        executor,
        OpcoesDeTransformacao {
            json_original_aninhado: Some(false),
            fonte_bronze: None,
            filtros_emendas: None,
            limites_rejeicao: HashMap::new(),
        },
    )
    .await
}

/// Executa SQL com limites de rejeição por motivo declarados pelo contrato.
pub(crate) async fn publicar_transformacao_sql_com_limites(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    configuracao: ConfiguracaoDePublicacao<'_>,
    sql: &str,
    executor: &ConfiguracaoExecutorSql,
    limites_rejeicao: HashMap<String, u64>,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao_sql_interno(
        bronze,
        silver,
        snapshot,
        configuracao,
        sql,
        executor,
        OpcoesDeTransformacao {
            json_original_aninhado: None,
            fonte_bronze: None,
            filtros_emendas: None,
            limites_rejeicao,
        },
    )
    .await
}

/// Executa um contrato SQL de emendas com filtros de ano/UF tipados.
pub(crate) async fn publicar_transformacao_sql_com_filtros_emendas(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    configuracao: ConfiguracaoDePublicacao<'_>,
    sql: &str,
    executor: &ConfiguracaoExecutorSql,
    filtros: &FiltrosSqlEmendas,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao_sql_interno(
        bronze,
        silver,
        snapshot,
        configuracao,
        sql,
        executor,
        OpcoesDeTransformacao {
            json_original_aninhado: None,
            fonte_bronze: None,
            filtros_emendas: Some(filtros),
            limites_rejeicao: HashMap::new(),
        },
    )
    .await
}

async fn publicar_transformacao_sql_interno(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    configuracao: ConfiguracaoDePublicacao<'_>,
    sql: &str,
    executor: &ConfiguracaoExecutorSql,
    opcoes: OpcoesDeTransformacao<'_>,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
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
    if snapshot.manifesto().fonte != opcoes.fonte_bronze.unwrap_or(fonte)
        || snapshot.manifesto().recurso != recurso
    {
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

    let entrada = match opcoes.json_original_aninhado {
        Some(true) => {
            preparar_snapshot_com_json_original(bronze, snapshot, executor.limite_entrada_bytes)
                .await
        }
        Some(false) => {
            preparar_snapshot_com_json_string_original(
                bronze,
                snapshot,
                executor.limite_entrada_bytes,
            )
            .await
        }
        None => preparar_snapshot(bronze, snapshot, executor.limite_entrada_bytes).await,
    }
    .map_err(|erro| {
        let detalhe = match erro {
            ErroExecutorSql::Snapshot(_) => {
                "falha ao validar ou baixar parte do snapshot Bronze para SQL"
            }
            ErroExecutorSql::Configuracao(_) => "snapshot Bronze excede os limites de entrada SQL",
            ErroExecutorSql::Parquet(_) | ErroExecutorSql::Io(_) => {
                "falha ao preparar arquivos temporários do snapshot Bronze"
            }
            ErroExecutorSql::DataFusion(_) => "falha ao preparar a consulta DataFusion",
        };
        erro_manifesto(recurso, detalhe)
    })?;
    let diretorio_spill = entrada.diretorio_execucao().join("spill");
    std::fs::create_dir(&diretorio_spill).map_err(|_| {
        erro_manifesto(
            recurso,
            "falha ao preparar diretório temporário do DataFusion",
        )
    })?;
    let sql = entrada
        .expandir_placeholders_sql(sql)
        .map_err(|erro| erro_manifesto(recurso, erro))?;
    let mut fluxo = executar_sql_com_filtros(
        entrada.diretorio_bronze(),
        &diretorio_spill,
        &sql,
        executor,
        opcoes.filtros_emendas,
    )
    .await
    .map_err(|erro| {
        erro_manifesto(
            recurso,
            format!("falha ao planejar contrato SQL Silver: {erro}"),
        )
    })?;
    let esquema_saida = fluxo.schema();
    validar_linhagem(&esquema_saida).map_err(|detalhe| erro_manifesto(recurso, detalhe))?;

    let indice_rejeicao = esquema_saida
        .fields()
        .iter()
        .position(|f| f.name() == COL_REJEICAO_MOTIVO);
    let esquema_silver = match indice_rejeicao {
        Some(_) => {
            let campos: Vec<_> = esquema_saida
                .fields()
                .iter()
                .filter(|f| f.name() != COL_REJEICAO_MOTIVO)
                .cloned()
                .collect();
            Arc::new(arrow::datatypes::Schema::new(campos))
        }
        None => esquema_saida.clone(),
    };

    if esquema
        .as_ref()
        .is_some_and(|esperado| !schemas_compativeis(esperado, &esquema_silver))
    {
        let esquema_esperado = esquema.as_ref().expect("schema checked above");
        return Err(erro_manifesto(
            recurso,
            format!(
                "schema SQL {esquema_silver:?} não corresponde ao schema declarado {esquema_esperado:?}"
            ),
        ));
    }

    let indices_silver: Vec<usize> = (0..esquema_saida.fields().len())
        .filter(|i| Some(*i) != indice_rejeicao)
        .collect();
    let idx_manifesto = esquema_saida
        .index_of(COL_MANIFESTO)
        .map_err(|_| erro_manifesto(recurso, "coluna de linhagem ausente"))?;
    let idx_parte = esquema_saida
        .index_of(COL_PARTE)
        .map_err(|_| erro_manifesto(recurso, "coluna de linhagem ausente"))?;
    let idx_ordinal = esquema_saida
        .index_of(COL_ORDINAL)
        .map_err(|_| erro_manifesto(recurso, "coluna de linhagem ausente"))?;

    let local = LocalizacaoRaw::nova(fonte, recurso_destino, iniciada_em, id_execucao);
    let arquivo = tempfile::Builder::new()
        .prefix("todeolho-silver-")
        .suffix(".parquet")
        .tempfile_in(entrada.diretorio_execucao())
        .map_err(|_| erro_manifesto(recurso, "falha ao criar Parquet Silver temporário"))?;
    let esquema_parquet = esquema.clone().unwrap_or(esquema_silver);
    let mut escritor = EscritorParquetTipado::novo_em(
        arquivo
            .reopen()
            .map_err(|_| erro_manifesto(recurso, "falha ao abrir Parquet Silver temporário"))?,
        esquema_parquet,
    )
    .map_err(|_| erro_manifesto(recurso, "falha ao iniciar escritor Parquet Silver"))?;

    let mut linhas_saida = 0u64;
    let mut acumulador = indice_rejeicao.map(|_| AcumuladorDeRejeicoes::novo(regra));

    while let Some(lote) = fluxo.next().await {
        let lote = lote.map_err(|erro| {
            erro_manifesto(
                recurso,
                format!("falha na execução do contrato SQL Silver: {erro}"),
            )
        })?;

        let lote_silver = if let Some((idx_rej, ref mut acum)) =
            indice_rejeicao.zip(acumulador.as_mut())
        {
            let col_motivo = lote.column(idx_rej);
            let mascara_valida = is_null(col_motivo)
                .map_err(|e| erro_manifesto(recurso, format!("filtro de rejeição: {e}")))?;
            let mascara_rejeitada = is_not_null(col_motivo)
                .map_err(|e| erro_manifesto(recurso, format!("filtro de rejeição: {e}")))?;

            let rejeitado = filter_record_batch(&lote, &mascara_rejeitada)
                .map_err(|e| erro_manifesto(recurso, format!("filtro de rejeição: {e}")))?;
            if rejeitado.num_rows() > 0 {
                let motivos = rejeitado.column(idx_rej).as_string::<i32>();
                let manifestos = rejeitado.column(idx_manifesto).as_string::<i32>();
                let partes = rejeitado.column(idx_parte).as_string::<i32>();
                let ordinais = rejeitado
                    .column(idx_ordinal)
                    .as_primitive::<UInt64Type>();
                acum.acumular(motivos, manifestos, partes, ordinais);
            }

            let valido = filter_record_batch(&lote, &mascara_valida)
                .map_err(|e| erro_manifesto(recurso, format!("filtro de rejeição: {e}")))?;
            if valido.num_rows() == 0 {
                continue;
            }
            valido
                .project(&indices_silver)
                .map_err(|e| erro_manifesto(recurso, format!("projeção Silver: {e}")))?
        } else {
            lote
        };

        let lote_silver = match &esquema {
            Some(esquema) => normalizar_schema_lote(lote_silver, esquema)
                .map_err(|detalhe| erro_manifesto(recurso, detalhe))?,
            None => lote_silver,
        };
        linhas_saida = linhas_saida
            .checked_add(lote_silver.num_rows() as u64)
            .ok_or_else(|| erro_manifesto(recurso, "contagem de linhas Silver excede u64"))?;
        escritor
            .escrever_lote(&lote_silver)
            .map_err(|_| erro_manifesto(recurso, "resultado SQL diverge do schema de saída"))?;
    }
    drop(fluxo);

    if let Some(ref acum) = acumulador {
        if !acum.esta_vazio() {
            verificar_limites(acum.contagens(), &opcoes.limites_rejeicao)
                .map_err(|msg| erro_manifesto(recurso, msg))?;
        }
    }

    escritor
        .fechar()
        .map_err(|_| erro_manifesto(recurso, "falha ao finalizar Parquet Silver"))?;

    let bytes = arquivo
        .as_file()
        .metadata()
        .map_err(|_| erro_manifesto(recurso, "falha ao medir Parquet Silver"))?
        .len();
    let mut hash = Sha256::new();
    let mut leitura = arquivo
        .reopen()
        .map_err(|_| erro_manifesto(recurso, "falha ao ler Parquet Silver temporário"))?;
    let mut bloco = [0u8; 64 * 1024];
    loop {
        let lidos = leitura
            .read(&mut bloco)
            .map_err(|_| erro_manifesto(recurso, "falha ao calcular hash do Parquet Silver"))?;
        if lidos == 0 {
            break;
        }
        hash.update(&bloco[..lidos]);
    }
    let chave = local.parte(1, "parquet");
    let mut fluxo_upload = tokio::fs::File::from_std(
        arquivo
            .reopen()
            .map_err(|_| erro_manifesto(recurso, "falha ao reabrir Parquet Silver"))?,
    );
    silver
        .gravar_fluxo(&chave, TIPO_PARQUET, &mut fluxo_upload)
        .await?;

    if let Some(ref acum) = acumulador {
        if !acum.esta_vazio() {
            let bytes_quarentena = acum
                .para_parquet()
                .map_err(|_| erro_manifesto(recurso, "falha ao gerar Parquet de quarentena"))?;
            let chave_q = chave_quarentena(fonte, recurso_destino, id_execucao);
            silver
                .gravar(ObjetoRaw {
                    chave: chave_q,
                    conteudo: bytes_quarentena,
                    tipo_de_conteudo: TIPO_PARQUET.into(),
                })
                .await?;
        }
    }

    let guarda = bloquear_publicacao(fonte, recurso).await;
    if !snapshot
        .ainda_vigente(bronze, opcoes.fonte_bronze.unwrap_or(fonte), recurso)
        .await
        .map_err(|erro| erro_manifesto(recurso, erro))?
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
        partes: vec![ParteRaw {
            chave,
            url: recurso.into(),
            bytes,
            sha256: hex::encode(hash.finalize()),
            tipo_de_conteudo: TIPO_PARQUET.into(),
            versao: Some(format!(
                "{}|regra={regra}",
                snapshot.manifesto().id_execucao
            )),
        }],
    };
    let mut manifesto_json =
        serde_json::to_value(&manifesto).map_err(|erro| erro_manifesto(recurso, erro))?;
    manifesto_json["qualidade"] = match &acumulador {
        Some(acum) => {
            let politica = if opcoes.limites_rejeicao.is_empty() {
                "falha_total"
            } else {
                "por_limite"
            };
            let mut q = serde_json::json!({
                "versao": 2,
                "manifesto_bronze": snapshot.chave_manifesto(),
                "regra": regra,
                "linhas_bronze_preparadas": entrada.linhas_bronze(),
                "linhas_saida": linhas_saida,
                "linhas_rejeitadas": acum.total(),
                "rejeicoes_por_motivo": acum.contagens(),
                "politica_rejeicao": politica,
            });
            if !opcoes.limites_rejeicao.is_empty() {
                q["limites_rejeicao"] = serde_json::to_value(&opcoes.limites_rejeicao)
                    .expect("HashMap<String, u64> é serializável");
            }
            if !acum.esta_vazio() {
                q["quarentena"] =
                    serde_json::json!(chave_quarentena(fonte, recurso_destino, id_execucao));
            }
            q
        }
        None => serde_json::json!({
            "versao": 1,
            "manifesto_bronze": snapshot.chave_manifesto(),
            "regra": regra,
            "linhas_bronze_preparadas": entrada.linhas_bronze(),
            "linhas_saida": linhas_saida,
            "linhas_rejeitadas": 0,
            "politica_rejeicao": "falha_total"
        }),
    };
    silver
        .gravar(ObjetoRaw {
            chave: local.manifesto(),
            conteudo: serde_json::to_vec_pretty(&manifesto_json)
                .map_err(|erro| erro_manifesto(recurso, erro))?,
            tipo_de_conteudo: "application/json".into(),
        })
        .await?;
    let publicou = publicar_ponteiro_sob_guarda(
        silver,
        fonte,
        recurso_destino,
        &UltimaVersao {
            versao: Some(versao_silver),
            manifesto: local.manifesto(),
            iniciada_em: Some(iniciada_em),
        },
        &guarda,
    )
    .await?;
    if !publicou {
        return Err(erro_manifesto(
            recurso,
            "ponteiro Silver mais recente preservado; esta execução não foi publicada",
        ));
    }
    Ok(ResumoDaTarefa {
        partes: 1,
        bytes,
        ..ResumoDaTarefa::default()
    })
}

/// Compara o contrato SQL sem exigir que o planejador prove nulabilidade a partir do Parquet.
fn schemas_compativeis(esperado: &SchemaRef, recebido: &SchemaRef) -> bool {
    esperado.metadata() == recebido.metadata()
        && esperado.fields().len() == recebido.fields().len()
        && esperado
            .fields()
            .iter()
            .zip(recebido.fields())
            .all(|(esperado, recebido)| {
                esperado.name() == recebido.name()
                    && esperado.data_type() == recebido.data_type()
                    && esperado.metadata() == recebido.metadata()
            })
}

/// Restaura a nulabilidade declarada e impede que SQL publique nulos proibidos pelo contrato.
fn normalizar_schema_lote(
    lote: RecordBatch,
    esperado: &SchemaRef,
) -> Result<RecordBatch, &'static str> {
    if esperado
        .fields()
        .iter()
        .zip(lote.columns())
        .any(|(campo, coluna)| !campo.is_nullable() && coluna.null_count() > 0)
    {
        return Err("resultado SQL contém nulos em campos obrigatórios do contrato Silver");
    }
    RecordBatch::try_new(esperado.clone(), lote.columns().to_vec())
        .map_err(|_| "resultado SQL não pode usar a nulabilidade declarada pelo contrato")
}

fn validar_linhagem(esquema: &SchemaRef) -> Result<(), String> {
    for (nome, tipo) in [
        (COL_MANIFESTO, arrow::datatypes::DataType::Utf8),
        (COL_PARTE, arrow::datatypes::DataType::Utf8),
        (COL_ORDINAL, arrow::datatypes::DataType::UInt64),
    ] {
        match esquema.field_with_name(nome) {
            Ok(campo) if campo.data_type() == &tipo => {}
            _ => {
                return Err(format!(
                    "contrato SQL Silver deve projetar {nome} ({tipo:?})"
                ));
            }
        }
    }
    Ok(())
}

fn erro_manifesto(recurso: &str, erro: impl std::fmt::Display) -> ErroDeIngestao {
    ErroDeIngestao::Manifesto {
        recurso: recurso.to_owned(),
        detalhe: erro.to_string(),
    }
}
