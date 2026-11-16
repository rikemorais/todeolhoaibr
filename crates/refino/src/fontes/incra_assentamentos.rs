//! Decodificador do CSV tabular de assentamentos do INCRA (SIPRA).
//!
//! O CSV publicado em gov.br tem cabeçalho ministerial (3 primeiras linhas),
//! separadores de Superintendência Regional intercalados, e uma linha de
//! totalizador no final. Este decodificador filtra essas linhas e emite
//! apenas os registros de projeto em Parquet, com colunas nomeadas.

use crate::csv_zip::partes_de_csv;
use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::ErroDeRefino;

const CABECALHO: &str = "codigo_projeto;nome_projeto;municipio_sede;area_ha;familias_assentadas;familias_assent;fase;tipo;numero;data_criacao;forma_obtencao;data_obtencao";

const COLUNAS: usize = 12;

fn e_linha_de_dados(linha: &str) -> bool {
    if linha.is_empty() {
        return false;
    }
    let campos = linha.split(';').count();
    if campos < COLUNAS {
        return false;
    }
    let primeiro = linha.split(';').next().unwrap_or("");
    !primeiro.starts_with("MINIST")
        && !primeiro.starts_with("INSTITU")
        && !primeiro.starts_with("DIRETO")
        && !primeiro.starts_with("Superintend")
        && !primeiro.starts_with("Total Geral")
        && !primeiro.starts_with('\t')
}

fn limpar_csv(bytes: &[u8]) -> Vec<u8> {
    let texto = encoding_rs::WINDOWS_1252.decode(bytes).0;
    let mut saida = CABECALHO.to_owned();
    saida.push('\n');

    for linha in texto.lines() {
        let linha = linha.trim_end();
        if e_linha_de_dados(linha) {
            let campos: Vec<&str> = linha.split(';').take(COLUNAS).collect();
            for (i, campo) in campos.iter().enumerate() {
                if i > 0 {
                    saida.push(';');
                }
                saida.push_str(campo.trim());
            }
            saida.push('\n');
        }
    }

    saida.into_bytes()
}

/// Decodifica o CSV de assentamentos do INCRA (SIPRA), filtrando linhas de
/// metadados e emitindo apenas os registros de projeto em Parquet.
pub struct DecodificadorAssentamentosIncra;

impl Decodificador for DecodificadorAssentamentosIncra {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let csv_limpo = limpar_csv(bytes);
        partes_de_csv(&csv_limpo, "assentamentos", b';', 0)
    }

    fn decodificar_arquivo(
        &self,
        caminho: &std::path::Path,
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let bytes = std::fs::read(caminho)?;
        self.decodificar(&bytes, "text/csv")
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::csv_zip::testes::ler_parte;

    fn c(s: &str) -> Option<String> {
        Some(s.to_owned())
    }

    fn csv_exemplo() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"MINISTERIO DO DESENVOLVIMENTO AGRARIO;;;;;;;;;Relatorio : 0227\n");
        v.extend_from_slice(b"INSTITUTO NACIONAL DE COLONIZACAO;;;;;;;;;Fonte: SIPRA\n");
        v.extend_from_slice(b"DIRETORIA DE OBTENCAO DE TERRAS;;;;;;;;;\n");
        v.extend_from_slice(b"Superintend\xeancia Regional: SR (01);;;;;\n");
        v.extend_from_slice(b"PA0006000;PA MIRITIPITANGA;TOME-ACU;4.356,0000;102;106;06;1;205;02/09/1992;Desapropria\xe7\xe3o;28/01/1987\n");
        v.extend_from_slice(b"PA0016000;PA MANOEL;AURORA DO PARA;18.894,8049;388;465;07;1;917;26/11/1986;Desapropria\xe7\xe3o;26/01/1987\n");
        v.extend_from_slice(b"Superintend\xeancia Regional: SR (02);;;;;\n");
        v.extend_from_slice(b"MA0001000;PA CENTRO HERDEIROS;SAO LUIS;5.153,0000;80;100;06;1;300;01/01/1988;Desapropria\xe7\xe3o;01/01/1986\n");
        v.extend_from_slice(b"\n");
        v.extend_from_slice(b"Total Geral:;3 Projetos;;28.403,8049;570;671;;;;;\n");
        v
    }

    #[test]
    fn filtra_cabecalho_e_separadores() {
        let partes = DecodificadorAssentamentosIncra
            .decodificar(&csv_exemplo(), "text/csv")
            .unwrap();
        assert_eq!(partes.len(), 1);
        assert_eq!(partes[0].nome, "assentamentos");

        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas[0], "codigo_projeto");
        assert_eq!(colunas[2], "municipio_sede");
        assert_eq!(linhas.len(), 3);
        assert_eq!(linhas[0][0], c("PA0006000"));
        assert_eq!(linhas[1][0], c("PA0016000"));
        assert_eq!(linhas[2][0], c("MA0001000"));
    }

    #[test]
    fn decodifica_acentos_windows1252() {
        let partes = DecodificadorAssentamentosIncra
            .decodificar(&csv_exemplo(), "text/csv")
            .unwrap();
        let (_, linhas) = ler_parte(&partes[0]);
        assert_eq!(linhas[0][10], c("Desapropriação"));
    }

    #[test]
    fn doze_colunas_nomeadas() {
        let partes = DecodificadorAssentamentosIncra
            .decodificar(&csv_exemplo(), "text/csv")
            .unwrap();
        let (colunas, _) = ler_parte(&partes[0]);
        assert_eq!(colunas.len(), COLUNAS);
        assert_eq!(colunas.last().unwrap(), "data_obtencao");
    }
}
