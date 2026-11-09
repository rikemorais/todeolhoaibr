//! Fluxo comum de publicação de transformações Silver tipadas.

use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use sha2::{Digest, Sha256};
use std::io::Read;
use todeolho_nucleo::{
    ArmazenamentoRaw, ErroDeIngestao, LocalizacaoRaw, ManifestoRaw, ObjetoRaw, ParteRaw,
    ResumoDaTarefa, UltimaVersao, estado_na_raw, publicar_ponteiro,
};

use crate::{
    EscritorParquetTipado, SnapshotBronze,
    executor_sql::{
        COL_MANIFESTO, COL_ORDINAL, COL_PARTE, ConfiguracaoExecutorSql, ErroExecutorSql,
        executar_sql, preparar_snapshot, preparar_snapshot_com_json_original,
        preparar_snapshot_com_json_string_original,
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

/// Executa um contrato SQL sobre o snapshot inteiro e publica sua saída em fluxo.
pub(crate) async fn publicar_transformacao_sql(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    configuracao: ConfiguracaoDePublicacao<'_>,
    sql: &str,
    executor: &ConfiguracaoExecutorSql,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    publicar_transformacao_sql_interno(bronze, silver, snapshot, configuracao, sql, executor, None)
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
        Some(true),
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
        Some(false),
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
    json_original_aninhado: Option<bool>,
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

    let entrada = match json_original_aninhado {
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
    let mut fluxo = executar_sql(entrada.diretorio_bronze(), &diretorio_spill, &sql, executor)
        .await
        .map_err(|erro| {
            erro_manifesto(
                recurso,
                format!("falha ao planejar contrato SQL Silver: {erro}"),
            )
        })?;
    let esquema_saida = fluxo.schema();
    validar_linhagem(&esquema_saida).map_err(|detalhe| erro_manifesto(recurso, detalhe))?;
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
    while let Some(lote) = fluxo.next().await {
        let lote = lote.map_err(|erro| {
            erro_manifesto(
                recurso,
                format!("falha na execução do contrato SQL Silver: {erro}"),
            )
        })?;
        let lote = match &esquema {
            Some(esquema) => normalizar_schema_lote(lote, esquema)
                .map_err(|detalhe| erro_manifesto(recurso, detalhe))?,
            None => lote,
        };
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
