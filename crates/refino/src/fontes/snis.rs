//! Decodificador Bronze para ZIP/XLSX do SINISA (ex-SNIS).
//!
//! Cada ZIP do SINISA contém subpastas (Base Municipal, Consolidado UF,
//! Locais+Regionais). O decodificador filtra apenas os XLSX em subpastas
//! "Base Municipal", localiza a linha de cabeçalho com códigos máquina
//! (cod_IBGE, CAD0002, IAG0001, …) e emite todos os dados como strings.

use std::io::{Cursor, Read};

use calamine::{Data, Reader, open_workbook_auto_from_rs};

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, Tabela};

/// Decodifica ZIPs do SINISA: extrai XLSX de "Base Municipal" e converte em Parquet.
pub struct DecodificadorSnis;

impl Decodificador for DecodificadorSnis {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let cursor = Cursor::new(bytes);
        let mut zip = zip::ZipArchive::new(cursor)
            .map_err(|erro| ErroDeRefino::Decodificacao(format!("ZIP inválido: {erro}")))?;

        let nomes: Vec<String> = (0..zip.len())
            .filter_map(|i| {
                zip.by_index(i)
                    .ok()
                    .and_then(|e| e.name().ok().map(|n| n.to_string()))
            })
            .collect();

        let mut partes = Vec::new();

        for nome in &nomes {
            if !eh_base_municipal(nome) {
                continue;
            }

            let mut bytes_xlsx = Vec::new();
            zip.by_name(nome)
                .map_err(|erro| {
                    ErroDeRefino::Decodificacao(format!("não foi possível ler {nome}: {erro}"))
                })?
                .read_to_end(&mut bytes_xlsx)?;

            let parte = decodificar_xlsx(&bytes_xlsx, nome)?;
            if let Some(p) = parte {
                partes.push(p);
            }
        }

        Ok(partes)
    }
}

fn eh_base_municipal(nome: &str) -> bool {
    let lower = nome.to_lowercase();
    lower.contains("base municipal") && lower.ends_with(".xlsx")
}

fn decodificar_xlsx(bytes: &[u8], nome_arquivo: &str) -> Result<Option<ParteBronze>, ErroDeRefino> {
    let mut workbook = open_workbook_auto_from_rs(Cursor::new(bytes)).map_err(|erro| {
        ErroDeRefino::Decodificacao(format!("XLSX inválido ({nome_arquivo}): {erro}"))
    })?;

    let aba = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| ErroDeRefino::Decodificacao(format!("nenhuma aba em {nome_arquivo}")))?;

    let planilha = workbook.worksheet_range(&aba).map_err(|erro| {
        ErroDeRefino::Decodificacao(format!("aba inválida em {nome_arquivo}: {erro}"))
    })?;

    let linhas: Vec<Vec<Data>> = planilha.rows().map(|r| r.to_vec()).collect();

    let cabecalho_idx = linhas
        .iter()
        .position(|l| {
            l.first()
                .is_some_and(|c| c.to_string().contains("cod_IBGE"))
        })
        .ok_or_else(|| {
            ErroDeRefino::Decodificacao(format!("cabeçalho cod_IBGE ausente em {nome_arquivo}"))
        })?;

    let colunas: Vec<String> = linhas[cabecalho_idx]
        .iter()
        .map(|c| celula_para_texto(c).unwrap_or_default())
        .collect();

    if colunas.is_empty() {
        return Ok(None);
    }

    let nome_parte = nome_de_parte(nome_arquivo);
    let colunas_com_planilha: Vec<String> = colunas
        .iter()
        .cloned()
        .chain(std::iter::once("_planilha".to_owned()))
        .collect();

    let mut tabela = Tabela::nova(colunas_com_planilha);

    for linha in &linhas[cabecalho_idx + 1..] {
        if linha.is_empty() {
            continue;
        }
        if linha
            .first()
            .is_some_and(|c| matches!(c, Data::Empty) || c.to_string().trim().is_empty())
        {
            continue;
        }

        let mut valores: Vec<Option<String>> = colunas
            .iter()
            .enumerate()
            .map(|(i, _)| linha.get(i).and_then(celula_para_texto))
            .collect();
        valores.push(Some(nome_parte.clone()));
        tabela = tabela.com_linha(valores);
    }

    Ok(Some(ParteBronze::de_tabela(&nome_parte, &tabela)?))
}

fn celula_para_texto(celula: &Data) -> Option<String> {
    match celula {
        Data::Empty => None,
        Data::Float(f) => Some(f.to_string()),
        Data::Int(i) => Some(i.to_string()),
        Data::Bool(b) => Some(b.to_string()),
        Data::String(s) if s.is_empty() => None,
        _ => Some(celula.to_string()),
    }
}

fn nome_de_parte(caminho_no_zip: &str) -> String {
    let nome = caminho_no_zip.rsplit('/').next().unwrap_or(caminho_no_zip);
    nome.strip_suffix(".xlsx")
        .or_else(|| nome.strip_suffix(".XLSX"))
        .unwrap_or(nome)
        .to_owned()
}

#[cfg(test)]
mod testes {
    use std::io::{Cursor, Write};

    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::*;
    use crate::csv_zip::testes::ler_parte;

