//! Decodificador do bronze para os **agregados do IBGE** (API v3 de agregados).
//!
//! O JSON é aninhado: `[{id, variavel, unidade, resultados:[{classificacoes, series:[{
//! localidade:{id, nivel:{id, nome}, nome}, serie:{periodo: valor}}]}]}]`. O bronze explode
//! isso em uma **linha por (variável, localidade, período)**, com tudo como texto — sem
//! interpretar valores (isso é do silver). Serve para IPCA, INPC, PIB (municipal e
//! trimestral), PNAD e população.

use serde_json::Value;

use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::{ErroDeRefino, Tabela};

const COLUNAS: [&str; 9] = [
    "variavel_id",
    "variavel",
    "unidade",
    "classificacoes",
    "nivel_id",
    "nivel_nome",
    "localidade_id",
    "localidade_nome",
    "periodo",
];
// A coluna "valor" é acrescentada no fim; os períodos viram linhas.

/// Decodifica o JSON de agregados do IBGE, explodindo as séries em linhas.
pub struct DecodificadorIbgeAgregados {
    nome: &'static str,
}

impl DecodificadorIbgeAgregados {
    /// Cria o decodificador com o nome lógico da tabela.
    pub const fn novo(nome: &'static str) -> Self {
        Self { nome }
    }
}

fn texto(valor: Option<&Value>) -> Option<String> {
    match valor {
        None | Some(Value::Null) => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(outro) => Some(outro.to_string()),
    }
}

impl Decodificador for DecodificadorIbgeAgregados {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let raiz: Value = serde_json::from_slice(bytes)
            .map_err(|erro| ErroDeRefino::Decodificacao(format!("JSON inválido: {erro}")))?;
        let Value::Array(agregados) = raiz else {
            return Err(ErroDeRefino::Decodificacao(
                "esperava um array de agregados do IBGE".to_owned(),
            ));
        };

        let mut colunas: Vec<String> = COLUNAS.iter().map(|c| c.to_string()).collect();
        colunas.push("valor".to_owned());
        let mut tabela = Tabela::nova(colunas);

        for agregado in &agregados {
            let variavel_id = texto(agregado.get("id"));
            let variavel = texto(agregado.get("variavel"));
            let unidade = texto(agregado.get("unidade"));
            let resultados = agregado.get("resultados").and_then(Value::as_array);
            for resultado in resultados.into_iter().flatten() {
                let classificacoes = resultado
                    .get("classificacoes")
                    .filter(|c| !matches!(c, Value::Array(a) if a.is_empty()))
                    .map(ToString::to_string);
                let series = resultado.get("series").and_then(Value::as_array);
                for serie in series.into_iter().flatten() {
                    let localidade = serie.get("localidade");
                    let nivel = localidade.and_then(|l| l.get("nivel"));
                    let nivel_id = texto(nivel.and_then(|n| n.get("id")));
                    let nivel_nome = texto(nivel.and_then(|n| n.get("nome")));
                    let localidade_id = texto(localidade.and_then(|l| l.get("id")));
                    let localidade_nome = texto(localidade.and_then(|l| l.get("nome")));
                    let valores = serie.get("serie").and_then(Value::as_object);
                    for (periodo, valor) in valores.into_iter().flatten() {
                        tabela = tabela.com_linha([
                            variavel_id.clone(),
                            variavel.clone(),
                            unidade.clone(),
                            classificacoes.clone(),
                            nivel_id.clone(),
                            nivel_nome.clone(),
                            localidade_id.clone(),
                            localidade_nome.clone(),
                            Some(periodo.clone()),
                            texto(Some(valor)),
                        ]);
                    }
                }
            }
        }

        Ok(vec![ParteBronze::de_tabela(self.nome, &tabela)?])
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::csv_zip::testes::ler_parte;

    #[test]
    fn explode_series_em_linhas_por_periodo() {
        let json = r#"[{"id":"37","variavel":"PIB","unidade":"Mil Reais","resultados":[
            {"classificacoes":[],"series":[
                {"localidade":{"id":"1100015","nivel":{"id":"N6","nome":"Município"},"nome":"Alta Floresta"},
                 "serie":{"2014":"377799","2015":"400000"}}
            ]}]}]"#;
        let partes = DecodificadorIbgeAgregados::novo("pib")
            .decodificar(json.as_bytes(), "application/json")
            .unwrap();
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas.last().unwrap(), "valor");
        assert_eq!(linhas.len(), 2, "duas linhas: 2014 e 2015");
        // variavel_id, variavel, unidade, classificacoes, nivel_id, nivel_nome, loc_id, loc_nome, periodo, valor
        assert_eq!(linhas[0][0], Some("37".into()));
        assert_eq!(linhas[0][4], Some("N6".into()));
        assert_eq!(linhas[0][6], Some("1100015".into()));
        assert_eq!(linhas[0][8], Some("2014".into()));
        assert_eq!(linhas[0][9], Some("377799".into()));
        assert_eq!(linhas[1][8], Some("2015".into()));
        assert_eq!(linhas[0][3], None, "classificacoes vazio vira nulo");
    }
}
