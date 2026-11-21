//! Fluxo comum de publicação de transformações Silver tipadas.

use arrow::array::{Array, BooleanArray, StringArray, UInt64Array};
use arrow::compute::filter_record_batch;
use arrow::datatypes::{DataType, Schema, SchemaRef};
use arrow::record_batch::RecordBatch;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};
use todeolho_nucleo::{
    ArmazenamentoRaw, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw,
    ResumoDaTarefa, UltimaVersao, bloquear_publicacao, estado_na_raw, publicar_ponteiro_sob_guarda,
};
use tokio::io::AsyncWriteExt;

use crate::{
    EscritorParquetTipado, SnapshotBronze,
    executor_sql::{
        COL_MANIFESTO, COL_ORDINAL, COL_PARTE, COL_URL, ConfiguracaoExecutorSql, ErroExecutorSql,
        FiltrosSqlEmendas, executar_sql_com_filtros, preparar_snapshot,
        preparar_snapshot_com_json_original, preparar_snapshot_com_json_string_original,
    },
};

const TIPO_PARQUET: &str = "application/vnd.apache.parquet";
const COL_REJEICAO: &str = "_tdoa_rejeicao_motivo";

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
    limites_rejeicao: Option<&'a BTreeMap<String, u64>>,
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
            limites_rejeicao: None,
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
            limites_rejeicao: None,
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
            limites_rejeicao: None,
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
            limites_rejeicao: None,
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
            limites_rejeicao: None,
        },
    )
    .await
}

