//! Extrai as estatísticas agregadas por UF da planilha mensal do Mapa de Empresas.

use std::io::Cursor;

use calamine::{Data, Reader, Xlsx, open_workbook_from_rs};

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, Tabela};

const ABA: &str = "Abertas, Fechadas e Ativas";
const COLUNAS: [&str; 8] = [
    "dt_referencia",
    "dt_geracao",
    "cod_ibge",
    "UF",
    "Estado",
    "Empresas Abertas",
    "Empresas Fechadas",
    "Empresas Ativas",
];
const UFS: [&str; 27] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "DF", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE",
    "PI", "PR", "RJ", "RN", "RO", "RR", "RS", "SC", "SE", "SP", "TO",
];

/// Decodifica o bloco agregado por UF do snapshot MEMP para uma tabela Bronze.
pub struct DecodificadorMapaEmpresas;

impl Decodificador for DecodificadorMapaEmpresas {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let mut workbook: Xlsx<_> = open_workbook_from_rs(Cursor::new(bytes)).map_err(|erro| {
            ErroDeRefino::Decodificacao(format!("XLSX inválido do Mapa de Empresas: {erro}"))
        })?;
        let planilha = workbook.worksheet_range(ABA).map_err(|erro| {
            ErroDeRefino::Decodificacao(format!("aba '{ABA}' ausente ou inválida: {erro}"))
        })?;
        let mut linhas = planilha.rows();
        let cabecalho = linhas.next().ok_or_else(|| {
            ErroDeRefino::Decodificacao("planilha do Mapa de Empresas sem cabeçalho".into())
        })?;
        let inicio = (0..cabecalho.len())
            .find(|inicio| {
                cabecalho
                    .get(*inicio..inicio.saturating_add(COLUNAS.len()))
                    .is_some_and(|celulas| {
                        celulas
                            .iter()
                            .map(ToString::to_string)
                            .eq(COLUNAS.iter().map(|coluna| (*coluna).to_owned()))
                    })
            })
            .ok_or_else(|| {
                ErroDeRefino::Decodificacao(format!(
                    "bloco de dados por UF não encontrado na aba '{ABA}'"
                ))
            })?;
        let coluna_uf = inicio + 3;
        let ufs_esperadas: std::collections::HashSet<String> =
            UFS.into_iter().map(str::to_owned).collect();
        let mut ufs_encontradas = std::collections::HashSet::new();
        let mut tabela = Tabela::nova(COLUNAS);

        for linha in linhas {
            let Some(uf) = linha.get(coluna_uf).and_then(texto) else {
                continue;
            };
            if !ufs_esperadas.contains(uf.as_str()) {
                continue;
            }
            if !ufs_encontradas.insert(uf.clone()) {
                return Err(ErroDeRefino::Decodificacao(format!(
                    "UF duplicada no bloco estadual: {uf}"
                )));
            }
            let valores = linha.get(inicio..inicio + COLUNAS.len()).ok_or_else(|| {
                ErroDeRefino::Decodificacao(format!("linha estadual incompleta para a UF {uf}"))
            })?;
            tabela = tabela.com_linha(valores.iter().map(texto));
        }

        if ufs_encontradas != ufs_esperadas {
            let ausentes = UFS
                .iter()
                .copied()
                .filter(|uf| !ufs_encontradas.contains(*uf))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(ErroDeRefino::Decodificacao(format!(
                "faltam UFs no bloco estadual do Mapa de Empresas: {ausentes}"
            )));
        }

        Ok(vec![ParteBronze::de_tabela("uf", &tabela)?])
    }
}

fn texto(celula: &Data) -> Option<String> {
    (!matches!(celula, Data::Empty)).then(|| celula.to_string())
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::io::Write;

    use zip::{ZipWriter, write::SimpleFileOptions};

    use crate::csv_zip::testes::ler_parte;

    fn xlsx_com_ufs(ufs: &[&str]) -> Vec<u8> {
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
                    r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Abertas, Fechadas e Ativas" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
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
                r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData><row r="1">"#,
            );
            for (indice, coluna) in COLUNAS.iter().enumerate() {
                let letra = char::from(b'I' + indice as u8);
                xml.push_str(&format!(
                    r#"<c r="{letra}1" t="inlineStr"><is><t>{coluna}</t></is></c>"#
                ));
            }
            xml.push_str("</row>");
            for (indice, uf) in ufs.iter().enumerate() {
                let numero_linha = indice + 2;
                let valores = ["46265", "46269", "12", uf, "Estado", "10", "3", "100"];
                xml.push_str(&format!(r#"<row r="{numero_linha}">"#));
                for (coluna, valor) in valores.iter().enumerate() {
                    let letra = char::from(b'I' + coluna as u8);
                    if coluna == 3 || coluna == 4 {
                        xml.push_str(&format!(
                            r#"<c r="{letra}{numero_linha}" t="inlineStr"><is><t>{valor}</t></is></c>"#
                        ));
                    } else {
                        xml.push_str(&format!(
                            r#"<c r="{letra}{numero_linha}"><v>{valor}</v></c>"#
                        ));
                    }
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
    fn extrai_as_27_ufs_e_preserva_as_metricas_estaduais() {
        let bytes = xlsx_com_ufs(&UFS);
        let partes = DecodificadorMapaEmpresas
            .decodificar(
                &bytes,
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            )
            .unwrap();
        let (colunas, linhas) = ler_parte(&partes[0]);

        assert_eq!(colunas, COLUNAS);
        assert_eq!(linhas.len(), 27);
        assert_eq!(
            linhas[0],
            vec![
                Some("46265".into()),
                Some("46269".into()),
                Some("12".into()),
                Some("AC".into()),
                Some("Estado".into()),
                Some("10".into()),
                Some("3".into()),
                Some("100".into()),
            ]
        );
    }

    #[test]
    fn rejeita_uf_faltante_e_duplicada() {
        let sem_to = UFS[..26].to_vec();
        assert!(
            DecodificadorMapaEmpresas
                .decodificar(&xlsx_com_ufs(&sem_to), "")
                .is_err()
        );

        let mut duplicada = UFS.to_vec();
        duplicada[26] = "AC";
        assert!(
            DecodificadorMapaEmpresas
                .decodificar(&xlsx_com_ufs(&duplicada), "")
                .is_err()
        );
    }
}
