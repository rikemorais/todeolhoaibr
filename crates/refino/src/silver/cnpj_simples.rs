//! Contrato Silver para opção pelo Simples/MEI da Receita Federal (CNPJ).

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// SQL do contrato `cnpj.simples.v1`.
pub const SQL_CNPJ_SIMPLES: &str = include_str!("../../sql/silver/cnpj_simples.sql");

/// Colunas Bronze (all-string) da tabela simples.
pub(crate) const COLUNAS_BRONZE: &[&str] = &[
    "cnpj_basico",
    "opcao_simples",
    "data_opcao_simples",
    "data_exclusao_simples",
    "opcao_mei",
    "data_opcao_mei",
    "data_exclusao_mei",
];

/// Schema `cnpj.simples.v1`.
pub fn esquema_cnpj_simples() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("cnpj_basico", DataType::Utf8, false),
        Field::new("opcao_simples", DataType::Utf8, true),
        Field::new("data_opcao_simples", DataType::Date32, true),
        Field::new("data_exclusao_simples", DataType::Date32, true),
        Field::new("opcao_mei", DataType::Utf8, true),
        Field::new("data_opcao_mei", DataType::Date32, true),
        Field::new("data_exclusao_mei", DataType::Date32, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

#[cfg(test)]
mod testes {
    use std::{fs, path::Path};

    use arrow::{
        array::{Array, Date32Array, StringArray},
        datatypes::Field,
        record_batch::RecordBatch,
    };
    use chrono::NaiveDate;
    use datafusion::physical_plan::SendableRecordBatchStream;
    use futures::StreamExt;
    use parquet::arrow::ArrowWriter;

    use super::*;
    use crate::executor_sql::{ConfiguracaoExecutorSql, executar_sql};

    fn parquet_bronze(linhas: &[Vec<&str>]) -> Vec<u8> {
        let mut campos: Vec<Field> = COLUNAS_BRONZE
            .iter()
            .map(|nome| Field::new(*nome, DataType::Utf8, true))
            .collect();
        campos.extend([
            Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
            Field::new("_tdoa_origem_parte", DataType::Utf8, false),
            Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
        ]);
        let schema = Arc::new(Schema::new(campos));
        let n = linhas.len();
        let mut arrays: Vec<Arc<dyn Array>> = (0..COLUNAS_BRONZE.len())
            .map(|i| {
                Arc::new(StringArray::from(
                    linhas.iter().map(|r| r[i]).collect::<Vec<_>>(),
                )) as _
            })
            .collect();
        arrays.extend([
            Arc::new(StringArray::from(vec!["m"; n])) as _,
            Arc::new(StringArray::from(vec!["p"; n])) as _,
            Arc::new(arrow::array::UInt64Array::from_iter_values(0..n as u64)) as _,
        ]);
        let batch = RecordBatch::try_new(schema.clone(), arrays).unwrap();
        let mut bytes = Vec::new();
        let mut writer = ArrowWriter::try_new(&mut bytes, schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
        bytes
    }

    async fn consultar(dir: &Path, linhas: &[Vec<&str>]) -> Vec<RecordBatch> {
        fs::write(dir.join("parte.parquet"), parquet_bronze(linhas)).unwrap();
        let spill = dir.join("spill");
        fs::create_dir(&spill).unwrap();
        let mut fluxo: SendableRecordBatchStream = executar_sql(
            dir,
            &spill,
            SQL_CNPJ_SIMPLES,
            &ConfiguracaoExecutorSql {
                limite_entrada_bytes: 1024 * 1024,
                limite_memoria_bytes: 64 * 1024 * 1024,
                limite_spill_bytes: 64 * 1024 * 1024,
                particoes: 1,
                tamanho_lote: 8192,
            },
        )
        .await
        .unwrap();
        let mut saida = Vec::new();
        while let Some(lote) = fluxo.next().await {
            saida.push(lote.unwrap());
        }
        saida
    }

    #[tokio::test]
    async fn converte_datas_yyyymmdd_e_preserva_nulos() {
        let dir = tempfile::tempdir().unwrap();
        let lotes = consultar(
            dir.path(),
            &[vec![
                "12345678", "S", "20200315", "", "N", "20210101", "20221231",
            ]],
        )
        .await;
        let lote = &lotes[0];

        let esperado = esquema_cnpj_simples();
        assert_eq!(
            lote.schema()
                .fields()
                .iter()
                .map(|f| (f.name().as_str(), f.data_type().clone()))
                .collect::<Vec<_>>(),
            esperado
                .fields()
                .iter()
                .map(|f| (f.name().as_str(), f.data_type().clone()))
                .collect::<Vec<_>>(),
        );

        let data_opcao = lote
            .column(2)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        let esperada = NaiveDate::from_ymd_opt(2020, 3, 15)
            .unwrap()
            .signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            .num_days() as i32;
        assert_eq!(data_opcao.value(0), esperada);

        let data_exclusao_simples = lote
            .column(3)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        assert!(data_exclusao_simples.is_null(0));

        let data_exclusao_mei = lote
            .column(6)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        let esperada_mei = NaiveDate::from_ymd_opt(2022, 12, 31)
            .unwrap()
            .signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            .num_days() as i32;
        assert_eq!(data_exclusao_mei.value(0), esperada_mei);
    }
}
