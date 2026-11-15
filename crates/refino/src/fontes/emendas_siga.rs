//! Decodifica o XLSX completo de favorecidos do SIGA Brasil para Parquet Bronze.

use std::io::{Cursor, Read, Seek};

use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, EscritorParquet};

const NOME_TABELA: &str = "favorecidos-pagamento";
const LINHAS_POR_LOTE: usize = 10_000;
const COLUNAS: [&str; 8] = [
    "Favorecido do Pagamento - Município/UF",
    "Favorecido do Pagamento (CPF/CNPJ)",
    "Favorecido do Pagamento (Natureza Subgrupo)",
    "Favorecido do Pagamento (Tipo)",
    "Favorecido do Pagamento (UF)",
    "Autor",
    "Emenda (Número/Ano)",
    "Pago + RP (Favorecido lista OB)",
];

/// Converte o XLSX do SIGA em Parquet preservando o cabeçalho e as células como texto.
pub struct DecodificadorEmendasSiga;

impl Decodificador for DecodificadorEmendasSiga {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let workbook = open_workbook_from_rs(Cursor::new(bytes)).map_err(|erro| {
            ErroDeRefino::Decodificacao(format!("XLSX inválido do SIGA Brasil: {erro}"))
        })?;
        converter(workbook)
    }

    fn decodificar_arquivo(
        &self,
        caminho: &std::path::Path,
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let arquivo = std::fs::File::open(caminho)?;
        let workbook = open_workbook_from_rs(arquivo).map_err(|erro| {
            ErroDeRefino::Decodificacao(format!("XLSX inválido do SIGA Brasil: {erro}"))
        })?;
        converter(workbook)
    }
}

fn converter<R: Read + Seek>(mut workbook: Xlsx<R>) -> Result<Vec<ParteBronze>, ErroDeRefino> {
    let planilha = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| ErroDeRefino::Decodificacao("XLSX do SIGA sem planilhas".into()))?
        .map_err(|erro| {
            ErroDeRefino::Decodificacao(format!("não foi possível ler a planilha SIGA: {erro}"))
        })?;
    let mut linhas = planilha.rows();
    let cabecalho = linhas
        .next()
        .ok_or_else(|| ErroDeRefino::Decodificacao("planilha SIGA vazia".into()))?;
    let colunas = cabecalho
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if colunas.iter().map(String::as_str).ne(COLUNAS) {
        return Err(ErroDeRefino::Decodificacao(format!(
            "schema XLSX do SIGA mudou; esperado: {}",
            COLUNAS.join(" | ")
        )));
    }

    let temporario = tempfile::NamedTempFile::new()?;
    let mut escritor = EscritorParquet::novo_em(temporario.reopen()?, &colunas)?;
    let mut lote = Vec::with_capacity(LINHAS_POR_LOTE);
    for linha in linhas {
        if linha.iter().all(|celula| matches!(celula, Data::Empty)) {
            continue;
        }
        let valores = (0..COLUNAS.len())
            .map(|indice| {
                linha
                    .get(indice)
                    .filter(|celula| !matches!(celula, Data::Empty))
                    .map(ToString::to_string)
            })
            .collect::<Vec<_>>();
        lote.push(valores);
        if lote.len() == LINHAS_POR_LOTE {
            escritor.escrever_lote(&lote)?;
            lote.clear();
        }
    }
    escritor.escrever_lote(&lote)?;
    escritor.fechar()?;

    Ok(vec![ParteBronze::de_arquivo(
        NOME_TABELA,
        temporario.into_temp_path(),
    )])
}

#[cfg(test)]
mod testes {
    use std::io::{Cursor, Write};

    use zip::{ZipWriter, write::SimpleFileOptions};

    use super::*;
    use crate::csv_zip::testes::ler_parte;

    fn xlsx(headers: &[&str], rows: &[&[&str]]) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut bytes);
            let options = SimpleFileOptions::default();
            for (name, xml) in [
                (
                    "[Content_Types].xml",
                    r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
                ),
                (
                    "_rels/.rels",
                    r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
                ),
                (
                    "xl/workbook.xml",
                    r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Dados" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
                ),
                (
                    "xl/_rels/workbook.xml.rels",
                    r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
                ),
            ] {
                zip.start_file(name, options).unwrap();
                zip.write_all(xml.as_bytes()).unwrap();
            }
            zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
            let mut xml = String::from(
                r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
            );
            for (linha, valores) in std::iter::once(headers)
                .chain(rows.iter().copied())
                .enumerate()
            {
                xml.push_str(&format!(r#"<row r="{}">"#, linha + 1));
                for (indice, valor) in valores.iter().enumerate() {
                    let mut coluna = indice + 1;
                    let mut letra = String::new();
                    while coluna > 0 {
                        let resto = (coluna - 1) % 26;
                        letra.insert(0, char::from(b'A' + resto as u8));
                        coluna = (coluna - 1) / 26;
                    }
                    xml.push_str(&format!(
                        r#"<c r="{letra}{}" t="inlineStr"><is><t>{valor}</t></is></c>"#,
                        linha + 1
                    ));
                }
                xml.push_str("</row>");
            }
            xml.push_str("</sheetData></worksheet>");
            zip.write_all(xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        bytes.into_inner()
    }

    #[test]
    fn preserva_o_schema_e_os_valores_do_xlsx_em_parquet() {
        let valores = [
            "MUNICIPIO DE BONFIM-BONFIM-RR",
            "DADO_PESSOAL_TESTE",
            "ÓRGÃO PÚBLICO MUNICIPAL",
            "PESSOA JURIDICA",
            "RR",
            "AUTOR DE TESTE",
            "30300001-2016",
            "2157073",
        ];
        let refs = COLUNAS.to_vec();
        let bytes = xlsx(&refs, &[&valores]);

        let partes = DecodificadorEmendasSiga
            .decodificar(
                &bytes,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            )
            .unwrap();
        let (colunas, linhas) = ler_parte(&partes[0]);

        assert_eq!(partes[0].nome, NOME_TABELA);
        assert_eq!(colunas, COLUNAS);
        assert_eq!(linhas.len(), 1);
        assert_eq!(linhas[0][0].as_deref(), Some(valores[0]));
        assert_eq!(linhas[0][1].as_deref(), Some(valores[1]));
        assert_eq!(linhas[0][7].as_deref(), Some(valores[7]));
    }

    #[test]
    fn rejeita_schema_alterado_antes_de_publicar_bronze() {
        let bytes = xlsx(&["coluna inesperada"], &[]);

        let erro = DecodificadorEmendasSiga
            .decodificar(&bytes, "application/octet-stream")
            .unwrap_err();

        assert!(erro.to_string().contains("schema XLSX do SIGA mudou"));
    }

    #[test]
    fn rejeita_arquivo_que_nao_e_xlsx() {
        assert!(
            DecodificadorEmendasSiga
                .decodificar(b"nao-xlsx", "")
                .is_err()
        );
    }
}
