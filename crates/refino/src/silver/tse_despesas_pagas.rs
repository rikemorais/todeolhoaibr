//! Contrato Silver das despesas pagas por candidatos do TSE em 2024.

#[cfg(test)]
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(test)]
use arrow::array::{
    Array, Date32Array, Decimal128Array, RecordBatch, StringArray, UInt16Array, UInt32Array,
};
use arrow::datatypes::{DataType, Field, Schema};
#[cfg(test)]
use chrono::NaiveDate;
#[cfg(test)]
use sha2::{Digest, Sha256};
#[cfg(test)]
use thiserror::Error;

#[cfg(test)]
#[derive(Debug, Error, PartialEq, Eq)]
/// Erros no contrato de despesas pagas do TSE.
pub enum ErroTseDespesasPagasSilver {
    /// O Parquet não contém as colunas textuais esperadas.
    #[error("schema Bronze de despesas pagas incompatível")]
    Schema,
    /// Um campo obrigatório da chave ou do lançamento é inválido.
    #[error("chave inválida na linha {linha}")]
    ChaveInvalida {
        /// Índice da linha dentro do lote, começando em zero.
        linha: usize,
    },
    /// Uma chave se repetiu com algum campo diferente.
    #[error("mesma chave de despesa paga com conteúdo diferente na linha {linha}")]
    ConflitoDeChave {
        /// Índice da linha conflitante dentro do lote, começando em zero.
        linha: usize,
    },
    /// A data informada não segue o formato do layout de 2024.
    #[error("data de pagamento inválida na linha {linha}")]
    Data {
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
    /// O valor não segue o formato decimal brasileiro.
    #[error("valor pago inválido na linha {linha}")]
    Valor {
        /// Índice da linha inválida dentro do lote, começando em zero.
        linha: usize,
    },
}

/// Estado de unicidade mantido entre os lotes do consolidado nacional.
#[cfg(test)]
#[derive(Default)]
pub struct EstadoDeChavesDeDespesasPagas {
    vistos: HashMap<String, [u8; 32]>,
    copias_exatas: u64,
}

#[cfg(test)]
impl EstadoDeChavesDeDespesasPagas {
    /// Quantas linhas repetidas e integralmente iguais foram removidas até agora.
    pub fn copias_exatas_removidas(&self) -> u64 {
        self.copias_exatas
    }
}

/// Schema versionado `tse.prestacao_contas.despesa_paga.v1`.
pub fn esquema_tse_despesas_pagas_2024() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("ano_eleicao", DataType::UInt16, false),
        Field::new("codigo_eleicao", DataType::Utf8, false),
        Field::new("sequencia_prestador", DataType::Utf8, false),
        Field::new("sequencia_despesa", DataType::UInt32, false),
        Field::new("sequencia_pagamento", DataType::UInt32, false),
        Field::new("data_pagamento", DataType::Date32, false),
        Field::new("valor_pago", DataType::Decimal128(18, 2), false),
        Field::new("registro_original", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

/// Tipa a chave, data e valor; elimina cópias integrais e rejeita conflitos de chave.
///
/// O estado deve ser mantido entre lotes de um mesmo consolidado `BRASIL`. Ele guarda a
/// chave e o SHA-256 do registro integral, usando memória proporcional às chaves distintas.
#[cfg(test)]
pub fn transformar_tse_despesas_pagas_2024(
    lote: &RecordBatch,
    estado: &mut EstadoDeChavesDeDespesasPagas,
) -> Result<RecordBatch, ErroTseDespesasPagasSilver> {
    if lote
        .schema()
        .fields()
        .iter()
        .any(|campo| campo.data_type() != &DataType::Utf8)
    {
        return Err(ErroTseDespesasPagasSilver::Schema);
    }
    let coluna = |nome: &str| -> Result<&StringArray, ErroTseDespesasPagasSilver> {
        let indice = lote
            .schema()
            .index_of(nome)
            .map_err(|_| ErroTseDespesasPagasSilver::Schema)?;
        lote.column(indice)
            .as_any()
            .downcast_ref()
            .ok_or(ErroTseDespesasPagasSilver::Schema)
    };
    let ano = coluna("AA_ELEICAO")?;
    let eleicao = coluna("CD_ELEICAO")?;
    let prestador = coluna("SQ_PRESTADOR_CONTAS")?;
    let despesa = coluna("SQ_DESPESA")?;
    let pagamento = coluna("SQ_PARCELAMENTO_DESPESA")?;
    let data = coluna("DT_PAGTO_DESPESA")?;
    let valor = coluna("VR_PAGTO_DESPESA")?;

    let mut prestadores = Vec::with_capacity(lote.num_rows());
    let mut codigos_eleicao = Vec::with_capacity(lote.num_rows());
    let mut despesas = Vec::with_capacity(lote.num_rows());
    let mut pagamentos = Vec::with_capacity(lote.num_rows());
    let mut datas = Vec::with_capacity(lote.num_rows());
    let mut valores = Vec::with_capacity(lote.num_rows());
    let mut originais = Vec::with_capacity(lote.num_rows());
    for linha in 0..lote.num_rows() {
        if texto(ano, linha).and_then(|s| s.parse::<u16>().ok()) != Some(2024) {
            return Err(ErroTseDespesasPagasSilver::ChaveInvalida { linha });
        }
        let codigo_eleicao =
            texto(eleicao, linha).ok_or(ErroTseDespesasPagasSilver::ChaveInvalida { linha })?;
        let sequencia_prestador =
            texto(prestador, linha).ok_or(ErroTseDespesasPagasSilver::ChaveInvalida { linha })?;
        let sequencia_despesa =
            numero(despesa, linha).ok_or(ErroTseDespesasPagasSilver::ChaveInvalida { linha })?;
        let sequencia_pagamento =
            numero(pagamento, linha).ok_or(ErroTseDespesasPagasSilver::ChaveInvalida { linha })?;
        let chave = format!(
            "2024\0{codigo_eleicao}\0{sequencia_prestador}\0{sequencia_despesa}\0{sequencia_pagamento}"
        );
        let original = registro_original(lote, linha)?;
        let digest: [u8; 32] = Sha256::digest(original.as_bytes()).into();
        match estado.vistos.get(&chave) {
            Some(anterior) if anterior == &digest => {
                estado.copias_exatas += 1;
                continue;
            }
            Some(_) => return Err(ErroTseDespesasPagasSilver::ConflitoDeChave { linha }),
            None => {
                estado.vistos.insert(chave, digest);
            }
        }
        let data: i32 = texto(data, linha)
            .and_then(|s| NaiveDate::parse_from_str(s, "%d/%m/%Y").ok())
            .and_then(|date| {
                date.signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1)?)
                    .num_days()
                    .try_into()
                    .ok()
            })
            .ok_or(ErroTseDespesasPagasSilver::Data { linha })?;
        let valor = texto(valor, linha)
            .and_then(decimal)
            .ok_or(ErroTseDespesasPagasSilver::Valor { linha })?;
        prestadores.push(sequencia_prestador.to_owned());
        codigos_eleicao.push(codigo_eleicao.to_owned());
        despesas.push(sequencia_despesa);
        pagamentos.push(sequencia_pagamento);
        datas.push(data);
        valores.push(Some(valor));
        originais.push(Some(original));
    }
    let valores = Decimal128Array::from(valores)
        .with_precision_and_scale(18, 2)
        .map_err(|_| ErroTseDespesasPagasSilver::Schema)?;
    let n_linhas = prestadores.len();
    let mut cols: Vec<Arc<dyn arrow::array::Array>> = vec![
        Arc::new(UInt16Array::from_value(2024, n_linhas)),
        Arc::new(StringArray::from(codigos_eleicao)),
        Arc::new(StringArray::from(prestadores)),
        Arc::new(UInt32Array::from(despesas)),
        Arc::new(UInt32Array::from(pagamentos)),
        Arc::new(Date32Array::from(datas)),
        Arc::new(valores),
        Arc::new(StringArray::from(originais)),
    ];
    cols.extend(crate::executor_sql::colunas_linhagem_teste(n_linhas));
    RecordBatch::try_new(esquema_tse_despesas_pagas_2024(), cols)
        .map_err(|_| ErroTseDespesasPagasSilver::Schema)
}

