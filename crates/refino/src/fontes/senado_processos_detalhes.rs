//! Decodificador do bronze para `senado-processos-detalhes`.
//!
//! O raw guarda **um arquivo JSON por processo** (um objeto, não um array), aos milhares por
//! ano. Este decodificador **agrega** todos os processos de um recurso em uma única tabela —
//! uma linha por processo —, em vez de gerar um Parquet por arquivo. Campos viram colunas de
//! texto; valores aninhados (ex.: `conteudo`) viram JSON compacto.

use serde_json::{Map, Value};

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, Tabela};

/// Agrega os processos (um objeto JSON por arquivo) em uma tabela por recurso.
pub struct DecodificadorSenadoProcessosDetalhes;

fn texto_de(valor: &Value) -> Option<String> {
    match valor {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        outro => Some(outro.to_string()),
    }
}

/// Monta a tabela a partir dos objetos: colunas = união das chaves (ordem de aparição).
fn tabela_de(objetos: &[Map<String, Value>]) -> Result<ParteBronze, ErroDeRefino> {
    let mut colunas: Vec<String> = Vec::new();
    for objeto in objetos {
        for chave in objeto.keys() {
            if !colunas.iter().any(|c| c == chave) {
                colunas.push(chave.clone());
            }
        }
    }
    let mut tabela = Tabela::nova(colunas.clone());
    for objeto in objetos {
        let linha: Vec<Option<String>> = colunas
            .iter()
            .map(|coluna| objeto.get(coluna).and_then(texto_de))
            .collect();
        tabela = tabela.com_linha(linha);
    }
    ParteBronze::de_tabela("processos-detalhes", &tabela)
}

impl Decodificador for DecodificadorSenadoProcessosDetalhes {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        // Um único processo (objeto) vira uma tabela de uma linha.
        let valor: Value = serde_json::from_slice(bytes)
            .map_err(|erro| ErroDeRefino::Decodificacao(format!("JSON inválido: {erro}")))?;
        let objeto = match valor {
            Value::Object(mapa) => vec![mapa],
            _ => {
                return Err(ErroDeRefino::Decodificacao(
                    "esperava um objeto de processo".to_owned(),
                ));
            }
        };
        Ok(vec![tabela_de(&objeto)?])
    }

    fn decodificar_arquivos(
        &self,
        caminhos: &[std::path::PathBuf],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let mut objetos: Vec<Map<String, Value>> = Vec::with_capacity(caminhos.len());
        for caminho in caminhos {
            let valor: Value = serde_json::from_slice(&std::fs::read(caminho)?)
                .map_err(|erro| ErroDeRefino::Decodificacao(format!("JSON inválido: {erro}")))?;
            if let Value::Object(mapa) = valor {
                objetos.push(mapa);
            }
        }
        if objetos.is_empty() {
            return Ok(Vec::new());
        }
        Ok(vec![tabela_de(&objetos)?])
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::csv_zip::testes::ler_parte;

    #[test]
    fn agrega_varios_processos_em_uma_tabela() {
        let a = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(a.path(), br#"{"id":1,"sigla":"PLS","conteudo":{"x":9}}"#).unwrap();
        let b = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(b.path(), br#"{"id":2,"ano":1997}"#).unwrap();
        let caminhos = vec![a.path().to_path_buf(), b.path().to_path_buf()];

        let partes = DecodificadorSenadoProcessosDetalhes
            .decodificar_arquivos(&caminhos, "application/json")
            .unwrap();
        assert_eq!(
            partes.len(),
            1,
            "uma tabela agregada, não um Parquet por arquivo"
        );
        let (colunas, linhas) = ler_parte(&partes[0]);
        // Chaves do serde_json::Map vêm ordenadas; a ordem não importa para o bronze.
        assert_eq!(colunas, vec!["conteudo", "id", "sigla", "ano"]);
        assert_eq!(linhas.len(), 2);
        assert_eq!(linhas[0][1], Some("1".into())); // id
        assert_eq!(linhas[0][0], Some(r#"{"x":9}"#.into())); // conteudo
        assert_eq!(linhas[1][3], Some("1997".into())); // ano
        assert_eq!(linhas[1][2], None); // sigla ausente no 2o
    }
}
