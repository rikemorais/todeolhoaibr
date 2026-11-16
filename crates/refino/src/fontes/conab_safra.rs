//! Extrai o resumo nacional por produto do XLSX de safra da CONAB.

use std::io::Cursor;

use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, Tabela};

const ABA: &str = "Brasil - Total por Produto";
const COLUNAS: [&str; 5] = [
    "produto",
    "safra",
    "area_mil_ha",
    "produtividade_kg_ha",
    "producao_mil_t",
];

/// Decodifica o XLSX de safra de grãos da CONAB (aba "Brasil - Total por Produto").
///
/// Cada linha da aba contém um produto com 3 métricas (área, produtividade, produção)
/// para duas safras. O decodificador emite duas linhas Bronze por produto (uma por safra).
pub struct DecodificadorConabSafra;

impl Decodificador for DecodificadorConabSafra {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let mut workbook: Xlsx<_> = open_workbook_from_rs(Cursor::new(bytes)).map_err(|erro| {
            ErroDeRefino::Decodificacao(format!("XLSX inválido da CONAB: {erro}"))
        })?;
        let planilha = workbook.worksheet_range(ABA).map_err(|erro| {
            ErroDeRefino::Decodificacao(format!("aba '{ABA}' ausente ou inválida: {erro}"))
        })?;

        let linhas: Vec<Vec<Data>> = planilha.rows().map(|r| r.to_vec()).collect();

        let (safra_ant, safra_atual) = extrair_safras(&linhas)?;
        let cabecalho = encontrar_cabecalho(&linhas)?;
        let inicio_dados = cabecalho + 1;

        let mut tabela = Tabela::nova(COLUNAS);

        for linha in &linhas[inicio_dados..] {
            if linha.len() < 10 {
                continue;
            }
            let Some(produto) = texto(&linha[0]) else {
                continue;
            };
            let produto = produto.trim().to_owned();
            if produto.is_empty() || eh_subtotal(&produto) {
                continue;
            }

            let area_ant = texto_numerico(&linha[1]);
            let area_atual = texto_numerico(&linha[2]);
            let prod_ant = texto_numerico(&linha[4]);
            let prod_atual = texto_numerico(&linha[5]);
            let producao_ant = texto_numerico(&linha[7]);
            let producao_atual = texto_numerico(&linha[8]);

            tabela = tabela.com_linha([
                Some(produto.clone()),
                Some(safra_ant.clone()),
                area_ant.clone(),
                prod_ant,
                producao_ant,
            ]);
            tabela = tabela.com_linha([
                Some(produto),
                Some(safra_atual.clone()),
                area_atual,
                prod_atual,
                producao_atual,
            ]);
        }

        Ok(vec![ParteBronze::de_tabela("produtos", &tabela)?])
    }
}

fn extrair_safras(linhas: &[Vec<Data>]) -> Result<(String, String), ErroDeRefino> {
    for linha in linhas.iter().take(10) {
        for celula in linha {
            if let Some(t) = texto(celula)
                && t.starts_with("Safra ")
                && t.len() >= 10
            {
                let safra = t.strip_prefix("Safra ").unwrap().to_owned();
                for outra in linha {
                    if let Some(t2) = texto(outra)
                        && t2.starts_with("Safra ")
                        && t2 != t
                    {
                        let s2 = t2.strip_prefix("Safra ").unwrap().to_owned();
                        return Ok((safra, s2));
                    }
                }
            }
        }
    }
    Err(ErroDeRefino::Decodificacao(
        "não encontrou os rótulos de safra no cabeçalho da CONAB".into(),
    ))
}

fn encontrar_cabecalho(linhas: &[Vec<Data>]) -> Result<usize, ErroDeRefino> {
    for (i, linha) in linhas.iter().enumerate().take(15) {
        for celula in linha {
            if let Some(t) = texto(celula)
                && (t.contains("REGIÃO/UF") || t.contains("PRODUÇÃO"))
            {
                return Ok(i);
            }
        }
    }
    Err(ErroDeRefino::Decodificacao(
        "não encontrou a linha de cabeçalho (REGIÃO/UF) na aba da CONAB".into(),
    ))
}

fn eh_subtotal(produto: &str) -> bool {
    let p = produto.to_uppercase();
    p.starts_with("NORTE")
        || p.starts_with("NORDESTE")
        || p.starts_with("CENTRO")
        || p.starts_with("SUDESTE")
        || p.starts_with("SUL ")
        || p.starts_with("BRASIL")
        || p.starts_with("TOTAL")
        || p.starts_with("GRÃOS")
        || p.starts_with("GRAOS")
        || p.starts_with("Fonte:")
}

fn texto(celula: &Data) -> Option<String> {
    (!matches!(celula, Data::Empty)).then(|| celula.to_string())
}