#[cfg(test)]
fn texto(coluna: &StringArray, linha: usize) -> Option<&str> {
    (!coluna.is_null(linha))
        .then(|| coluna.value(linha).trim())
        .filter(|s| !s.is_empty() && *s != "-1" && *s != "-3")
}

#[cfg(test)]
fn numero(coluna: &StringArray, linha: usize) -> Option<u32> {
    texto(coluna, linha)?.parse().ok()
}

#[cfg(test)]
fn decimal(texto: &str) -> Option<i128> {
    let (inteiro, centavos) = texto.trim().split_once(',')?;
    if inteiro.is_empty()
        || !inteiro.bytes().all(|b| b.is_ascii_digit())
        || centavos.len() != 2
        || !centavos.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    inteiro
        .parse::<i128>()
        .ok()?
        .checked_mul(100)?
        .checked_add(centavos.parse().ok()?)
}

#[cfg(test)]
fn registro_original(
    lote: &RecordBatch,
    linha: usize,
) -> Result<String, ErroTseDespesasPagasSilver> {
    let mut original = serde_json::Map::new();
    for (indice, campo) in lote.schema().fields().iter().enumerate() {
        let coluna = lote
            .column(indice)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or(ErroTseDespesasPagasSilver::Schema)?;
        original.insert(
            campo.name().clone(),
            if coluna.is_null(linha) {
                serde_json::Value::Null
            } else {
                serde_json::Value::String(coluna.value(linha).to_owned())
            },
        );
    }
    serde_json::to_string(&original).map_err(|_| ErroTseDespesasPagasSilver::Schema)
}

