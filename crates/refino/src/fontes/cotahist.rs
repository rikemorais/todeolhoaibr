//! Decodificador do bronze para o arquivo COTAHIST da B3 (fixed-width).
//!
//! Cada registro tem 245 bytes. O tipo 01 contém as cotações diárias;
//! tipos 00 (header) e 99 (trailer) são ignorados. Campos numéricos com
//! decimais implícitos (`(11)V99`, `(16)V99`, `(07)V06`) são convertidos
//! para texto decimal ("12345" com V99 → "123.45").

use std::io::{BufRead, BufReader, Read};

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, EscritorParquet, Tabela};
use tempfile::NamedTempFile;

const TAMANHO_REGISTRO: usize = 245;
const LINHAS_POR_LOTE: usize = 50_000;

struct Campo {
    nome: &'static str,
    inicio: usize,
    fim: usize,
    decimais: u8,
}

const CAMPOS: &[Campo] = &[
    Campo {
        nome: "tipreg",
        inicio: 0,
        fim: 2,
        decimais: 0,
    },
    Campo {
        nome: "data_pregao",
        inicio: 2,
        fim: 10,
        decimais: 0,
    },
    Campo {
        nome: "codbdi",
        inicio: 10,
        fim: 12,
        decimais: 0,
    },
    Campo {
        nome: "codneg",
        inicio: 12,
        fim: 24,
        decimais: 0,
    },
    Campo {
        nome: "tpmerc",
        inicio: 24,
        fim: 27,
        decimais: 0,
    },
    Campo {
        nome: "nomres",
        inicio: 27,
        fim: 39,
        decimais: 0,
    },
    Campo {
        nome: "especi",
        inicio: 39,
        fim: 49,
        decimais: 0,
    },
    Campo {
        nome: "prazot",
        inicio: 49,
        fim: 52,
        decimais: 0,
    },
    Campo {
        nome: "modref",
        inicio: 52,
        fim: 56,
        decimais: 0,
    },
    Campo {
        nome: "preabe",
        inicio: 56,
        fim: 69,
        decimais: 2,
    },
    Campo {
        nome: "premax",
        inicio: 69,
        fim: 82,
        decimais: 2,
    },
    Campo {
        nome: "premin",
        inicio: 82,
        fim: 95,
        decimais: 2,
    },
    Campo {
        nome: "premed",
        inicio: 95,
        fim: 108,
        decimais: 2,
    },
    Campo {
        nome: "preult",
        inicio: 108,
        fim: 121,
        decimais: 2,
    },
    Campo {
        nome: "preofc",
        inicio: 121,
        fim: 134,
        decimais: 2,
    },
    Campo {
        nome: "preofv",
        inicio: 134,
        fim: 147,
        decimais: 2,
    },
    Campo {
        nome: "totneg",
        inicio: 147,
        fim: 152,
        decimais: 0,
    },
    Campo {
        nome: "quatot",
        inicio: 152,
        fim: 170,
        decimais: 0,
    },
    Campo {
        nome: "voltot",
        inicio: 170,
        fim: 188,
        decimais: 2,
    },
    Campo {
        nome: "preexe",
        inicio: 188,
        fim: 201,
        decimais: 2,
    },
    Campo {
        nome: "indopc",
        inicio: 201,
        fim: 202,
        decimais: 0,
    },
    Campo {
        nome: "datven",
        inicio: 202,
        fim: 210,
        decimais: 0,
    },
    Campo {
        nome: "fatcot",
        inicio: 210,
        fim: 217,
        decimais: 0,
    },
    Campo {
        nome: "ptoexe",
        inicio: 217,
        fim: 230,
        decimais: 6,
    },
    Campo {
        nome: "codisi",
        inicio: 230,
        fim: 242,
        decimais: 0,
    },
    Campo {
        nome: "dismes",
        inicio: 242,
        fim: 245,
        decimais: 0,
    },
];

fn campo_texto(linha: &[u8], campo: &Campo) -> Option<String> {
    if campo.inicio >= linha.len() || campo.fim > linha.len() {
        return None;
    }
    let fatia = &linha[campo.inicio..campo.fim];
    let texto = String::from_utf8_lossy(fatia).trim().to_string();
    if texto.is_empty() {
        return None;
    }
    if campo.decimais == 0 {
        return Some(texto);
    }
    formatar_decimal(&texto, campo.decimais)
}

fn formatar_decimal(inteiro: &str, decimais: u8) -> Option<String> {
    let limpo: String = inteiro.chars().filter(|c| c.is_ascii_digit()).collect();
    if limpo.is_empty() || limpo.chars().all(|c| c == '0') {
        return Some("0".to_string());
    }
    let dec = decimais as usize;
    if limpo.len() <= dec {
        let zeros = "0".repeat(dec - limpo.len());
        return Some(format!("0.{zeros}{limpo}"));
    }
    let (parte_int, parte_dec) = limpo.split_at(limpo.len() - dec);
    let int_limpo = parte_int.trim_start_matches('0');
    let inteiro_final = if int_limpo.is_empty() { "0" } else { int_limpo };
    Some(format!("{inteiro_final}.{parte_dec}"))
}

