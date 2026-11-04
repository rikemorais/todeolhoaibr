//! Helper compartilhado: decodificar um ZIP de CSVs (reencodando para UTF-8) em Parquet.
//!
//! Boa parte das fontes de arquivo grava, no raw, um ZIP com um ou mais CSVs — às vezes em
//! Latin-1 (CEAPS do Senado, TSE), às vezes em UTF-8 (CEAP da Câmara). O bronze só traduz:
//! descompacta, reencoda e põe em colunas, mantendo os valores como texto.
//!
//! A escrita é **em lotes** (row groups): as linhas são agrupadas e gravadas aos poucos, de
//! modo que o pico de memória fique no tamanho de um lote, não no arquivo inteiro.

use std::io::{Cursor, Read};

use crate::decodificador::ParteBronze;
use crate::escritor::{ErroDeRefino, EscritorParquet};

/// Quantas linhas acumular antes de gravar um row group.
const LINHAS_POR_LOTE: usize = 50_000;

/// Decodifica, para UTF-8, os bytes de um CSV que pode estar em UTF-8 ou em Windows-1252
/// (Latin-1). Tenta UTF-8 primeiro; se não for válido, assume Windows-1252, que nunca falha.
fn para_utf8(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(texto) => texto.to_owned(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    }
}

/// Lê um CSV (já em UTF-8) com o delimitador dado e grava Parquet em lotes.
fn parquet_de_csv(texto: &str, delimitador: u8) -> Result<Vec<u8>, ErroDeRefino> {
    let mut leitor = csv::ReaderBuilder::new()
        .delimiter(delimitador)
        .flexible(true)
        .from_reader(texto.as_bytes());

    let colunas: Vec<String> = leitor
        .headers()?
        .iter()
        .map(|c| c.trim().to_owned())
        .collect();
    let num_colunas = colunas.len();

    let mut escritor = EscritorParquet::novo(&colunas)?;
    let mut lote: Vec<Vec<Option<String>>> = Vec::with_capacity(LINHAS_POR_LOTE);
    for registro in leitor.records() {
        let registro = registro?;
        // Mantém o número de colunas do cabeçalho: completa linhas curtas com nulos e
        // ignora células excedentes, sem interpretar o conteúdo (fidelidade do bronze).
        let linha: Vec<Option<String>> = (0..num_colunas)
            .map(|i| registro.get(i).map(|v| v.to_owned()))
            .collect();
        lote.push(linha);
        if lote.len() >= LINHAS_POR_LOTE {
            escritor.escrever_lote(&lote)?;
            lote.clear();
        }
    }
    escritor.escrever_lote(&lote)?;
    escritor.finalizar()
}

/// Nome lógico de um CSV interno do ZIP: o basename sem a extensão `.csv`.
fn nome_logico(nome_interno: &str) -> String {
    let base = nome_interno.rsplit('/').next().unwrap_or(nome_interno);
    match base.len() {
        n if n >= 4 && base[n - 4..].eq_ignore_ascii_case(".csv") => base[..n - 4].to_owned(),
        _ => base.to_owned(),
    }
}

/// Descompacta um ZIP em memória e decodifica cada CSV em uma parte Parquet, nomeada pelo
/// arquivo interno (sem a extensão). Entradas que não sejam `.csv` são ignoradas.
pub fn partes_de_zip_csv(bytes: &[u8], delimitador: u8) -> Result<Vec<ParteBronze>, ErroDeRefino> {
    let mut arquivo = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut partes = Vec::new();

    for indice in 0..arquivo.len() {
        let mut entrada = arquivo.by_index(indice)?;
        if !entrada.is_file() {
            continue;
        }
        let nome_interno = entrada.name()?.into_owned();
        if !nome_interno.to_ascii_lowercase().ends_with(".csv") {
            continue;
        }
        let mut conteudo = Vec::new();
        entrada.read_to_end(&mut conteudo)?;
        let texto = para_utf8(&conteudo);
        drop(conteudo);
        let parquet = parquet_de_csv(&texto, delimitador)?;
        partes.push(ParteBronze::nova(nome_logico(&nome_interno), parquet));
    }

    if partes.is_empty() {
        return Err(ErroDeRefino::Decodificacao(
            "nenhum CSV encontrado no ZIP".to_owned(),
        ));
    }
    Ok(partes)
}