fn texto_numerico(celula: &Data) -> Option<String> {
    match celula {
        Data::Empty => None,
        Data::Float(f) => Some(f.to_string()),
        Data::Int(i) => Some(i.to_string()),
        outro => texto(outro),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::io::{Cursor, Write};

    use zip::{ZipWriter, write::SimpleFileOptions};

    use crate::csv_zip::testes::ler_parte;

    fn xlsx_safra(produtos: &[&str]) -> Vec<u8> {
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
                    r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Brasil - Total por Produto" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
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

            // Row 6: safra labels
            xml.push_str(r#"<row r="6">"#);
            xml.push_str(r#"<c r="A6" t="inlineStr"><is><t></t></is></c>"#);
            xml.push_str(r#"<c r="B6" t="inlineStr"><is><t>Safra 24/25</t></is></c>"#);
            xml.push_str(r#"<c r="C6" t="inlineStr"><is><t>Safra 25/26</t></is></c>"#);
            xml.push_str(r#"<c r="D6" t="inlineStr"><is><t></t></is></c>"#);
            xml.push_str(r#"<c r="E6" t="inlineStr"><is><t>Safra 24/25</t></is></c>"#);
            xml.push_str(r#"<c r="F6" t="inlineStr"><is><t>Safra 25/26</t></is></c>"#);
            xml.push_str(r#"<c r="G6" t="inlineStr"><is><t></t></is></c>"#);
            xml.push_str(r#"<c r="H6" t="inlineStr"><is><t>Safra 24/25</t></is></c>"#);
            xml.push_str(r#"<c r="I6" t="inlineStr"><is><t>Safra 25/26</t></is></c>"#);
            xml.push_str(r#"<c r="J6" t="inlineStr"><is><t></t></is></c>"#);
            xml.push_str("</row>");

            // Row 7: header
            xml.push_str(r#"<row r="7">"#);
            xml.push_str(r#"<c r="A7" t="inlineStr"><is><t></t></is></c>"#);
            xml.push_str(r#"<c r="B7" t="inlineStr"><is><t></t></is></c>"#);
            xml.push_str(r#"<c r="C7" t="inlineStr"><is><t></t></is></c>"#);
            xml.push_str(r#"<c r="D7" t="inlineStr"><is><t></t></is></c>"#);
            xml.push_str(r#"<c r="E7" t="inlineStr"><is><t></t></is></c>"#);
            xml.push_str(r#"<c r="F7" t="inlineStr"><is><t></t></is></c>"#);
            xml.push_str(r#"<c r="G7" t="inlineStr"><is><t>REGIÃO/UF</t></is></c>"#);
            xml.push_str(r#"<c r="H7" t="inlineStr"><is><t>ÁREA (Em mil ha)</t></is></c>"#);
            xml.push_str(r#"<c r="I7" t="inlineStr"><is><t>PRODUTIVIDADE (Em kg/ha)</t></is></c>"#);
            xml.push_str(r#"<c r="J7" t="inlineStr"><is><t>PRODUÇÃO (Em mil t)</t></is></c>"#);
            xml.push_str("</row>");

            // Data rows
            for (i, produto) in produtos.iter().enumerate() {
                let r = i + 8;
                xml.push_str(&format!(r#"<row r="{r}">"#));
                xml.push_str(&format!(
                    r#"<c r="A{r}" t="inlineStr"><is><t>{produto}</t></is></c>"#
                ));
                // area ant, area atual, var%
                xml.push_str(&format!(r#"<c r="B{r}"><v>100.5</v></c>"#));
                xml.push_str(&format!(r#"<c r="C{r}"><v>110.2</v></c>"#));
                xml.push_str(&format!(r#"<c r="D{r}"><v>9.7</v></c>"#));
                // produtividade ant, produtividade atual, var%
                xml.push_str(&format!(r#"<c r="E{r}"><v>3500</v></c>"#));
                xml.push_str(&format!(r#"<c r="F{r}"><v>3600</v></c>"#));
                xml.push_str(&format!(r#"<c r="G{r}"><v>2.9</v></c>"#));
                // producao ant, producao atual, var%
                xml.push_str(&format!(r#"<c r="H{r}"><v>351.75</v></c>"#));
                xml.push_str(&format!(r#"<c r="I{r}"><v>396.72</v></c>"#));
                xml.push_str(&format!(r#"<c r="J{r}"><v>12.8</v></c>"#));
                xml.push_str("</row>");
            }

            xml.push_str("</sheetData></worksheet>");
            zip.write_all(xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        bytes.into_inner()
    }

    #[test]
    fn extrai_produtos_em_duas_safras() {
        let bytes = xlsx_safra(&["SOJA", "MILHO TOTAL"]);
        let partes = DecodificadorConabSafra.decodificar(&bytes, "").unwrap();
        let (colunas, linhas) = ler_parte(&partes[0]);

        assert_eq!(colunas, COLUNAS);
        assert_eq!(linhas.len(), 4);
        assert_eq!(linhas[0][0], Some("SOJA".into()));
        assert_eq!(linhas[0][1], Some("24/25".into()));
        assert_eq!(linhas[1][0], Some("SOJA".into()));
        assert_eq!(linhas[1][1], Some("25/26".into()));
        assert_eq!(linhas[2][0], Some("MILHO TOTAL".into()));
    }

    #[test]
    fn filtra_subtotais() {
        let bytes = xlsx_safra(&["SOJA", "NORTE", "BRASIL", "MILHO TOTAL"]);
        let partes = DecodificadorConabSafra.decodificar(&bytes, "").unwrap();
        let (_, linhas) = ler_parte(&partes[0]);

        let produtos: Vec<_> = linhas.iter().filter_map(|l| l[0].as_deref()).collect();
        assert!(!produtos.contains(&"NORTE"));
        assert!(!produtos.contains(&"BRASIL"));
        assert!(produtos.contains(&"SOJA"));
        assert!(produtos.contains(&"MILHO TOTAL"));
    }
}
