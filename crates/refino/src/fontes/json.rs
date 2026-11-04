//! Decodificador do bronze para fontes que gravam **JSON de API** no raw.
//!
//! Cobre os casos de "array de registros": no topo (`bcb`), sob uma chave (`value` no
//! ipeadata, `dados` nas APIs da Câmara) ou um array de objetos (listas do IBGE, processos e
//! votações do Senado). Cada registro vira uma linha; os campos viram colunas de **texto**,
//! e valores aninhados (objeto/array) são guardados como **JSON compacto** — o bronze
//! mantém a fidelidade, a interpretação é do silver.

use serde_json::Value;

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, Tabela};

/// Decodifica um JSON de API em uma parte Parquet, a partir do array de registros.
pub struct DecodificadorJson {
    nome: &'static str,
    /// Chave do array de registros; `None` quando o array está no topo.
    raiz: Option<&'static str>,
}

impl DecodificadorJson {
    /// Para JSON cujo array de registros está no **topo** (ex.: `bcb`).
    pub const fn novo(nome: &'static str) -> Self {
        Self { nome, raiz: None }
    }

    /// Para JSON cujo array de registros está **sob uma chave** (ex.: `value`, `dados`).
    pub const fn com_raiz(nome: &'static str, raiz: &'static str) -> Self {
        Self {
            nome,
            raiz: Some(raiz),
        }
    }
}

/// Converte um valor JSON em texto: escalares viram o texto natural; objeto/array viram
/// JSON compacto; `null` vira ausência (nulo no Parquet).
fn texto_de(valor: &Value) -> Option<String> {
    match valor {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(n.to_string()),
        outro => Some(outro.to_string()),
    }
}

impl Decodificador for DecodificadorJson {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let raiz: Value = serde_json::from_slice(bytes)
            .map_err(|erro| ErroDeRefino::Decodificacao(format!("JSON inválido: {erro}")))?;
        let registros = match self.raiz {
            Some(chave) => raiz.get(chave),
            None => Some(&raiz),
        };
        let Some(Value::Array(registros)) = registros else {
            let onde = self
                .raiz
                .map(|chave| format!(" na chave '{chave}'"))
                .unwrap_or_default();
            return Err(ErroDeRefino::Decodificacao(format!(
                "esperava um array de registros{onde}"
            )));
        };

        // Colunas: a união das chaves dos registros, na ordem de primeira aparição.
        let mut colunas: Vec<String> = Vec::new();
        for registro in registros {
            if let Value::Object(mapa) = registro {
                for chave in mapa.keys() {
                    if !colunas.iter().any(|c| c == chave) {
                        colunas.push(chave.clone());
                    }
                }
            }
        }

        let mut tabela = Tabela::nova(colunas.clone());
        for registro in registros {
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

    fn c(s: &str) -> Option<String> {
        Some(s.to_owned())
    }

    #[test]
    fn decodifica_array_no_topo() {
        let json =
            br#"[{"data":"01/06/1989","valor":"19.68"},{"data":"01/07/1989","valor":"35.90"}]"#;
        let partes = DecodificadorJson::novo("serie")
            .decodificar(json, "application/json")
            .unwrap();
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, vec!["data", "valor"]);
        assert_eq!(
            linhas,
            vec![
                vec![c("01/06/1989"), c("19.68")],
                vec![c("01/07/1989"), c("35.90")]
            ]
        );
    }

    #[test]
    fn decodifica_array_sob_uma_chave_com_numeros_e_nulos() {
        let json = br#"{"value":[{"a":5.78,"b":"x"},{"a":null,"b":"y","c":true}]}"#;
        let partes = DecodificadorJson::com_raiz("v", "value")
            .decodificar(json, "application/json")
            .unwrap();
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, vec!["a", "b", "c"]);
        assert_eq!(
            linhas,
            vec![vec![c("5.78"), c("x"), None], vec![None, c("y"), c("true")]]
        );
    }

    #[test]
    fn aninhados_viram_json_compacto() {
        let json = br#"[{"id":11,"regiao":{"id":1,"sigla":"N"}}]"#;
        let partes = DecodificadorJson::novo("estados")
            .decodificar(json, "application/json")
            .unwrap();
        let (_, linhas) = ler_parte(&partes[0]);
        assert_eq!(linhas, vec![vec![c("11"), c(r#"{"id":1,"sigla":"N"}"#)]]);
    }

    #[test]
    fn falha_quando_nao_ha_array() {
        let erro = DecodificadorJson::com_raiz("x", "dados")
            .decodificar(b"{\"dados\":{}}", "application/json")
            .unwrap_err();
        assert!(matches!(erro, ErroDeRefino::Decodificacao(_)), "{erro:?}");
    }
}