#[cfg(test)]
pub(crate) mod testes {
    use super::*;
    use arrow::array::{Array, StringArray};
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn zip_com(nome: &str, conteudo: &[u8]) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut buffer));
            zw.start_file(nome, SimpleFileOptions::default()).unwrap();
            zw.write_all(conteudo).unwrap();
            zw.finish().unwrap();
        }
        buffer
    }

    /// Lê um Parquet de volta em (colunas, linhas) para as asserções.
    pub(crate) fn ler_parquet(bytes: &[u8]) -> (Vec<String>, Vec<Vec<Option<String>>>) {
        let builder =
            ParquetRecordBatchReaderBuilder::try_new(bytes::Bytes::from(bytes.to_vec())).unwrap();
        let colunas: Vec<String> = builder
            .schema()
            .fields()
            .iter()
            .map(|f| f.name().to_owned())
            .collect();
        let mut linhas = Vec::new();
        for lote in builder.build().unwrap() {
            let lote = lote.unwrap();
            for linha in 0..lote.num_rows() {
                let celulas = (0..lote.num_columns())
                    .map(|coluna| {
                        let arr = lote
                            .column(coluna)
                            .as_any()
                            .downcast_ref::<StringArray>()
                            .unwrap();
                        if arr.is_null(linha) {
                            None
                        } else {
                            Some(arr.value(linha).to_owned())
                        }
                    })
                    .collect();
                linhas.push(celulas);
            }
        }
        (colunas, linhas)
    }

    fn c(s: &str) -> Option<String> {
        Some(s.to_owned())
    }

    #[test]
    fn decodifica_csv_utf8_separado_por_ponto_e_virgula() {
        let csv = "id;nome\n1;São Paulo\n2;\n";
        let partes = partes_de_zip_csv(&zip_com("Ano-2024.csv", csv.as_bytes()), b';').unwrap();
        assert_eq!(partes.len(), 1);
        assert_eq!(partes[0].nome, "Ano-2024");
        let (colunas, linhas) = ler_parquet(&partes[0].parquet);
        assert_eq!(colunas, vec!["id", "nome"]);
        assert_eq!(
            linhas,
            vec![vec![c("1"), c("São Paulo")], vec![c("2"), c("")],]
        );
    }

    #[test]
    fn reencoda_latin1_para_utf8() {
        // "João" em Windows-1252: 0x4A 0x6F 0xE3 0x6F.
        let mut csv = b"cidade\n".to_vec();
        csv.extend_from_slice(&[0x4A, 0x6F, 0xE3, 0x6F]);
        csv.push(b'\n');
        let partes = partes_de_zip_csv(&zip_com("x.csv", &csv), b';').unwrap();
        let (_, linhas) = ler_parquet(&partes[0].parquet);
        assert_eq!(linhas, vec![vec![c("João")]]);
    }

    #[test]
    fn ignora_entradas_que_nao_sao_csv() {
        let mut buffer = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut buffer));
            zw.start_file("leia-me.txt", SimpleFileOptions::default())
                .unwrap();
            zw.write_all(b"ignore").unwrap();
            zw.start_file("dados.csv", SimpleFileOptions::default())
                .unwrap();
            zw.write_all(b"a;b\n1;2\n").unwrap();
            zw.finish().unwrap();
        }
        let partes = partes_de_zip_csv(&buffer, b';').unwrap();
        assert_eq!(partes.len(), 1);
        assert_eq!(partes[0].nome, "dados");
    }

    #[test]
    fn grava_em_varios_lotes_sem_perder_linhas() {
        // Mais linhas que um lote, para exercitar múltiplos row groups.
        let mut csv = String::from("n\n");
        let total = LINHAS_POR_LOTE + 123;
        for i in 0..total {
            csv.push_str(&format!("{i}\n"));
        }
        let partes = partes_de_zip_csv(&zip_com("g.csv", csv.as_bytes()), b';').unwrap();
        let (_, linhas) = ler_parquet(&partes[0].parquet);
        assert_eq!(linhas.len(), total);
        assert_eq!(linhas[0], vec![c("0")]);
        assert_eq!(linhas[total - 1], vec![c(&(total - 1).to_string())]);
    }
}
