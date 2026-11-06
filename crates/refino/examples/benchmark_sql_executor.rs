//! Smoke/benchmark sintético do caminho Parquet → DataFusion SQL → Parquet.
//!
//! Uso: `cargo run --release -p todeolho-refino --example benchmark_sql_executor -- 1000000 64`

use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use arrow::array::{StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use futures::StreamExt;
use todeolho_refino::{
    EscritorParquetTipado, executor_sql::ConfiguracaoExecutorSql, executor_sql::executar_sql,
};

const TAMANHO_LOTE: usize = 8_192;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argumentos = std::env::args().skip(1).collect::<Vec<_>>();
    let linhas = argumentos
        .first()
        .map(|valor| valor.parse::<u64>())
        .transpose()?
        .unwrap_or(1_000_000);
    let memoria_mib = argumentos
        .get(1)
        .map(|valor| valor.parse::<usize>())
        .transpose()?
        .unwrap_or(64);
    let temporario = tempfile::tempdir()?;
    let bronze = temporario.path().join("bronze");
    let spill = temporario.path().join("spill");
    fs::create_dir(&bronze)?;
    fs::create_dir(&spill)?;
    let arquivo_entrada = bronze.join("parte-00000000.parquet");
    gerar_entrada(&arquivo_entrada, linhas)?;
    let bytes_entrada = fs::metadata(&arquivo_entrada)?.len();

    let configuracao = ConfiguracaoExecutorSql {
        limite_entrada_bytes: bytes_entrada.saturating_mul(2).max(1),
        limite_memoria_bytes: memoria_mib.saturating_mul(1024 * 1024),
        limite_spill_bytes: 2 * 1024 * 1024 * 1024,
        particoes: 1,
        tamanho_lote: TAMANHO_LOTE,
    };
    let consulta = "SELECT id, SUM(CAST(valor AS BIGINT)) AS total FROM bronze WHERE descricao <> '' GROUP BY id";
    let iniciou = Instant::now();
    let mut fluxo = executar_sql(&bronze, &spill, consulta, &configuracao).await?;
    let caminho_saida = temporario.path().join("silver.parquet");
    let mut escritor =
        EscritorParquetTipado::novo_em(File::create(&caminho_saida)?, fluxo.schema())?;
    let mut linhas_saida = 0u64;
    let mut rss_pico = rss_atual_bytes().unwrap_or_default();
    let mut spill_pico = bytes_em_disco(&spill)?;
    while let Some(lote) = fluxo.next().await {
        let lote = lote?;
        linhas_saida += lote.num_rows() as u64;
        escritor.escrever_lote(&lote)?;
        rss_pico = rss_pico.max(rss_atual_bytes().unwrap_or_default());
        spill_pico = spill_pico.max(bytes_em_disco(&spill)?);
    }
    drop(fluxo);
    escritor.fechar()?;

    println!("dataset=sintetico");
    println!("linhas_entrada={linhas}");
    println!("bytes_entrada={bytes_entrada}");
    println!("linhas_saida={linhas_saida}");
    println!("bytes_saida={}", fs::metadata(&caminho_saida)?.len());
    println!("limite_memoria_bytes={}", configuracao.limite_memoria_bytes);
    println!("pico_rss_observado_bytes={rss_pico}");
    println!("pico_spill_observado_bytes={spill_pico}");
    println!("duracao_sql_e_escrita_ms={}", iniciou.elapsed().as_millis());
    Ok(())
}

fn gerar_entrada(caminho: &Path, linhas: u64) -> Result<(), Box<dyn std::error::Error>> {
    let esquema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::UInt64, false),
        Field::new("valor", DataType::Utf8, false),
        Field::new("descricao", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]));
    let mut escritor = EscritorParquetTipado::novo_em(File::create(caminho)?, esquema.clone())?;
    let mut inicio = 0u64;
    while inicio < linhas {
        let quantidade = (linhas - inicio).min(TAMANHO_LOTE as u64) as usize;
        let ids = (inicio..inicio + quantidade as u64).collect::<Vec<_>>();
        let ordinais = ids.iter().map(|id| id + 1).collect::<Vec<_>>();
        let descricoes = ids
            .iter()
            .map(|id| {
                format!(
                    "registro-{id:010}-setor-{:05}-despesa-publica",
                    id % 100_000
                )
            })
            .collect::<Vec<_>>();
        let lote = RecordBatch::try_new(
            esquema.clone(),
            vec![
                Arc::new(UInt64Array::from(ids)) as _,
                Arc::new(StringArray::from_iter_values(
                    (inicio..inicio + quantidade as u64).map(|id| (id % 1_000).to_string()),
                )) as _,
                Arc::new(StringArray::from_iter_values(descricoes.iter())) as _,
                Arc::new(StringArray::from(vec!["manifesto-sintetico"; quantidade])) as _,
                Arc::new(StringArray::from(vec!["parte-sintetica"; quantidade])) as _,
                Arc::new(UInt64Array::from(ordinais)) as _,
            ],
        )?;
        escritor.escrever_lote(&lote)?;
        inicio += quantidade as u64;
    }
    escritor.fechar()?;
    Ok(())
}

fn bytes_em_disco(diretorio: &Path) -> Result<u64, std::io::Error> {
    fs::read_dir(diretorio)?.try_fold(0u64, |total, entrada| {
        let entrada = entrada?;
        let metadata = entrada.metadata()?;
        let bytes = if metadata.is_dir() {
            bytes_em_disco(&entrada.path())?
        } else {
            metadata.len()
        };
        Ok(total.saturating_add(bytes))
    })
}

fn rss_atual_bytes() -> Option<u64> {
    let conteudo = fs::read_to_string("/proc/self/status").ok()?;
    let kib = conteudo
        .lines()
        .find(|linha| linha.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse::<u64>()
        .ok()?;
    Some(kib.saturating_mul(1024))
}
