//! Decodificador Bronze para o CNPJ da Receita Federal (dados abertos).
//!
//! Converte cada ZIP (um CSV sem cabeçalho, `;`, Latin-1) em Parquet com colunas nomeadas
//! conforme o layout oficial (cnpj-metadados.pdf).

use std::io::{Cursor, Read, Seek};

use crate::csv_zip::{leitor_auto_encoding, parte_de_csv_com_colunas};
use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::ErroDeRefino;

const COLUNAS_EMPRESAS: &[&str] = &[
    "cnpj_basico",
    "razao_social",
    "natureza_juridica",
    "qualificacao_responsavel",
    "capital_social",
    "porte_empresa",
    "ente_federativo",
];

const COLUNAS_ESTABELECIMENTOS: &[&str] = &[
    "cnpj_basico",
    "cnpj_ordem",
    "cnpj_dv",
    "matriz_filial",
    "nome_fantasia",
    "situacao_cadastral",
    "data_situacao",
    "motivo_situacao",
    "cidade_exterior",
    "pais",
    "data_inicio",
    "cnae_principal",
    "cnae_secundaria",
    "tipo_logradouro",
    "logradouro",
    "numero",
    "complemento",
    "bairro",
    "cep",
    "uf",
    "municipio",
    "ddd1",
    "telefone1",
    "ddd2",
    "telefone2",
    "ddd_fax",
    "fax",
    "email",
    "situacao_especial",
    "data_situacao_especial",
];

const COLUNAS_SOCIOS: &[&str] = &[
    "cnpj_basico",
    "identificador_socio",
    "nome_socio",
    "cnpj_cpf_socio",
    "qualificacao",
    "data_entrada",
    "pais",
    "representante_legal",
    "nome_representante",
    "qualificacao_representante",
    "faixa_etaria",
];

const COLUNAS_SIMPLES: &[&str] = &[
    "cnpj_basico",
    "opcao_simples",
    "data_opcao_simples",
    "data_exclusao_simples",
    "opcao_mei",
    "data_opcao_mei",
    "data_exclusao_mei",
];

const COLUNAS_AUXILIAR: &[&str] = &["codigo", "descricao"];

fn colunas_da_tabela(nome_interno: &str) -> Result<&'static [&'static str], ErroDeRefino> {
    let base = nome_interno
        .rsplit('/')
        .next()
        .unwrap_or(nome_interno)
        .split('.')
        .next()
        .unwrap_or(nome_interno);
    let prefixo = base.to_ascii_uppercase();
    if prefixo.starts_with("EMPRESA") && !prefixo.starts_with("ESTABELECIMENTO") {
        Ok(COLUNAS_EMPRESAS)
    } else if prefixo.starts_with("ESTABELECIMENTO") {
        Ok(COLUNAS_ESTABELECIMENTOS)
    } else if prefixo.starts_with("SOCIO") {
        Ok(COLUNAS_SOCIOS)
    } else if prefixo.starts_with("SIMPLES") {
        Ok(COLUNAS_SIMPLES)
    } else if prefixo.starts_with("CNAE")
        || prefixo.starts_with("MUNICIPIO")
        || prefixo.starts_with("NATUREZA")
        || prefixo.starts_with("MOTIVO")
        || prefixo.starts_with("QUALIFICACO") // Qualificacoes → QUALIFICACO...
        || prefixo.starts_with("PAIS")
    {
        Ok(COLUNAS_AUXILIAR)
    } else {
        Err(ErroDeRefino::Decodificacao(format!(
            "tabela CNPJ desconhecida: {nome_interno}"
        )))
    }
}

fn nome_logico(nome_interno: &str) -> String {
    let base = nome_interno.rsplit('/').next().unwrap_or(nome_interno);
    match base.rfind('.') {
        Some(pos) => base[..pos].to_owned(),
        None => base.to_owned(),
    }
}

fn decodificar_zip<F: Read + Seek>(fonte: F) -> Result<Vec<ParteBronze>, ErroDeRefino> {
    let mut arquivo = zip::ZipArchive::new(fonte)?;
    let mut partes = Vec::new();

    for indice in 0..arquivo.len() {
        let entrada = arquivo.by_index(indice)?;
        if !entrada.is_file() {
            continue;
        }
        let nome_interno = entrada.name()?.into_owned();
        let base_lower = nome_interno.to_ascii_lowercase();
        if !(base_lower.ends_with(".csv") || base_lower.ends_with(".txt")) {
            continue;
        }
        let colunas = colunas_da_tabela(&nome_interno)?;
        let leitor = leitor_auto_encoding(entrada);
        let nome = nome_logico(&nome_interno);
        partes.push(parte_de_csv_com_colunas(&nome, leitor, b';', colunas)?);
    }

    if partes.is_empty() {
        return Err(ErroDeRefino::Decodificacao(
            "nenhum CSV encontrado no ZIP do CNPJ".to_owned(),
        ));
    }
    Ok(partes)
}

/// Decodifica ZIPs de CSVs do CNPJ (Receita Federal) em Parquet com colunas nomeadas.
///
/// Cada ZIP contém um CSV sem cabeçalho, com separador `;` e encoding Latin-1/Windows-1252.
/// O nome do CSV interno determina o esquema de colunas (layout do PDF da Receita).
pub struct DecodificadorCnpj;