#[cfg(test)]
mod testes {
    use super::*;
    use arrow::array::{Date32Array, Decimal128Array, StringArray, UInt32Array};
    use arrow::datatypes::DataType;
    use std::sync::Arc;

    fn lote(valores: &[&str]) -> RecordBatch {
        let n = valores.len();
        RecordBatch::try_from_iter(vec![
            (
                "AA_ELEICAO",
                Arc::new(StringArray::from(vec![Some("2024"); n])) as _,
            ),
            (
                "CD_ELEICAO",
                Arc::new(StringArray::from(vec![Some("2"); n])) as _,
            ),
            (
                "SQ_PRESTADOR_CONTAS",
                Arc::new(StringArray::from(vec![Some("17"); n])) as _,
            ),
            (
                "SQ_DESPESA",
                Arc::new(StringArray::from(vec![Some("8"); n])) as _,
            ),
            (
                "SQ_PARCELAMENTO_DESPESA",
                Arc::new(StringArray::from(vec![Some("1"); n])) as _,
            ),
            (
                "DT_PAGTO_DESPESA",
                Arc::new(StringArray::from(vec![Some("06/10/2024"); n])) as _,
            ),
            (
                "VR_PAGTO_DESPESA",
                Arc::new(StringArray::from(
                    valores.iter().copied().map(Some).collect::<Vec<_>>(),
                )) as _,
            ),
        ])
        .unwrap()
    }

    #[test]
    fn remove_copia_exata_e_tipa_data_valor_e_sequencias() {
        let bronze = lote(&["123,45", "123,45"]);
        let mut chaves = EstadoDeChavesDeDespesasPagas::default();
        let silver = transformar_tse_despesas_pagas_2024(&bronze, &mut chaves).unwrap();

        assert_eq!(silver.num_rows(), 1);
        assert_eq!(chaves.copias_exatas_removidas(), 1);
        assert_eq!(silver.column(0).data_type(), &DataType::UInt16);
        assert_eq!(
            silver
                .column(3)
                .as_any()
                .downcast_ref::<UInt32Array>()
                .unwrap()
                .value(0),
            8
        );
        assert_eq!(
            silver
                .column(5)
                .as_any()
                .downcast_ref::<Date32Array>()
                .unwrap()
                .value(0),
            20_002
        );
        assert_eq!(
            silver
                .column(6)
                .as_any()
                .downcast_ref::<Decimal128Array>()
                .unwrap()
                .value(0),
            12_345
        );
    }

    #[test]
    fn rejeita_conteudo_diferente_sob_a_mesma_chave() {
        let bronze = lote(&["123,45", "123,46"]);
        let mut chaves = EstadoDeChavesDeDespesasPagas::default();
        assert_eq!(
            transformar_tse_despesas_pagas_2024(&bronze, &mut chaves),
            Err(ErroTseDespesasPagasSilver::ConflitoDeChave { linha: 1 })
        );
    }

    #[test]
    fn remove_copia_exata_que_aparece_em_lotes_diferentes() {
        let mut chaves = EstadoDeChavesDeDespesasPagas::default();
        let primeira =
            transformar_tse_despesas_pagas_2024(&lote(&["123,45"]), &mut chaves).unwrap();
        let segunda = transformar_tse_despesas_pagas_2024(&lote(&["123,45"]), &mut chaves).unwrap();

        assert_eq!(primeira.num_rows(), 1);
        assert_eq!(segunda.num_rows(), 0);
        assert_eq!(chaves.copias_exatas_removidas(), 1);
    }
}
