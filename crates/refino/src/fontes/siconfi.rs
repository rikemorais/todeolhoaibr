//! Decodificador do bronze para `siconfi-dca`, `siconfi-rreo` e `siconfi-rgf`.
//!
//! O raw guarda **um JSON por entidade** (envelope ORDS com `items`), precedido
//! pela listagem de entes (parte 1). Este decodificador **pula a primeira parte**
//! e agrega os `items` de todas as partes seguintes em uma tabela única.

use serde_json::Value;

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, Tabela};

pub struct DecodificadorSiconfi {
    nome: &'static str,
}

impl DecodificadorSiconfi {
    pub const fn novo(nome: &'static str) -> Self {
        Self { nome }
    }
}

fn texto_de(valor: &Value) -> Option<String> {
    match valor {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        outro => Some(outro.to_string()),
    }
}

fn extrair_items(bytes: &[u8]) -> Result<Vec<Value>, ErroDeRefino> {
    let raiz: Value = serde_json::from_slice(bytes)
        .map_err(|erro| ErroDeRefino::Decodificacao(format!("JSON inválido: {erro}")))?;
    match raiz.get("items") {
        Some(Value::Array(itens)) => Ok(itens.clone()),
        _ => Ok(Vec::new()),
    }
}

impl Decodificador for DecodificadorSiconfi {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let registros = extrair_items(bytes)?;
        if registros.is_empty() {
            return Ok(Vec::new());
        }

        let mut colunas: Vec<String> = Vec::new();
        for registro in &registros {
            if let Value::Object(mapa) = registro {
                for chave in mapa.keys() {
                    if !colunas.iter().any(|c| c == chave) {
                        colunas.push(chave.clone());
                    }
                }
            }
        }

        if colunas.is_empty() {
            return Ok(Vec::new());
        }

        let mut tabela = Tabela::nova(colunas.clone());
        for registro in &registros {
            let objeto = registro.as_object();
            let linha: Vec<Option<String>> = colunas
                .iter()
                .map(|coluna| objeto.and_then(|mapa| mapa.get(coluna)).and_then(texto_de))
                .collect();
            tabela = tabela.com_linha(linha);
        }

        Ok(vec![ParteBronze::de_tabela(self.nome, &tabela)?])
    }

    fn decodificar_arquivos(
        &self,
        caminhos: &[std::path::PathBuf],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let mut todos_registros: Vec<Value> = Vec::new();

        for (i, caminho) in caminhos.iter().enumerate() {
            // Pula a primeira parte (listagem de entes).
            if i == 0 {
                continue;
            }
            let bytes = std::fs::read(caminho)?;
            todos_registros.extend(extrair_items(&bytes)?);
        }

        if todos_registros.is_empty() {
            return Ok(Vec::new());
        }

        let mut colunas: Vec<String> = Vec::new();
        for registro in &todos_registros {
            if let Value::Object(mapa) = registro {
                for chave in mapa.keys() {
                    if !colunas.iter().any(|c| c == chave) {
                        colunas.push(chave.clone());
                    }
                }
            }
        }

        if colunas.is_empty() {
            return Ok(Vec::new());
        }

        let mut tabela = Tabela::nova(colunas.clone());
        for registro in &todos_registros {
            let objeto = registro.as_object();
            let linha: Vec<Option<String>> = colunas
                .iter()
                .map(|coluna| objeto.and_then(|mapa| mapa.get(coluna)).and_then(texto_de))
                .collect();
            tabela = tabela.com_linha(linha);
        }

        Ok(vec![ParteBronze::de_tabela(self.nome, &tabela)?])
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::csv_zip::testes::ler_parte;

    #[test]
    fn decodifica_items_do_envelope_ords() {
        let json = br#"{"items":[{"an_exercicio":"2024","cod_ibge":"1700301","valor":"100.00"},{"an_exercicio":"2024","cod_ibge":"1700301","valor":"200.00"}],"hasMore":false,"limit":100,"offset":0,"count":2}"#;
        let partes = DecodificadorSiconfi::novo("dca")
            .decodificar(json, "application/json")
            .unwrap();
        assert_eq!(partes.len(), 1);
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, vec!["an_exercicio", "cod_ibge", "valor"]);
        assert_eq!(linhas.len(), 2);
    }

    #[test]
    fn pula_a_primeira_parte_na_agregacao() {
        let entes = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            entes.path(),
            br#"{"items":[{"cod_ibge":1700301,"uf":"TO"}]}"#,
        )
        .unwrap();

        let dca = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            dca.path(),
            br#"{"items":[{"an_exercicio":"2024","valor":"42"}]}"#,
        )
        .unwrap();

        let caminhos = vec![entes.path().to_path_buf(), dca.path().to_path_buf()];
        let partes = DecodificadorSiconfi::novo("dca")
            .decodificar_arquivos(&caminhos, "application/json")
            .unwrap();
        assert_eq!(partes.len(), 1);
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, vec!["an_exercicio", "valor"]);
        assert_eq!(linhas.len(), 1);
        assert_eq!(linhas[0][1], Some("42".into()));
    }

    #[test]
    fn items_vazio_nao_gera_parquet() {
        let json = br#"{"items":[],"hasMore":false}"#;
        let partes = DecodificadorSiconfi::novo("dca")
            .decodificar(json, "application/json")
            .unwrap();
        assert!(partes.is_empty());
    }
}