impl Decodificador for DecodificadorCnpj {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        decodificar_zip(Cursor::new(bytes))
    }

    fn decodificar_arquivo(
        &self,
        caminho: &std::path::Path,
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        decodificar_zip(std::fs::File::open(caminho)?)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::csv_zip::testes::ler_parte;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn zip_com_csv(nome_csv: &str, conteudo: &[u8]) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut buffer));
            zw.start_file(nome_csv, SimpleFileOptions::default())
                .unwrap();
            zw.write_all(conteudo).unwrap();
            zw.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn decodifica_empresas_com_colunas_nomeadas() {
        let csv = b"12345678;Empresa Teste;2062;50;1000.00;05;";
        let zip = zip_com_csv("Empresas0.csv", csv);
        let partes = DecodificadorCnpj
            .decodificar(&zip, "application/zip")
            .unwrap();
        assert_eq!(partes.len(), 1);
        assert_eq!(partes[0].nome, "Empresas0");
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, COLUNAS_EMPRESAS);
        assert_eq!(linhas.len(), 1);
        assert_eq!(linhas[0][0], Some("12345678".to_owned()));
        assert_eq!(linhas[0][1], Some("Empresa Teste".to_owned()));
    }

    #[test]
    fn decodifica_estabelecimentos_30_colunas() {
        let mut linha = "12345678;0001;00;1;Fantasia;02;20200101;01;;999;20100101;4711301;4711302;RUA;Centro;100;SL 1;Bairro;01001000;SP;7107;11;12345678;11;87654321;11;99999999;email@teste.com;;".to_string();
        linha.push('\n');
        let zip = zip_com_csv("Estabelecimentos5.csv", linha.as_bytes());
        let partes = DecodificadorCnpj
            .decodificar(&zip, "application/zip")
            .unwrap();
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas.len(), 30);
        assert_eq!(colunas, COLUNAS_ESTABELECIMENTOS);
        assert_eq!(linhas.len(), 1);
    }

    #[test]
    fn decodifica_socios() {
        let csv = b"12345678;2;Fulano;00000000000;49;20180315;106;;;00;3";
        let zip = zip_com_csv("Socios7.csv", csv);
        let partes = DecodificadorCnpj
            .decodificar(&zip, "application/zip")
            .unwrap();
        let (colunas, _) = ler_parte(&partes[0]);
        assert_eq!(colunas, COLUNAS_SOCIOS);
    }

    #[test]
    fn decodifica_simples() {
        let csv = b"12345678;S;20180101;;N;;";
        let zip = zip_com_csv("Simples.csv", csv);
        let partes = DecodificadorCnpj
            .decodificar(&zip, "application/zip")
            .unwrap();
        let (colunas, _) = ler_parte(&partes[0]);
        assert_eq!(colunas, COLUNAS_SIMPLES);
    }

    #[test]
    fn decodifica_tabelas_auxiliares() {
        for nome in [
            "Cnaes.csv",
            "Municipios.csv",
            "Naturezas.csv",
            "Motivos.csv",
            "Qualificacoes.csv",
            "Paises.csv",
        ] {
            let csv = b"001;Descricao teste";
            let zip = zip_com_csv(nome, csv);
            let partes = DecodificadorCnpj
                .decodificar(&zip, "application/zip")
                .unwrap();
            let (colunas, _) = ler_parte(&partes[0]);
            assert_eq!(colunas, COLUNAS_AUXILIAR, "falhou para {nome}");
        }
    }

    #[test]
    fn converte_latin1_para_utf8() {
        // "São Paulo" em Latin-1: S=53, ã=E3, o=6F, ' '=20, P=50, ...
        let csv: &[u8] = &[
            b'1', b'2', b';', 0xC7, 0xE3, 0x6F, b' ', b'P', b'a', b'u', b'l', b'o',
        ];
        let zip = zip_com_csv("Paises.csv", csv);
        let partes = DecodificadorCnpj
            .decodificar(&zip, "application/zip")
            .unwrap();
        let (_, linhas) = ler_parte(&partes[0]);
        assert_eq!(linhas[0][1], Some("Ção Paulo".to_owned()));
    }

    #[test]
    fn erro_para_tabela_desconhecida() {
        let zip = zip_com_csv("Desconhecido.csv", b"1;2;3");
        let resultado = DecodificadorCnpj.decodificar(&zip, "application/zip");
        assert!(resultado.is_err());
        let msg = resultado.unwrap_err().to_string();
        assert!(msg.contains("desconhecida"), "{msg}");
    }

    #[test]
    fn mapeia_prefixos_corretamente() {
        assert_eq!(colunas_da_tabela("Empresas0.csv").unwrap().len(), 7);
        assert_eq!(
            colunas_da_tabela("Estabelecimentos9.csv").unwrap().len(),
            30
        );
        assert_eq!(colunas_da_tabela("Socios3.csv").unwrap().len(), 11);
        assert_eq!(colunas_da_tabela("Simples.csv").unwrap().len(), 7);
        assert_eq!(colunas_da_tabela("Cnaes.csv").unwrap().len(), 2);
    }
}