/// Decodifica um ZIP contendo o arquivo COTAHIST (TXT fixed-width de 245 bytes por linha).
pub struct DecodificadorCotahist;

fn extrair_txt_do_zip(bytes: &[u8]) -> Result<Vec<u8>, ErroDeRefino> {
    let cursor = std::io::Cursor::new(bytes);
    let mut arquivo = zip::ZipArchive::new(cursor)
        .map_err(|e| ErroDeRefino::Decodificacao(format!("ZIP inválido: {e}")))?;
    for i in 0..arquivo.len() {
        let mut entrada = arquivo
            .by_index(i)
            .map_err(|e| ErroDeRefino::Decodificacao(format!("entrada ZIP: {e}")))?;
        let nome = entrada
            .name()
            .map_err(|e| ErroDeRefino::Decodificacao(format!("nome ZIP: {e}")))?;
        if membro_cotahist(&nome) {
            let mut conteudo = Vec::new();
            entrada
                .read_to_end(&mut conteudo)
                .map_err(|e| ErroDeRefino::Decodificacao(format!("leitura ZIP: {e}")))?;
            return Ok(conteudo);
        }
    }
    Err(ErroDeRefino::Decodificacao(
        "ZIP não contém arquivo COTAHIST".into(),
    ))
}

impl Decodificador for DecodificadorCotahist {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let txt = extrair_txt_do_zip(bytes)?;
        decodificar_cotahist(&txt)
    }

    fn decodificar_arquivo(
        &self,
        caminho: &std::path::Path,
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let arquivo = std::fs::File::open(caminho)?;
        let mut zip = zip::ZipArchive::new(arquivo)
            .map_err(|e| ErroDeRefino::Decodificacao(format!("ZIP inválido: {e}")))?;
        for i in 0..zip.len() {
            let mut entrada = zip
                .by_index(i)
                .map_err(|e| ErroDeRefino::Decodificacao(format!("entrada ZIP: {e}")))?;
            let nome = entrada
                .name()
                .map_err(|e| ErroDeRefino::Decodificacao(format!("nome ZIP: {e}")))?;
            if membro_cotahist(&nome) {
                return decodificar_cotahist_em_fluxo(&mut entrada);
            }
        }
        Err(ErroDeRefino::Decodificacao(
            "ZIP não contém arquivo COTAHIST".into(),
        ))
    }
}

fn membro_cotahist(nome: &str) -> bool {
    nome.rsplit('/')
        .next()
        .is_some_and(|nome| nome.to_ascii_lowercase().starts_with("cotahist"))
}

fn decodificar_cotahist_em_fluxo<R: Read>(fonte: R) -> Result<Vec<ParteBronze>, ErroDeRefino> {
    let colunas: Vec<String> = CAMPOS
        .iter()
        .skip(1)
        .map(|campo| campo.nome.to_string())
        .collect();
    let arquivo = NamedTempFile::new()?;
    let mut escritor = EscritorParquet::novo_em(arquivo.reopen()?, &colunas)?;
    let mut leitor = BufReader::new(fonte);
    let mut linha = Vec::new();
    let mut lote = Vec::with_capacity(LINHAS_POR_LOTE);
    let mut contagem = 0u64;

    loop {
        linha.clear();
        if leitor.read_until(b'\n', &mut linha)? == 0 {
            break;
        }
        if linha.last() == Some(&b'\n') {
            linha.pop();
        }
        let linha = linha.strip_suffix(b"\r").unwrap_or(&linha);
        if linha.len() < TAMANHO_REGISTRO || !linha.starts_with(b"01") {
            continue;
        }
        lote.push(
            CAMPOS
                .iter()
                .skip(1)
                .map(|campo| campo_texto(linha, campo))
                .collect(),
        );
        contagem += 1;
        if lote.len() >= LINHAS_POR_LOTE {
            escritor.escrever_lote(&lote)?;
            lote.clear();
        }
    }
    escritor.escrever_lote(&lote)?;
    escritor.fechar()?;

    if contagem == 0 {
        return Ok(Vec::new());
    }
    Ok(vec![ParteBronze::de_arquivo(
        "cotahist",
        arquivo.into_temp_path(),
    )])
}

