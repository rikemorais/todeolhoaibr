//! Contrato Silver de votos nominais individuais do Senado.

use std::sync::Arc;

use arrow::array::{Array, StringArray, UInt32Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use serde_json::Value;
use thiserror::Error;

/// Erros ao expandir a lista aninhada de votos do Senado.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ErroSenadoVotosSilver {
    /// O Bronze não contém as colunas esperadas ou não as armazena como texto.
    #[error("schema Bronze de votações do Senado incompatível")]
    Schema,
    /// Um registro não contém código de sessão válido.
    #[error("código de sessão de votação inválido na linha {linha} do lote")]
    Sessao {
        /// Índice da linha Bronze, começando em zero.
        linha: usize,
    },
    /// A coluna `votos` não contém um array JSON válido.
    #[error("lista de votos inválida na linha {linha} do lote")]
    Votos {
        /// Índice da linha Bronze, começando em zero.
        linha: usize,
    },
    /// Um voto individual não traz código parlamentar.
    #[error("código parlamentar inválido no voto {ordem} da linha {linha} do lote")]
    Parlamentar {
        /// Índice da linha Bronze, começando em zero.
        linha: usize,
        /// Índice do voto no array original, começando em zero.
        ordem: usize,
    },
    /// O ordinal original não cabe no tipo de saída.
    #[error("ordinal do voto fora do limite no registro {linha} do lote")]
    Ordinal {
        /// Índice da linha Bronze, começando em zero.
        linha: usize,
    },
}