/// Publica um contrato que marca linhas rejeitadas em `_tdoa_rejeicao_motivo`.
pub(crate) async fn publicar_transformacao_sql_com_rejeicoes(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    configuracao: ConfiguracaoDePublicacao<'_>,
    sql: &str,
    executor: &ConfiguracaoExecutorSql,
    limites: &BTreeMap<String, u64>,
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
            limites_rejeicao: Some(limites),
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
        && (opcoes.limites_rejeicao.is_none()
            || quarentena_disponivel(silver, fonte, recurso_destino).await?)
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
    let esquema_saida = esquema_publicado(&esquema_saida, opcoes.limites_rejeicao.is_some())
        .map_err(|detalhe| erro_manifesto(recurso, detalhe))?;
    if esquema
        .as_ref()
        .is_some_and(|esperado| !schemas_compativeis(esperado, &esquema_saida))
    {
        let esquema_esperado = esquema.as_ref().expect("schema checked above");
        return Err(erro_manifesto(
            recurso,
            format!(
                "schema SQL {esquema_saida:?} não corresponde ao schema declarado {esquema_esperado:?}"
            ),
        ));
    }

    let local = LocalizacaoRaw::nova(fonte, recurso_destino, iniciada_em, id_execucao);
    let arquivo = tempfile::Builder::new()
        .prefix("todeolho-silver-")
        .suffix(".parquet")
        .tempfile_in(entrada.diretorio_execucao())
        .map_err(|_| erro_manifesto(recurso, "falha ao criar Parquet Silver temporário"))?;
    let esquema_parquet = esquema.clone().unwrap_or(esquema_saida);
    let mut escritor = EscritorParquetTipado::novo_em(
        arquivo
            .reopen()
            .map_err(|_| erro_manifesto(recurso, "falha ao abrir Parquet Silver temporário"))?,
        esquema_parquet,
    )
    .map_err(|_| erro_manifesto(recurso, "falha ao iniciar escritor Parquet Silver"))?;
    let mut arquivo_diagnosticos = opcoes
        .limites_rejeicao
        .map(|_| {
            tempfile::Builder::new()
                .prefix("todeolho-rejeicoes-")
                .suffix(".jsonl")
                .tempfile_in(entrada.diretorio_execucao())
        })
        .transpose()
        .map_err(|_| erro_manifesto(recurso, "falha ao criar arquivo de diagnósticos Silver"))?;
    let mut rejeicoes_por_motivo = BTreeMap::new();
    let mut linhas_saida = 0u64;
    while let Some(lote) = fluxo.next().await {
        let lote = lote.map_err(|erro| {
            erro_manifesto(
                recurso,
                format!("falha na execução do contrato SQL Silver: {erro}"),
            )
        })?;
        let lote = if let Some(limites) = opcoes.limites_rejeicao {
            separar_rejeicoes(
                lote,
                regra,
                limites,
                &mut rejeicoes_por_motivo,
                arquivo_diagnosticos
                    .as_mut()
                    .expect("arquivo criado")
                    .as_file_mut(),
            )
            .map_err(|detalhe| erro_manifesto(recurso, detalhe))?
        } else {
            lote
        };
        let lote = match &esquema {
            Some(esquema) => normalizar_schema_lote(lote, esquema)
                .map_err(|detalhe| erro_manifesto(recurso, detalhe))?,
            None => lote,
        };
        linhas_saida = linhas_saida
            .checked_add(lote.num_rows() as u64)
            .ok_or_else(|| erro_manifesto(recurso, "contagem de linhas Silver excede u64"))?;
        escritor
            .escrever_lote(&lote)
            .map_err(|_| erro_manifesto(recurso, "resultado SQL diverge do schema de saída"))?;
    }
    drop(fluxo);
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

    let linhas_rejeitadas = rejeicoes_por_motivo
        .values()
        .try_fold(0u64, |total, quantidade| total.checked_add(*quantidade))
        .ok_or_else(|| erro_manifesto(recurso, "contagem total de rejeições excede u64"))?;
    let mut quarentena = None;
    if linhas_rejeitadas > 0 {
        let diagnosticos = arquivo_diagnosticos.as_mut().expect("arquivo criado");
        diagnosticos
            .as_file_mut()
            .flush()
            .map_err(|_| erro_manifesto(recurso, "falha ao finalizar diagnósticos Silver"))?;
        let bytes = diagnosticos
            .as_file()
            .metadata()
            .map_err(|_| erro_manifesto(recurso, "falha ao medir diagnósticos Silver"))?
            .len();
        let mut hash = Sha256::new();
        let mut leitura = diagnosticos
            .reopen()
            .map_err(|_| erro_manifesto(recurso, "falha ao ler diagnósticos Silver"))?;
        loop {
            let lidos = leitura.read(&mut bloco).map_err(|_| {
                erro_manifesto(recurso, "falha ao calcular hash dos diagnósticos Silver")
            })?;
            if lidos == 0 {
                break;
            }
            hash.update(&bloco[..lidos]);
        }
        let chave_quarentena =
            format!("_quarentena/{fonte}/{recurso_destino}/execucao={id_execucao}/rejeicoes.jsonl");
        let mut fluxo = tokio::fs::File::from_std(
            diagnosticos
                .reopen()
                .map_err(|_| erro_manifesto(recurso, "falha ao reabrir diagnósticos Silver"))?,
        );
        silver
            .gravar_fluxo(&chave_quarentena, "application/x-ndjson", &mut fluxo)
            .await?;
        quarentena = Some(serde_json::json!({
            "chave": chave_quarentena,
            "bytes": bytes,
            "sha256": hex::encode(hash.finalize()),
        }));
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
    manifesto_json["qualidade"] = serde_json::json!({
        "versao": 1,
        "manifesto_bronze": snapshot.chave_manifesto(),
        "regra": regra,
        "linhas_bronze_preparadas": entrada.linhas_bronze(),
        "linhas_saida": linhas_saida,
        "linhas_rejeitadas": linhas_rejeitadas,
        "rejeicoes_por_motivo": rejeicoes_por_motivo,
        "politica_rejeicao": if opcoes.limites_rejeicao.is_some() { "limites_absolutos" } else { "falha_total" },
        "quarentena": quarentena,
    });
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

/// Confere a presença e o tamanho da quarentena apontada pelo manifesto vigente.
pub(crate) async fn quarentena_disponivel(
    silver: &dyn ArmazenamentoRaw,
    fonte: &str,
    recurso: &str,
) -> Result<bool, ErroDeIngestao> {
    let Some(ponteiro) = silver
        .ler_controle(&LocalizacaoRaw::ultima_versao(fonte, recurso))
        .await?
    else {
        return Ok(false);
    };
    let Ok(ponteiro) = serde_json::from_slice::<UltimaVersao>(&ponteiro) else {
        return Ok(false);
    };
    let Some(manifesto) = silver.ler_controle(&ponteiro.manifesto).await? else {
        return Ok(false);
    };
    let Ok(manifesto) = serde_json::from_slice::<serde_json::Value>(&manifesto) else {
        return Ok(false);
    };
    if manifesto["qualidade"]["politica_rejeicao"] != "limites_absolutos" {
        return Ok(false);
    }
    let quarentena = &manifesto["qualidade"]["quarentena"];
    if quarentena.is_null() {
        return Ok(manifesto["qualidade"]["linhas_rejeitadas"] == 0);
    }
    let (Some(chave), Some(bytes), Some(sha256)) = (
        quarentena["chave"].as_str(),
        quarentena["bytes"].as_u64(),
        quarentena["sha256"].as_str(),
    ) else {
        return Ok(false);
    };
    if silver.tamanho(chave).await? != Some(bytes) {
        return Ok(false);
    }
    let mut arquivo = tempfile::tempfile().map_err(|erro| erro_manifesto(recurso, erro))?;
    let mut escrita = tokio::fs::File::from_std(
        arquivo
            .try_clone()
            .map_err(|erro| erro_manifesto(recurso, erro))?,
    );
    if !silver.ler_fluxo(chave, &mut escrita).await? {
        return Ok(false);
    }
    escrita
        .flush()
        .await
        .map_err(|erro| erro_manifesto(recurso, erro))?;
    drop(escrita);
    arquivo
        .rewind()
        .map_err(|erro| erro_manifesto(recurso, erro))?;
    let mut hash = Sha256::new();
    let mut total = 0u64;
    let mut bloco = [0u8; 64 * 1024];
    loop {
        let lidos = arquivo
            .read(&mut bloco)
            .map_err(|erro| erro_manifesto(recurso, erro))?;
        if lidos == 0 {
            break;
        }
        total = total
            .checked_add(lidos as u64)
            .ok_or_else(|| erro_manifesto(recurso, "tamanho da quarentena excede u64"))?;
        hash.update(&bloco[..lidos]);
    }
    Ok(total == bytes && hex::encode(hash.finalize()) == sha256)
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

/// Colunas internas de diagnóstico não fazem parte do Parquet Silver publicado.
fn esquema_publicado(recebido: &SchemaRef, com_rejeicao: bool) -> Result<SchemaRef, &'static str> {
    if !com_rejeicao {
        return Ok(recebido.clone());
    }
    let motivo = recebido
        .field_with_name(COL_REJEICAO)
        .map_err(|_| "contrato com rejeição deve projetar _tdoa_rejeicao_motivo")?;
    if motivo.data_type() != &DataType::Utf8 {
        return Err("_tdoa_rejeicao_motivo deve ser Utf8");
    }
    let url = recebido
        .field_with_name(COL_URL)
        .map_err(|_| "contrato com rejeição deve projetar _tdoa_origem_url")?;
    if url.data_type() != &DataType::Utf8 {
        return Err("_tdoa_origem_url deve ser Utf8");
    }
    let campos = recebido
        .fields()
        .iter()
        .filter(|campo| campo.name() != COL_REJEICAO && campo.name() != COL_URL)
        .cloned()
        .collect::<Vec<_>>();
    Ok(std::sync::Arc::new(Schema::new(campos)))
}

fn separar_rejeicoes(
    lote: RecordBatch,
    regra: &str,
    limites: &BTreeMap<String, u64>,
    contagens: &mut BTreeMap<String, u64>,
    diagnosticos: &mut std::fs::File,
) -> Result<RecordBatch, String> {
    let esquema = lote.schema();
    let indice_motivo = esquema.index_of(COL_REJEICAO).map_err(|e| e.to_string())?;
    let motivos = lote
        .column(indice_motivo)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or("motivo de rejeição não é Utf8")?;
    let texto = |nome: &str| -> Result<&StringArray, String> {
        let indice = esquema.index_of(nome).map_err(|e| e.to_string())?;
        lote.column(indice)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| format!("{nome} não é Utf8"))
    };
    let manifestos = texto(COL_MANIFESTO)?;
    let partes = texto(COL_PARTE)?;
    let urls = texto(COL_URL)?;
    let indice_ordinal = esquema.index_of(COL_ORDINAL).map_err(|e| e.to_string())?;
    let ordinais = lote
        .column(indice_ordinal)
        .as_any()
        .downcast_ref::<UInt64Array>()
        .ok_or("ordinal não é UInt64")?;
    let mut validas = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        if motivos.is_null(linha) {
            validas.push(true);
            continue;
        }
        let motivo = motivos.value(linha);
        let limite = limites
            .get(motivo)
            .ok_or_else(|| format!("motivo de rejeição não declarado: {motivo}"))?;
        let contagem = contagens.entry(motivo.to_owned()).or_default();
        *contagem = contagem
            .checked_add(1)
            .ok_or("contagem de rejeições excede u64")?;
        if *contagem > *limite {
            return Err(format!("rejeições por {motivo} excedem o limite {limite}"));
        }
        if manifestos.is_null(linha)
            || partes.is_null(linha)
            || urls.is_null(linha)
            || ordinais.is_null(linha)
        {
            return Err("linha rejeitada não tem referência Bronze completa".into());
        }
        serde_json::to_writer(
            &mut *diagnosticos,
            &serde_json::json!({
                "motivo": motivo,
                "regra": regra,
                "manifesto_bronze": manifestos.value(linha),
                "parte_bronze": partes.value(linha),
                "tabela_bronze": urls.value(linha),
                "ordinal_bronze": ordinais.value(linha),
            }),
        )
        .map_err(|e| e.to_string())?;
        diagnosticos.write_all(b"\n").map_err(|e| e.to_string())?;
        validas.push(false);
    }
    let filtrado =
        filter_record_batch(&lote, &BooleanArray::from(validas)).map_err(|e| e.to_string())?;
    let indices = esquema
        .fields()
        .iter()
        .enumerate()
        .filter(|(_, campo)| campo.name() != COL_REJEICAO && campo.name() != COL_URL)
        .map(|(indice, _)| indice)
        .collect::<Vec<_>>();
    filtrado.project(&indices).map_err(|e| e.to_string())
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
