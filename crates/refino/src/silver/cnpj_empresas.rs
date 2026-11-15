//! Contrato Silver para o cadastro de empresas da Receita Federal (CNPJ).

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// SQL do contrato `cnpj.empresas.v1`.
pub const SQL_CNPJ_EMPRESAS: &str = include_str!("../../sql/silver/cnpj_empresas.sql");

/// Schema `cnpj.empresas.v1`.
pub fn esquema_cnpj_empresas() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("cnpj_basico", DataType::Utf8, false),
        Field::new("razao_social", DataType::Utf8, false),
        Field::new("natureza_juridica", DataType::Utf8, true),
        Field::new("qualificacao_responsavel", DataType::Utf8, true),
        Field::new("capital_social", DataType::Decimal128(18, 2), true),
        Field::new("porte_empresa", DataType::Utf8, true),
        Field::new("ente_federativo", DataType::Utf8, true),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

#[cfg(test)]
mod testes {
    use std::{fs, path::Path};

    use arrow::{
        array::{Array, Decimal128Array, StringArray},
        datatypes::Field,
        record_batch::RecordBatch,
    };
    use datafusion::physical_plan::SendableRecordBatchStream;
    use futures::StreamExt;
    use parquet::arrow::ArrowWriter;

    use super::*;
    use crate::executor_sql::{ConfiguracaoExecutorSql, executar_sql};

    const COLUNAS_BRONZE: &[&str] = &[
        "cnpj_basico",
        "razao_social",
        "natureza_juridica",
        "qualificacao_responsavel",
        "capital_social",
        "porte_empresa",
        "ente_federativo",
    ];

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
            SQL_CNPJ_EMPRESAS,
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
    async fn tipa_capital_social_e_preserva_textos() {
        let dir = tempfile::tempdir().unwrap();
        let lotes = consultar(
            dir.path(),
            &[vec![
                "12345678",
                "ACME LTDA",
                "2062",
                "50",
                "1000,50",
                "05",
                "SP",
            ]],
        )
        .await;
        let lote = &lotes[0];

        let esperado = esquema_cnpj_empresas();
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

        let cnpj = lote
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(cnpj.value(0), "12345678");

        let capital = lote
            .column(4)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        assert_eq!(capital.value(0), 100_050);
    }

    #[tokio::test]
    async fn capital_vazio_vira_null() {
        let dir = tempfile::tempdir().unwrap();
        let lotes = consultar(
            dir.path(),
            &[vec!["99999999", "TESTE SA", "2062", "50", "", "05", ""]],
        )
        .await;
        let lote = &lotes[0];
        let capital = lote
            .column(4)
            .as_any()
            .downcast_ref::<Decimal128Array>()
            .unwrap();
        assert!(capital.is_null(0));
    }
}