    fn xlsx_sinisa(municipios: &[(&str, &str, &str)]) -> Vec<u8> {
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
                    r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Indicadores" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
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

            // Preâmbulo (linhas 1-10)
            for r in 1..=10 {
                xml.push_str(&format!(
                    r#"<row r="{r}"><c r="A{r}" t="inlineStr"><is><t>preambulo</t></is></c></row>"#
                ));
            }

            // Linha 11: cabeçalho com códigos máquina
            xml.push_str(r#"<row r="11">"#);
            xml.push_str(r#"<c r="A11" t="inlineStr"><is><t>cod_IBGE</t></is></c>"#);
            xml.push_str(r#"<c r="B11" t="inlineStr"><is><t>Município</t></is></c>"#);
            xml.push_str(r#"<c r="C11" t="inlineStr"><is><t>IAG0001</t></is></c>"#);
            xml.push_str("</row>");

            // Dados
            for (i, (cod, mun, ind)) in municipios.iter().enumerate() {
                let r = 12 + i;
                xml.push_str(&format!(r#"<row r="{r}">"#));
                xml.push_str(&format!(
                    r#"<c r="A{r}" t="inlineStr"><is><t>{cod}</t></is></c>"#
                ));
                xml.push_str(&format!(
                    r#"<c r="B{r}" t="inlineStr"><is><t>{mun}</t></is></c>"#
                ));
                xml.push_str(&format!(r#"<c r="C{r}"><v>{ind}</v></c>"#));
                xml.push_str("</row>");
            }

            xml.push_str("</sheetData></worksheet>");
            zip.write_all(xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        bytes.into_inner()
    }

    fn zip_com_xlsx(nome_no_zip: &str, municipios: &[(&str, &str, &str)]) -> Vec<u8> {
        let xlsx_bytes = xlsx_sinisa(municipios);
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            zip.start_file(nome_no_zip, SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&xlsx_bytes).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn decodifica_xlsx_base_municipal_dentro_de_zip() {
        let zip_bytes = zip_com_xlsx(
            "Água - Base Municipal/indicadores.xlsx",
            &[
                ("1100015", "Alta Floresta D Oeste", "60.35"),
                ("1100023", "Ariquemes", "95.12"),
            ],
        );

        let partes = DecodificadorSnis
            .decodificar(&zip_bytes, "application/zip")
            .unwrap();

        assert_eq!(partes.len(), 1);
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(
            colunas,
            vec!["cod_IBGE", "Município", "IAG0001", "_planilha"]
        );
        assert_eq!(linhas.len(), 2);
        assert_eq!(linhas[0][0], Some("1100015".into()));
        assert_eq!(linhas[0][1], Some("Alta Floresta D Oeste".into()));
        assert_eq!(linhas[0][2], Some("60.35".into()));
        assert_eq!(linhas[0][3], Some("indicadores".into()));
    }

    #[test]
    fn ignora_subpastas_que_nao_sao_base_municipal() {
        let xlsx_bytes = xlsx_sinisa(&[("1100015", "Teste", "1.0")]);
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            let opcoes = SimpleFileOptions::default();

            zip.start_file("Consolidado UF/resumo.xlsx", opcoes)
                .unwrap();
            zip.write_all(&xlsx_bytes).unwrap();

            zip.start_file("Base Municipal/dados.xlsx", opcoes).unwrap();
            zip.write_all(&xlsx_bytes).unwrap();

            zip.finish().unwrap();
        }

        let partes = DecodificadorSnis
            .decodificar(&buf.into_inner(), "application/zip")
            .unwrap();

        assert_eq!(partes.len(), 1);
        let (_, linhas) = ler_parte(&partes[0]);
        assert_eq!(linhas.len(), 1);
    }

    #[test]
    fn zip_sem_base_municipal_retorna_vazio() {
        let xlsx_bytes = xlsx_sinisa(&[("1100015", "Teste", "1.0")]);
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            zip.start_file("Consolidado/resumo.xlsx", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&xlsx_bytes).unwrap();
            zip.finish().unwrap();
        }

        let partes = DecodificadorSnis
            .decodificar(&buf.into_inner(), "application/zip")
            .unwrap();
        assert!(partes.is_empty());
    }

    #[test]
    fn pula_linhas_vazias() {
        let zip_bytes = zip_com_xlsx(
            "Água - Base Municipal/ind.xlsx",
            &[("1100015", "Cidade", "42.0")],
        );
        let partes = DecodificadorSnis
            .decodificar(&zip_bytes, "application/zip")
            .unwrap();
        let (_, linhas) = ler_parte(&partes[0]);
        assert_eq!(linhas.len(), 1);
    }

    #[test]
    fn eh_base_municipal_case_insensitive() {
        assert!(super::eh_base_municipal("Água - Base Municipal/ind.xlsx"));
        assert!(super::eh_base_municipal("base municipal/ind.xlsx"));
        assert!(super::eh_base_municipal("ÁGUA - BASE MUNICIPAL/IND.XLSX"));
        assert!(!super::eh_base_municipal("Consolidado/ind.xlsx"));
        assert!(!super::eh_base_municipal("Base Municipal/glossario.pdf"));
    }
}
