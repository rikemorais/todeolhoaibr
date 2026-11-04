//! Utilitário de validação: conta linhas e colunas de um Parquet. Uso: contar_parquet <arquivo>
use std::fs::File;

use parquet::file::reader::{FileReader, SerializedFileReader};

fn main() {
    let caminho = std::env::args()
        .nth(1)
        .expect("uso: contar_parquet <arquivo.parquet>");
    let arquivo = File::open(&caminho).expect("abrir");
    let leitor = SerializedFileReader::new(arquivo).expect("ler parquet");
    let meta = leitor.metadata().file_metadata();
    println!(
        "linhas={} colunas={}",
        meta.num_rows(),
        meta.schema_descr().num_columns()
    );
}
