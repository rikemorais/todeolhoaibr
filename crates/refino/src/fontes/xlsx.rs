//! Decodificador genérico de XLSX: lê a primeira aba (ou uma aba nomeada) e converte
//! todas as linhas em uma tabela bronze. Serve para fontes como SEEG, MapBiomas, IMF (WEO)
//! e FBSP (segurança) cujos XLSX são tabulares sem transformação complexa.

use std::io::Cursor;

use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, Tabela};

/// Lê a primeira aba (ou uma aba nomeada) e converte todas as linhas em tabela bronze.
pub struct DecodificadorXlsx {
    nome: &'static str,
    aba: AbaAlvo,
    linhas_a_pular: usize,
}

enum AbaAlvo {
    Primeira,
    Nomeada(&'static str),
}

impl DecodificadorXlsx {
    /// Cria o decodificador lendo a primeira aba do XLSX.
    pub const fn novo(nome: &'static str) -> Self {
        Self {
            nome,
            aba: AbaAlvo::Primeira,
            linhas_a_pular: 0,
        }
    }

    /// Cria o decodificador lendo uma aba específica pelo nome.
    pub const fn com_aba(nome: &'static str, aba: &'static str) -> Self {
        Self {
            nome,
            aba: AbaAlvo::Nomeada(aba),
            linhas_a_pular: 0,
        }
    }

    /// Pula `n` linhas antes do cabeçalho (preâmbulo).
    pub const fn com_pular(mut self, n: usize) -> Self {
        self.linhas_a_pular = n;
        self
    }
}

impl Decodificador for DecodificadorXlsx {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let mut workbook: Xlsx<_> = open_workbook_from_rs(Cursor::new(bytes))
            .map_err(|e| ErroDeRefino::Decodificacao(format!("XLSX inválido: {e}")))?;

        let planilha = match &self.aba {
            AbaAlvo::Primeira => {
                let nome_aba = workbook
                    .sheet_names()
                    .first()
                    .ok_or_else(|| ErroDeRefino::Decodificacao("XLSX sem abas".into()))?
                    .clone();
                workbook.worksheet_range(&nome_aba)
            }
            AbaAlvo::Nomeada(nome) => workbook.worksheet_range(nome),
        }
        .map_err(|e| ErroDeRefino::Decodificacao(format!("aba inválida: {e}")))?;

        let linhas: Vec<Vec<Data>> = planilha.rows().map(|r| r.to_vec()).collect();
        if linhas.len() <= self.linhas_a_pular {
            return Ok(vec![]);
        }

        let cabecalho = &linhas[self.linhas_a_pular];
        let colunas: Vec<String> = cabecalho
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let texto = celula_para_texto(c);
                if texto.is_empty() {
                    format!("coluna_{i}")
                } else {
                    texto
                }
            })
            .collect();
        let n_colunas = colunas.len();

        let mut tabela = Tabela::nova(colunas);
        for linha in &linhas[self.linhas_a_pular + 1..] {
            let vazia = linha.iter().all(|c| matches!(c, Data::Empty));
            if vazia {
                continue;
            }
            let valores: Vec<Option<String>> = (0..n_colunas)
                .map(|i| {
                    linha
                        .get(i)
                        .filter(|c| !matches!(c, Data::Empty))
                        .map(celula_para_texto)
                })
                .collect();
            tabela = tabela.com_linha(valores);
        }

        Ok(vec![ParteBronze::de_tabela(self.nome, &tabela)?])
    }
}

fn celula_para_texto(celula: &Data) -> String {
    match celula {
        Data::Empty => String::new(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        _ => celula.to_string(),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

    use crate::csv_zip::testes::ler_parte;

    fn xlsx_simples(cabecalho: &[&str], linhas: &[Vec<&str>]) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut bytes);
            let opcoes = SimpleFileOptions::default();
            let entradas = [
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
            ];
            for (nome, conteudo) in entradas {
                zip.start_file(nome, opcoes).unwrap();
                zip.write_all(conteudo.as_bytes()).unwrap();
            }

            zip.start_file("xl/worksheets/sheet1.xml", opcoes).unwrap();
            let mut xml = String::from(
                r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>"#,
            );

            // Header row
            xml.push_str(r#"<row r="1">"#);
            for (j, col) in cabecalho.iter().enumerate() {
                let col_letra = (b'A' + j as u8) as char;
                xml.push_str(&format!(
                    r#"<c r="{col_letra}1" t="inlineStr"><is><t>{col}</t></is></c>"#
                ));
            }
            xml.push_str("</row>");

            // Data rows
            for (i, linha) in linhas.iter().enumerate() {
                let r = i + 2;
                xml.push_str(&format!(r#"<row r="{r}">"#));
                for (j, val) in linha.iter().enumerate() {
                    let col_letra = (b'A' + j as u8) as char;
                    xml.push_str(&format!(
                        r#"<c r="{col_letra}{r}" t="inlineStr"><is><t>{val}</t></is></c>"#
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
    fn decodifica_xlsx_simples() {
        let bytes = xlsx_simples(
            &["pais", "ano", "valor"],
            &[
                vec!["Brasil", "2020", "100"],
                vec!["Argentina", "2020", "50"],
            ],
        );
        let dec = DecodificadorXlsx::novo("teste");
        let partes = dec.decodificar(&bytes, "").unwrap();
        assert_eq!(partes.len(), 1);
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, ["pais", "ano", "valor"]);
        assert_eq!(linhas.len(), 2);
        assert_eq!(linhas[0][0], Some("Brasil".into()));
    }
}
