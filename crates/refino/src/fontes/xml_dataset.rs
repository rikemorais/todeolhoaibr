//! Decodificador do bronze para fontes que gravam um **XML DataSet .NET** no raw.
//!
//! Cobre a ANA/SNIRH, cuja API ServiceANA retorna XML com `<Table>` como registros
//! e elementos filhos como colunas. Cada campo vira texto; o bronze mantém a
//! fidelidade e a interpretação é do silver.

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, Tabela};

/// Decodifica XML DataSet .NET (ex.: ANA/SNIRH) com `<Table>` como registros.
pub struct DecodificadorXmlDataset {
    nome: &'static str,
}

impl DecodificadorXmlDataset {
    /// Cria um decodificador de XML DataSet que grava o Parquet com o nome dado.
    pub const fn novo(nome: &'static str) -> Self {
        Self { nome }
    }
}

impl Decodificador for DecodificadorXmlDataset {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let mut reader = Reader::from_reader(bytes);
        reader.config_mut().trim_text(true);

        let mut colunas: Vec<String> = Vec::new();
        let mut linhas: Vec<Vec<Option<String>>> = Vec::new();
        let mut dentro_de_table = false;
        let mut coluna_atual: Option<String> = None;
        let mut texto_atual = String::new();
        let mut linha_atual: Vec<(String, String)> = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let nome_local = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                    if nome_local == "Table" {
                        dentro_de_table = true;
                        linha_atual.clear();
                    } else if dentro_de_table {
                        coluna_atual = Some(nome_local);
                        texto_atual.clear();
                    }
                }
                Ok(Event::Text(e)) => {
                    if coluna_atual.is_some()
                        && let Ok(t) = e.decode()
                    {
                        texto_atual.push_str(&t);
                    }
                }
                Ok(Event::End(e)) => {
                    let nome_local = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                    if nome_local == "Table" && dentro_de_table {
                        dentro_de_table = false;
                        for (col, _) in &linha_atual {
                            if !colunas.iter().any(|c| c == col) {
                                colunas.push(col.clone());
                            }
                        }
                        let linha: Vec<Option<String>> = colunas
                            .iter()
                            .map(|col| {
                                linha_atual
                                    .iter()
                                    .find(|(c, _)| c == col)
                                    .map(|(_, v)| v.clone())
                                    .filter(|v| !v.is_empty())
                            })
                            .collect();
                        linhas.push(linha);
                    } else if coluna_atual.as_deref() == Some(&nome_local) {
                        linha_atual.push((nome_local, texto_atual.clone()));
                        coluna_atual = None;
                    }
                }
                Ok(Event::Empty(e)) => {
                    let nome_local = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                    if dentro_de_table {
                        linha_atual.push((nome_local, String::new()));
                    }
                }
                Ok(Event::Eof) => break,
                Err(erro) => {
                    return Err(ErroDeRefino::Decodificacao(format!("XML inválido: {erro}")));
                }
                _ => {}
            }
            buf.clear();
        }

        if colunas.is_empty() {
            return Ok(Vec::new());
        }

        let mut tabela = Tabela::nova(colunas.clone());
        for linha in &linhas {
            let mut completa = linha.clone();
            completa.resize(colunas.len(), None);
            tabela = tabela.com_linha(completa);
        }

        Ok(vec![ParteBronze::de_tabela(self.nome, &tabela)?])
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::csv_zip::testes::ler_parte;

    fn c(s: &str) -> Option<String> {
        Some(s.to_owned())
    }

    #[test]
    fn decodifica_xml_dataset_da_ana() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<DataSet xmlns="http://MRCS/">
  <xs:schema id="Estacoes" xmlns="" xmlns:xs="http://www.w3.org/2001/XMLSchema">
    <xs:element name="Estacoes" />
  </xs:schema>
  <diffgr:diffgram xmlns:msdata="urn:schemas-microsoft-com:xml-msdata" xmlns:diffgr="urn:schemas-microsoft-com:xml-diffgram-v1">
    <Estacoes xmlns="">
      <Table>
        <BaciaCodigo>1</BaciaCodigo>
        <RioNome>RIO AMAZONAS</RioNome>
        <Latitude>-3.13</Latitude>
      </Table>
      <Table>
        <BaciaCodigo>2</BaciaCodigo>
        <RioNome>RIO TOCANTINS</RioNome>
        <Latitude>-10.5</Latitude>
      </Table>
    </Estacoes>
  </diffgr:diffgram>
</DataSet>"#;
        let partes = DecodificadorXmlDataset::novo("estacoes")
            .decodificar(xml, "text/xml")
            .unwrap();
        assert_eq!(partes.len(), 1);
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, vec!["BaciaCodigo", "RioNome", "Latitude"]);
        assert_eq!(
            linhas,
            vec![
                vec![c("1"), c("RIO AMAZONAS"), c("-3.13")],
                vec![c("2"), c("RIO TOCANTINS"), c("-10.5")],
            ]
        );
    }

    #[test]
    fn elementos_vazios_viram_nulo() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<DataSet xmlns="http://MRCS/">
  <diffgr:diffgram xmlns:diffgr="urn:schemas-microsoft-com:xml-diffgram-v1">
    <Estacoes xmlns="">
      <Table>
        <Codigo>1</Codigo>
        <Descricao />
      </Table>
    </Estacoes>
  </diffgr:diffgram>
</DataSet>"#;
        let partes = DecodificadorXmlDataset::novo("estacoes")
            .decodificar(xml, "text/xml")
            .unwrap();
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, vec!["Codigo", "Descricao"]);
        assert_eq!(linhas, vec![vec![c("1"), None]]);
    }

    #[test]
    fn xml_sem_registros_nao_gera_parquet() {
        let xml = br#"<?xml version="1.0" encoding="utf-8"?>
<DataSet xmlns="http://MRCS/">
  <diffgr:diffgram xmlns:diffgr="urn:schemas-microsoft-com:xml-diffgram-v1">
    <Estacoes xmlns="" />
  </diffgr:diffgram>
</DataSet>"#;
        let partes = DecodificadorXmlDataset::novo("estacoes")
            .decodificar(xml, "text/xml")
            .unwrap();
        assert!(partes.is_empty());
    }
}
