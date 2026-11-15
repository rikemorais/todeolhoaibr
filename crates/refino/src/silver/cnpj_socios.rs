//! Contrato Silver para o quadro societário da Receita Federal (CNPJ).

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// SQL do contrato `cnpj.socios.v1`.
pub const SQL_CNPJ_SOCIOS: &str = include_str!("../../sql/silver/cnpj_socios.sql");

/// Colunas Bronze (all-string) da tabela sócios.
pub(crate) const COLUNAS_BRONZE: &[&str] = &[
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

/// Schema `cnpj.socios.v1`.
pub fn esquema_cnpj_socios() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("cnpj_basico", DataType::Utf8, false),
        Field::new("identificador_socio", DataType::UInt8, true),
        Field::new("nome_socio", DataType::Utf8, true),
        Field::new("cnpj_cpf_socio", DataType::Utf8, true),
        Field::new("qualificacao", DataType::Utf8, true),
        Field::new("data_entrada", DataType::Date32, true),
        Field::new("pais", DataType::Utf8, true),
        Field::new("representante_legal", DataType::Utf8, true),
        Field::new("nome_representante", DataType::Utf8, true),
        Field::new("qualificacao_representante", DataType::Utf8, true),
        Field::new("faixa_etaria", DataType::UInt8, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

#[cfg(test)]
mod testes {
    use std::{fs, path::Path};

    use arrow::{
        array::{Array, Date32Array, StringArray, UInt8Array},
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
            SQL_CNPJ_SOCIOS,
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
    async fn tipa_identificador_faixa_etaria_e_data_entrada() {
        let dir = tempfile::tempdir().unwrap();
        let lotes = consultar(
            dir.path(),
            &[vec![
                "12345678",
                "2",
                "JOAO SILVA",
                "11122233344",
                "49",
                "20190601",
                "105",
                "",
                "",
                "00",
                "4",
            ]],
        )
        .await;
        let lote = &lotes[0];

        let esperado = esquema_cnpj_socios();
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

        let id = lote
            .column(1)
            .as_any()
            .downcast_ref::<UInt8Array>()
            .unwrap();
        assert_eq!(id.value(0), 2);

        let data = lote
            .column(5)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        let esperada = NaiveDate::from_ymd_opt(2019, 6, 1)
            .unwrap()
            .signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            .num_days() as i32;
        assert_eq!(data.value(0), esperada);

        let faixa = lote
            .column(10)
            .as_any()
            .downcast_ref::<UInt8Array>()
            .unwrap();
        assert_eq!(faixa.value(0), 4);
    }
}
