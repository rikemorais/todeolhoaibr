//! Helper compartilhado: decodificar um ZIP de CSVs (reencodando para UTF-8) em tabelas.
//!
//! Boa parte das fontes de arquivo grava, no raw, um ZIP com um ou mais CSVs — às vezes em
//! Latin-1 (CEAPS do Senado, TSE), às vezes em UTF-8 (CEAP da Câmara). O bronze só traduz:
//! descompacta, reencoda e põe em colunas, mantendo os valores como texto.

use std::io::{Cursor, Read};

use crate::decodificador::TabelaNomeada;
use crate::escritor::{ErroDeRefino, Tabela};

/// Decodifica, para UTF-8, os bytes de um CSV que pode estar em UTF-8 ou em Windows-1252
/// (Latin-1). Tenta UTF-8 primeiro; se não for válido, assume Windows-1252, que nunca falha.
fn para_utf8(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(texto) => texto.to_owned(),
        Err(_) => encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned(),
    }
}

/// Lê um CSV (já em UTF-8) com o delimitador dado, em uma [`Tabela`] de texto.
fn tabela_de_csv(texto: &str, delimitador: u8) -> Result<Tabela, ErroDeRefino> {
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

    let mut tabela = Tabela::nova(colunas);
    for registro in leitor.records() {
        let registro = registro?;
        // Mantém o número de colunas do cabeçalho: completa com nulos ou ignora excedentes,
        // sem interpretar o conteúdo (fidelidade do bronze).
        let mut linha: Vec<Option<String>> = (0..num_colunas)
            .map(|i| registro.get(i).map(|v| v.to_owned()))
            .collect();
        linha.truncate(num_colunas);
        tabela.linhas.push(linha);
    }
    Ok(tabela)
}

/// Descompacta um ZIP em memória e decodifica cada CSV em uma tabela, nomeada pelo arquivo
/// interno (sem a extensão). Entradas que não sejam `.csv` são ignoradas.
pub fn tabelas_de_zip_csv(
    bytes: &[u8],
    delimitador: u8,
) -> Result<Vec<TabelaNomeada>, ErroDeRefino> {
    let mut arquivo = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut tabelas = Vec::new();

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
        let tabela = tabela_de_csv(&texto, delimitador)?;
        let base = nome_interno.rsplit('/').next().unwrap_or(&nome_interno);
        let nome = match base.len() {
            n if n >= 4 && base[n - 4..].eq_ignore_ascii_case(".csv") => base[..n - 4].to_owned(),
            _ => base.to_owned(),
        };
        tabelas.push(TabelaNomeada::nova(nome, tabela));
    }

    if tabelas.is_empty() {
        return Err(ErroDeRefino::Decodificacao(
            "nenhum CSV encontrado no ZIP".to_owned(),
        ));
    }
    Ok(tabelas)
}

#[cfg(test)]
mod testes {
    use super::*;
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

    #[test]
    fn decodifica_csv_utf8_separado_por_ponto_e_virgula() {
        let csv = "id;nome\n1;São Paulo\n2;\n";
        let tabelas = tabelas_de_zip_csv(&zip_com("Ano-2024.csv", csv.as_bytes()), b';').unwrap();
        assert_eq!(tabelas.len(), 1);
        assert_eq!(tabelas[0].nome, "Ano-2024");
        assert_eq!(tabelas[0].tabela.colunas, vec!["id", "nome"]);
        assert_eq!(
            tabelas[0].tabela.linhas,
            vec![
                vec![Some("1".into()), Some("São Paulo".into())],
                vec![Some("2".into()), Some("".into())],
            ]
        );
    }

    #[test]
    fn reencoda_latin1_para_utf8() {
        // "João" em Windows-1252: 0x4A 0x6F 0xE3 0x6F.
        let mut csv = b"cidade\n".to_vec();
        csv.extend_from_slice(&[0x4A, 0x6F, 0xE3, 0x6F]);
        csv.push(b'\n');
        let tabelas = tabelas_de_zip_csv(&zip_com("x.csv", &csv), b';').unwrap();
        assert_eq!(tabelas[0].tabela.linhas, vec![vec![Some("João".into())]]);
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
        let tabelas = tabelas_de_zip_csv(&buffer, b';').unwrap();
        assert_eq!(tabelas.len(), 1);
        assert_eq!(tabelas[0].nome, "dados");
    }
}