fn decodificar_cotahist(bytes: &[u8]) -> Result<Vec<ParteBronze>, ErroDeRefino> {
    let colunas: Vec<String> = CAMPOS.iter().skip(1).map(|c| c.nome.to_string()).collect();

    let mut tabela = Tabela::nova(colunas);
    let mut contagem = 0u64;

    for linha in bytes.split(|&b| b == b'\n') {
        let linha = linha.strip_suffix(b"\r").unwrap_or(linha);
        if linha.len() < TAMANHO_REGISTRO {
            continue;
        }
        if !linha.starts_with(b"01") {
            continue;
        }
        let valores: Vec<Option<String>> = CAMPOS
            .iter()
            .skip(1)
            .map(|campo| campo_texto(linha, campo))
            .collect();
        tabela = tabela.com_linha(valores);
        contagem += 1;
    }

    if contagem == 0 {
        return Ok(Vec::new());
    }

    Ok(vec![ParteBronze::de_tabela("cotahist", &tabela)?])
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::csv_zip::testes::ler_parte;

    fn c(s: &str) -> Option<String> {
        Some(s.to_owned())
    }

    fn zip_com_txt(nome: &str, conteudo: &[u8]) -> Vec<u8> {
        use std::io::Write;
        use zip::write::SimpleFileOptions;
        let buf = Vec::new();
        let cursor = std::io::Cursor::new(buf);
        let mut zip = zip::ZipWriter::new(cursor);
        zip.start_file(nome, SimpleFileOptions::default()).unwrap();
        zip.write_all(conteudo).unwrap();
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn formata_decimal_com_zeros_a_esquerda() {
        assert_eq!(formatar_decimal("0000001234500", 2), c("12345.00"));
        assert_eq!(formatar_decimal("0000000000100", 2), c("1.00"));
        assert_eq!(formatar_decimal("0000000000000", 2), Some("0".to_string()));
    }

    #[test]
    fn formata_decimal_com_6_casas() {
        assert_eq!(formatar_decimal("0001234567890", 6), c("1234.567890"));
    }

    #[test]
    fn decodifica_registro_tipo_01_de_zip() {
        let mut registro = vec![b' '; TAMANHO_REGISTRO];
        registro[0] = b'0';
        registro[1] = b'1';
        for (i, b) in b"20240102".iter().enumerate() {
            registro[2 + i] = *b;
        }
        registro[10] = b'0';
        registro[11] = b'2';
        for (i, b) in b"PETR4".iter().enumerate() {
            registro[12 + i] = *b;
        }
        for (i, b) in b"010".iter().enumerate() {
            registro[24 + i] = *b;
        }
        for (i, b) in b"0000000003850".iter().enumerate() {
            registro[56 + i] = *b;
        }
        for (i, b) in b"0000000003950".iter().enumerate() {
            registro[69 + i] = *b;
        }
        registro.push(b'\n');

        let dados_zip = zip_com_txt("COTAHIST_A2024.TXT", &registro);
        let partes = DecodificadorCotahist
            .decodificar(&dados_zip, "application/zip")
            .unwrap();
        assert_eq!(partes.len(), 1);
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert!(colunas.contains(&"data_pregao".to_string()));
        assert!(colunas.contains(&"codneg".to_string()));
        assert_eq!(linhas.len(), 1);
        assert_eq!(linhas[0][0], c("20240102"));
        assert_eq!(linhas[0][2], c("PETR4"));
        assert_eq!(linhas[0][8], c("38.50"));
        assert_eq!(linhas[0][9], c("39.50"));
    }

    #[test]
    fn decodifica_nome_historico_cotahist_sem_extensao_txt() {
        let mut registro = vec![b' '; TAMANHO_REGISTRO];
        registro[..2].copy_from_slice(b"01");
        registro[2..10].copy_from_slice(b"20240102");
        registro[12..17].copy_from_slice(b"PETR4");
        registro.push(b'\n');
        let dados_zip = zip_com_txt("COTAHIST.A1998", &registro);

        let partes = DecodificadorCotahist
            .decodificar(&dados_zip, "application/zip")
            .unwrap();

        assert_eq!(partes.len(), 1);
        assert_eq!(ler_parte(&partes[0]).1.len(), 1);
    }

    #[test]
    fn decodifica_arquivo_cotahist_em_fluxo_para_disco() {
        use std::io::Write;

        let mut registro = vec![b' '; TAMANHO_REGISTRO];
        registro[..2].copy_from_slice(b"01");
        registro[2..10].copy_from_slice(b"20240102");
        registro[12..17].copy_from_slice(b"PETR4");
        registro.push(b'\n');
        let zip = zip_com_txt("COTAHIST_A2024.TXT", &registro);
        let mut arquivo = NamedTempFile::new().unwrap();
        arquivo.write_all(&zip).unwrap();

        let partes = DecodificadorCotahist
            .decodificar_arquivo(arquivo.path(), "application/zip")
            .unwrap();

        assert_eq!(partes.len(), 1);
        assert_eq!(ler_parte(&partes[0]).1.len(), 1);
    }

    #[test]
    fn ignora_header_e_trailer() {
        let header = format!("{:0<245}\n", "00COTAHIST.2024");
        let trailer = format!("{:0<245}\n", "99");
        let conteudo = format!("{header}{trailer}");
        let dados_zip = zip_com_txt("COTAHIST_A2024.TXT", conteudo.as_bytes());
        let partes = DecodificadorCotahist
            .decodificar(&dados_zip, "application/zip")
            .unwrap();
        assert!(partes.is_empty());
    }
}
