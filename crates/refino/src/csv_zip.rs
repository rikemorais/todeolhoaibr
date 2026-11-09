//! Helper compartilhado: decodificar um ZIP de CSVs (reencodando para UTF-8) em Parquet.
//!
//! Boa parte das fontes de arquivo grava, no raw, um ZIP com um ou mais CSVs — às vezes em
//! Latin-1 (CEAPS do Senado, TSE), às vezes em UTF-8 (CEAP da Câmara). O bronze só traduz:
//! descompacta, reencoda e põe em colunas, mantendo os valores como texto.
//!
//! A escrita é **em lotes** (row groups): as linhas são agrupadas e gravadas aos poucos, de
//! modo que o pico de memória fique no tamanho de um lote, não no arquivo inteiro.

use std::io::{BufRead, BufReader, Cursor, Read, Seek};

use crate::decodificador::ParteBronze;
use crate::escritor::{ErroDeRefino, EscritorParquet};

/// Quantas linhas acumular antes de gravar um row group.
const LINHAS_POR_LOTE: usize = 50_000;

/// Leitor que auto-detecta encoding: UTF-8 passa direto, senão transcodifica de Latin-1.
enum LeitorAutoEncoding<R> {
    Utf8(BufReader<R>),
    Latin1(encoding_rs_io::DecodeReaderBytes<BufReader<R>, Vec<u8>>),
}

impl<R: Read> Read for LeitorAutoEncoding<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Utf8(r) => r.read(buf),
            Self::Latin1(r) => r.read(buf),
        }
    }
}

/// Envolve um `Read` num leitor que auto-detecta o encoding a partir dos primeiros bytes
/// buffered. Se o conteúdo disponível no buffer for UTF-8 válido, os bytes passam direto;
/// senão, são transcodificados de Windows-1252 — o mesmo critério de [`para_utf8`], mas
/// em fluxo.
fn leitor_auto_encoding<R: Read>(fonte: R) -> LeitorAutoEncoding<R> {
    let mut buf = BufReader::with_capacity(16_384, fonte);
    let e_utf8 = buf
        .fill_buf()
        .map_or(true, |a| std::str::from_utf8(a).is_ok());
    if e_utf8 {
        LeitorAutoEncoding::Utf8(buf)
    } else {
        LeitorAutoEncoding::Latin1(
            encoding_rs_io::DecodeReaderBytesBuilder::new()
                .encoding(Some(encoding_rs::WINDOWS_1252))
                .strip_bom(true)
                .build(buf),
        )
    }
}