/// Schema versionado `senado.voto.nominal.v1`.
pub fn esquema_senado_votos() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("codigo_sessao_votacao", DataType::Utf8, false),
        Field::new("data_sessao", DataType::Utf8, true),
        Field::new("ordem_voto", DataType::UInt32, false),
        Field::new("codigo_parlamentar", DataType::Utf8, false),
        Field::new("nome_parlamentar", DataType::Utf8, true),
        Field::new("sigla_partido", DataType::Utf8, true),
        Field::new("uf", DataType::Utf8, true),
        Field::new("voto", DataType::Utf8, true),
        Field::new("voto_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

/// Expande cada elemento do array `votos` para uma linha, sem filtrar nem agregar votos.
pub fn transformar_senado_votos(lote: &RecordBatch) -> Result<RecordBatch, ErroSenadoVotosSilver> {
    let schema = lote.schema();
    let indice_sessao = indice_coluna(
        schema.as_ref(),
        &["codigoSessaoVotacao", "CodigoSessaoVotacao"],
    )?;
    let indice_data = indice_coluna(schema.as_ref(), &["dataSessao", "DataSessao"])?;
    let indice_votos = indice_coluna(schema.as_ref(), &["votos", "Votos"])?;
    if [indice_sessao, indice_data, indice_votos]
        .iter()
        .any(|&indice| schema.field(indice).data_type() != &DataType::Utf8)
    {
        return Err(ErroSenadoVotosSilver::Schema);
    }
    let sessoes = lote
        .column(indice_sessao)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or(ErroSenadoVotosSilver::Schema)?;
    let datas = lote
        .column(indice_data)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or(ErroSenadoVotosSilver::Schema)?;
    let votos = lote
        .column(indice_votos)
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or(ErroSenadoVotosSilver::Schema)?;

    let mut codigo_sessao = Vec::new();
    let mut data_sessao = Vec::new();
    let mut ordem_voto = Vec::new();
    let mut codigo_parlamentar = Vec::new();
    let mut nome_parlamentar = Vec::new();
    let mut sigla_partido = Vec::new();
    let mut uf = Vec::new();
    let mut voto = Vec::new();
    let mut voto_original = Vec::new();

    for linha in 0..lote.num_rows() {
        if sessoes.is_null(linha) || sessoes.value(linha).trim().is_empty() {
            return Err(ErroSenadoVotosSilver::Sessao { linha });
        }
        if votos.is_null(linha) {
            continue;
        }
        let json: Value = serde_json::from_str(votos.value(linha))
            .map_err(|_| ErroSenadoVotosSilver::Votos { linha })?;
        let registros = registros_de_votos(&json).ok_or(ErroSenadoVotosSilver::Votos { linha })?;
        for (ordem, registro) in registros.iter().enumerate() {
            let ordem_saida =
                u32::try_from(ordem).map_err(|_| ErroSenadoVotosSilver::Ordinal { linha })?;
            let identidade = registro
                .get("identificacaoParlamentar")
                .or_else(|| registro.get("IdentificacaoParlamentar"))
                .unwrap_or(registro);
            let codigo = campo(identidade, &["codigoParlamentar", "CodigoParlamentar"])
                .ok_or(ErroSenadoVotosSilver::Parlamentar { linha, ordem })?;

            codigo_sessao.push(Some(sessoes.value(linha).to_owned()));
            data_sessao.push((!datas.is_null(linha)).then(|| datas.value(linha).to_owned()));
            ordem_voto.push(ordem_saida);
            codigo_parlamentar.push(Some(codigo));
            nome_parlamentar.push(campo(identidade, &["nomeParlamentar", "NomeParlamentar"]));
            sigla_partido.push(campo(
                identidade,
                &["siglaPartido", "SiglaPartido", "SiglaPartidoParlamentar"],
            ));
            uf.push(campo(
                identidade,
                &["siglaUf", "siglaUF", "SiglaUF", "UfParlamentar"],
            ));
            voto.push(campo(
                registro,
                &[
                    "voto",
                    "SiglaVoto",
                    "Voto",
                    "descricaoVoto",
                    "DescricaoVoto",
                ],
            ));
            voto_original.push(Some(registro.to_string()));
        }
    }

    let n_linhas = codigo_sessao.len();
    let mut colunas: Vec<Arc<dyn arrow::array::Array>> = vec![
        Arc::new(StringArray::from(codigo_sessao)),
        Arc::new(StringArray::from(data_sessao)),
        Arc::new(UInt32Array::from(ordem_voto)),
        Arc::new(StringArray::from(codigo_parlamentar)),
        Arc::new(StringArray::from(nome_parlamentar)),
        Arc::new(StringArray::from(sigla_partido)),
        Arc::new(StringArray::from(uf)),
        Arc::new(StringArray::from(voto)),
        Arc::new(StringArray::from(voto_original)),
    ];
    colunas.extend(crate::executor_sql::colunas_linhagem_teste(n_linhas));
    RecordBatch::try_new(esquema_senado_votos(), colunas).map_err(|_| ErroSenadoVotosSilver::Schema)
}

fn registros_de_votos(json: &Value) -> Option<Vec<&Value>> {
    let envelope = json
        .get("Votos")
        .or_else(|| json.get("votos"))
        .unwrap_or(json);
    match envelope {
        Value::Array(registros) => Some(registros.iter().collect()),
        Value::Object(objeto) => {
            let lista = objeto
                .get("votoParlamentar")
                .or_else(|| objeto.get("VotoParlamentar"))?;
            match lista {
                Value::Array(registros) => Some(registros.iter().collect()),
                Value::Object(_) => Some(vec![lista]),
                _ => None,
            }
        }
        _ => None,
    }
}

fn indice_coluna(schema: &Schema, nomes: &[&str]) -> Result<usize, ErroSenadoVotosSilver> {
    nomes
        .iter()
        .find_map(|nome| schema.index_of(nome).ok())
        .ok_or(ErroSenadoVotosSilver::Schema)
}

fn campo(objeto: &Value, nomes: &[&str]) -> Option<String> {
    nomes.iter().find_map(|nome| {
        objeto.get(*nome).and_then(|valor| match valor {
            Value::String(texto) if !texto.trim().is_empty() => Some(texto.clone()),
            Value::Number(numero) => Some(numero.to_string()),
            _ => None,
        })
    })
}

#[cfg(test)]
mod testes {
    use arrow::array::{Array, StringArray, UInt32Array};

    use super::*;

    fn lote(votos: &str) -> RecordBatch {
        RecordBatch::try_from_iter(vec![
            (
                "codigoSessaoVotacao",
                Arc::new(StringArray::from(vec!["S1"])) as _,
            ),
            (
                "dataSessao",
                Arc::new(StringArray::from(vec!["2024-02-01"])) as _,
            ),
            ("votos", Arc::new(StringArray::from(vec![votos])) as _),
        ])
        .unwrap()
    }

    #[test]
    fn expande_votos_camel_case_sem_agregar_registros() {
        let bronze = lote(
            r#"[{"codigoParlamentar":7,"nomeParlamentar":"Senadora A","siglaPartido":"ABC","siglaUf":"DF","voto":"Sim"},{"codigoParlamentar":"8","nomeParlamentar":"Senador B","voto":"Não"}]"#,
        );
        let silver = transformar_senado_votos(&bronze).unwrap();
        assert_eq!(silver.schema(), esquema_senado_votos());
        assert_eq!(silver.num_rows(), 2);
        assert_eq!(
            silver
                .column(2)
                .as_any()
                .downcast_ref::<UInt32Array>()
                .unwrap()
                .values(),
            &[0, 1]
        );
        assert_eq!(
            silver
                .column(3)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "7"
        );
        assert_eq!(
            silver
                .column(7)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(1),
            "Não"
        );
        assert!(
            silver
                .column(8)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0)
                .contains("codigoParlamentar")
        );
    }

    #[test]
    fn entende_o_objeto_legado_votoparlamentar() {
        let bronze = lote(
            r#"{"VotoParlamentar":[{"IdentificacaoParlamentar":{"CodigoParlamentar":"12","NomeParlamentar":"Senador C","SiglaPartidoParlamentar":"XYZ","UfParlamentar":"AC"},"SiglaVoto":"Abstenção"}]}"#,
        );
        let silver = transformar_senado_votos(&bronze).unwrap();
        assert_eq!(silver.num_rows(), 1);
        assert_eq!(
            silver
                .column(3)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "12"
        );
        assert_eq!(
            silver
                .column(7)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "Abstenção"
        );
    }

    #[test]
    fn entende_nomes_legados_nas_colunas_da_votacao() {
        let bronze = RecordBatch::try_from_iter(vec![
            (
                "CodigoSessaoVotacao",
                Arc::new(StringArray::from(vec!["S2"])) as _,
            ),
            (
                "DataSessao",
                Arc::new(StringArray::from(vec!["2024-02-02"])) as _,
            ),
            (
                "Votos",
                Arc::new(StringArray::from(vec![
                    r#"[{"CodigoParlamentar": "13", "NomeParlamentar": "Senadora D", "SiglaVoto": "Sim"}]"#,
                ])) as _,
            ),
        ])
        .unwrap();

        let silver = transformar_senado_votos(&bronze).unwrap();
        assert_eq!(silver.num_rows(), 1);
        assert_eq!(
            silver
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "S2"
        );
        assert_eq!(
            silver
                .column(3)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "13"
        );
        assert_eq!(
            silver
                .column(7)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "Sim"
        );
    }

    #[test]
    fn mantem_sessoes_sem_votos_e_rejeita_identidade_ausente() {
        assert_eq!(transformar_senado_votos(&lote("[]")).unwrap().num_rows(), 0);
        assert_eq!(
            transformar_senado_votos(&lote(r#"[{"nomeParlamentar":"Sem código"}]"#)),
            Err(ErroSenadoVotosSilver::Parlamentar { linha: 0, ordem: 0 })
        );
        assert!(matches!(
            transformar_senado_votos(&lote("not json")),
            Err(ErroSenadoVotosSilver::Votos { linha: 0 })
        ));
    }
}