/// Decodifica, para UTF-8, os bytes de um CSV que pode estar em UTF-8 ou em Windows-1252
/// (Latin-1). Tenta UTF-8 primeiro; se não for válido, assume Windows-1252, que nunca falha.
/// Remove o BOM de UTF-8 quando presente (ex.: arquivos da Câmara).
fn para_utf8(bytes: &[u8]) -> String {
    let mut texto = match std::str::from_utf8(bytes) {
        Ok(texto) => texto.to_owned(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    };
    if texto.starts_with('\u{feff}') {
        texto.drain(..'\u{feff}'.len_utf8());
    }
    texto
}

/// Descarta `n` linhas físicas do início do texto (preâmbulo antes do cabeçalho, ex.: a
/// linha "ULTIMA ATUALIZACAO" do CEAPS do Senado).
fn descartar_linhas(texto: &str, n: usize) -> &str {
    let mut resto = texto;
    for _ in 0..n {
        match resto.find('\n') {
            Some(fim) => resto = &resto[fim + 1..],
            None => return "",
        }
    }
    resto
}

/// Abre um CSV e separa o cabeçalho (colunas) do leitor de registros.
fn colunas_e_registros<R: Read>(
    leitor: R,
    delimitador: u8,
    sem_cabecalho: bool,
) -> Result<(Vec<String>, csv::Reader<R>), ErroDeRefino> {
    let mut leitor = csv::ReaderBuilder::new()
        .delimiter(delimitador)
        .flexible(true)
        .has_headers(!sem_cabecalho)
        .from_reader(leitor);
    let primeiro = leitor.headers()?;
    let colunas = if sem_cabecalho {
        (1..=primeiro.len())
            .map(|indice| format!("coluna_{indice:03}"))
            .collect()
    } else {
        primeiro.iter().map(|c| c.trim().to_owned()).collect()
    };
    Ok((colunas, leitor))
}

/// Lê os registros em lotes e os grava no escritor dado (memória ou arquivo).
fn alimentar<R: Read, W: std::io::Write + Send>(
    mut registros: csv::Reader<R>,
    escritor: &mut EscritorParquet<W>,
    num_colunas: usize,
) -> Result<(), ErroDeRefino> {
    let mut lote: Vec<Vec<Option<String>>> = Vec::with_capacity(LINHAS_POR_LOTE);
    for registro in registros.records() {
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
    escritor.escrever_lote(&lote)
}

/// Lê um CSV de um `Read` e devolve o Parquet **em memória** (para arquivos modestos).
fn parquet_de_leitor<R: Read>(
    leitor: R,
    delimitador: u8,
    sem_cabecalho: bool,
) -> Result<Vec<u8>, ErroDeRefino> {
    let (colunas, registros) = colunas_e_registros(leitor, delimitador, sem_cabecalho)?;
    let mut escritor = EscritorParquet::novo(&colunas)?;
    alimentar(registros, &mut escritor, colunas.len())?;
    escritor.finalizar()
}

/// Lê um CSV de um `Read` e grava o Parquet **direto em um arquivo temporário**, sem manter
/// o Parquet inteiro em memória — para arquivos grandes (#171).
fn parte_em_arquivo<R: Read>(
    nome: &str,
    leitor: R,
    delimitador: u8,
    sem_cabecalho: bool,
) -> Result<ParteBronze, ErroDeRefino> {
    let (colunas, registros) = colunas_e_registros(leitor, delimitador, sem_cabecalho)?;
    let arquivo = tempfile::NamedTempFile::new()?;
    let mut escritor = EscritorParquet::novo_em(arquivo.reopen()?, &colunas)?;
    alimentar(registros, &mut escritor, colunas.len())?;
    escritor.fechar()?;
    Ok(ParteBronze::de_arquivo(nome, arquivo.into_temp_path()))
}

/// Lê um CSV (já em UTF-8, em memória) pulando `linhas_a_pular` de preâmbulo.
fn parquet_de_csv(
    texto: &str,
    delimitador: u8,
    linhas_a_pular: usize,
    sem_cabecalho: bool,
) -> Result<Vec<u8>, ErroDeRefino> {
    parquet_de_leitor(
        descartar_linhas(texto, linhas_a_pular).as_bytes(),
        delimitador,
        sem_cabecalho,
    )
}

/// Extensões tabulares aceitas dentro do ZIP (o TSE usa `.csv` nos anos recentes e `.txt`
/// nos antigos, com o mesmo formato `;`/Latin-1).
fn e_tabular(nome: &str) -> bool {
    let n = nome.to_ascii_lowercase();
    n.ends_with(".csv") || n.ends_with(".txt")
}

/// Nome lógico de um arquivo interno do ZIP: o basename sem a extensão `.csv`/`.txt`.
fn nome_logico(nome_interno: &str) -> String {
    let base = nome_interno.rsplit('/').next().unwrap_or(nome_interno);
    match base.len() {
        n if n >= 4
            && (base[n - 4..].eq_ignore_ascii_case(".csv")
                || base[n - 4..].eq_ignore_ascii_case(".txt")) =>
        {
            base[..n - 4].to_owned()
        }
        _ => base.to_owned(),
    }
}

/// Os 27 TXT de receitas partidárias de 2012 com sufixo de duas letras começam por um registro; o
/// consolidado `_brasil` do mesmo ZIP tem cabeçalho e não entra nesta exceção.
fn sem_cabecalho_tse_2012(nome: &str) -> bool {
    nome.strip_prefix("receitas_partidos_2012_")
        .is_some_and(|uf| uf.len() == 2 && uf.bytes().all(|byte| byte.is_ascii_uppercase()))
}

/// Descompacta um ZIP e decodifica cada CSV em uma parte Parquet. Delega para
/// [`partes_de_zip_csv_em_fluxo`] — mantida para testes e chamadores com `&[u8]`.
pub fn partes_de_zip_csv(bytes: &[u8], delimitador: u8) -> Result<Vec<ParteBronze>, ErroDeRefino> {
    partes_de_zip_csv_em_fluxo(Cursor::new(bytes), delimitador)
}

/// Decodifica cada CSV de um ZIP **em fluxo**: lê a entrada do ZIP, transcodifica
/// encoding e parseia CSV direto para lotes Parquet em disco, sem materializar o
/// arquivo inteiro em memória. Usa `utf8_passthru`: se o CSV for UTF-8 válido,
/// passa direto; senão, transcodifica de Windows-1252 (Latin-1). Remove BOM quando
/// presente.
pub fn partes_de_zip_csv_em_fluxo<F: Read + Seek>(
    fonte: F,
    delimitador: u8,
) -> Result<Vec<ParteBronze>, ErroDeRefino> {
    let mut arquivo = zip::ZipArchive::new(fonte)?;
    let mut partes = Vec::new();

    for indice in 0..arquivo.len() {
        let entrada = arquivo.by_index(indice)?;
        if !entrada.is_file() {
            continue;
        }
        let nome_interno = entrada.name()?.into_owned();
        if !e_tabular(&nome_interno) {
            continue;
        }
        let leitor = leitor_auto_encoding(entrada);
        let nome = nome_logico(&nome_interno);
        partes.push(parte_em_arquivo(
            &nome,
            leitor,
            delimitador,
            sem_cabecalho_tse_2012(&nome),
        )?);
    }

    if partes.is_empty() {
        return Err(ErroDeRefino::Decodificacao(
            "nenhum CSV encontrado no ZIP".to_owned(),
        ));
    }
    Ok(partes)
}

/// Decodifica um CSV "solto" (não zipado) em uma única parte Parquet, com o `nome` lógico
/// dado. `linhas_a_pular` descarta um preâmbulo antes do cabeçalho (ex.: CEAPS do Senado).
pub fn partes_de_csv(
    bytes: &[u8],
    nome: &str,
    delimitador: u8,
    linhas_a_pular: usize,
) -> Result<Vec<ParteBronze>, ErroDeRefino> {
    let texto = para_utf8(bytes);
    let parquet = parquet_de_csv(&texto, delimitador, linhas_a_pular, false)?;
    Ok(vec![ParteBronze::nova(nome, &parquet)?])
}

/// Como [`partes_de_csv`], mas lê o CSV **em fluxo** a partir de um `Read`, transcodificando
/// encoding e gravando Parquet em disco sem materializar o CSV inteiro em memória.
pub fn partes_de_csv_em_fluxo<R: Read>(
    fonte: R,
    nome: &str,
    delimitador: u8,
    linhas_a_pular: usize,
) -> Result<Vec<ParteBronze>, ErroDeRefino> {
    let leitor = leitor_auto_encoding(fonte);
    let mut leitor_buf = BufReader::new(leitor);
    for _ in 0..linhas_a_pular {
        let mut lixo = String::new();
        leitor_buf.read_line(&mut lixo)?;
    }
    Ok(vec![parte_em_arquivo(
        nome,
        leitor_buf,
        delimitador,
        false,
    )?])
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

    /// Lê o Parquet de uma [`ParteBronze`] (em disco) de volta em (colunas, linhas).
    pub(crate) fn ler_parte(parte: &ParteBronze) -> (Vec<String>, Vec<Vec<Option<String>>>) {
        ler_parquet(&std::fs::read(&parte.parquet).unwrap())
    }

    /// Lê um Parquet (bytes) de volta em (colunas, linhas) para as asserções.
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
        let (colunas, linhas) = ler_parte(&partes[0]);
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
        let (_, linhas) = ler_parte(&partes[0]);
        assert_eq!(linhas, vec![vec![c("João")]]);
    }

    #[test]
    fn aceita_txt_alem_de_csv() {
        let partes =
            partes_de_zip_csv(&zip_com("votacao_1998_BR.txt", b"a;b\n1;2\n"), b';').unwrap();
        assert_eq!(partes.len(), 1);
        assert_eq!(partes[0].nome, "votacao_1998_BR");
    }

    #[test]
    fn preserva_primeiro_registro_de_receitas_partidos_estadual_2012() {
        let txt = b"15/05/201313:46:47;MT;100\n16/05/201313:46:47;MT;200\n";
        let zip = zip_com("receitas_partidos_2012_MT.txt", txt);
        let partes = partes_de_zip_csv_em_fluxo(Cursor::new(&zip), b';').unwrap();
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, vec!["coluna_001", "coluna_002", "coluna_003"]);
        assert_eq!(linhas.len(), 2);
        assert_eq!(linhas[0], vec![c("15/05/201313:46:47"), c("MT"), c("100")]);

        let em_memoria = partes_de_zip_csv(&zip, b';').unwrap();
        assert_eq!(ler_parte(&em_memoria[0]), (colunas, linhas));
    }

    #[test]
    fn preserva_cabecalho_do_consolidado_brasil_2012() {
        let zip = zip_com("receitas_partidos_2012_brasil.txt", b"id;uf\n1;BR\n");
        let partes = partes_de_zip_csv_em_fluxo(Cursor::new(zip), b';').unwrap();
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, vec!["id", "uf"]);
        assert_eq!(linhas, vec![vec![c("1"), c("BR")]]);
    }

    #[test]
    fn ignora_entradas_que_nao_sao_csv() {
        let mut buffer = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut buffer));
            zw.start_file("leia-me.pdf", SimpleFileOptions::default())
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
        let (_, linhas) = ler_parte(&partes[0]);
        assert_eq!(linhas.len(), total);
        assert_eq!(linhas[0], vec![c("0")]);
        assert_eq!(linhas[total - 1], vec![c(&(total - 1).to_string())]);
    }

}
