//! Contrato Silver para o cadastro de estabelecimentos da Receita Federal (CNPJ).

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// SQL do contrato `cnpj.estabelecimentos.v1`.
pub const SQL_CNPJ_ESTABELECIMENTOS: &str =
    include_str!("../../sql/silver/cnpj_estabelecimentos.sql");

/// Colunas Bronze (all-string) da tabela estabelecimentos.
pub(crate) const COLUNAS_BRONZE: &[&str] = &[
    "cnpj_basico",
    "cnpj_ordem",
    "cnpj_dv",
    "matriz_filial",
    "nome_fantasia",
    "situacao_cadastral",
    "data_situacao",
    "motivo_situacao",
    "cidade_exterior",
    "pais",
    "data_inicio",
    "cnae_principal",
    "cnae_secundaria",
    "tipo_logradouro",
    "logradouro",
    "numero",
    "complemento",
    "bairro",
    "cep",
    "uf",
    "municipio",
    "ddd1",
    "telefone1",
    "ddd2",
    "telefone2",
    "ddd_fax",
    "fax",
    "email",
    "situacao_especial",
    "data_situacao_especial",
];

/// Schema `cnpj.estabelecimentos.v1`.
pub fn esquema_cnpj_estabelecimentos() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("cnpj_basico", DataType::Utf8, false),
        Field::new("cnpj_ordem", DataType::Utf8, false),
        Field::new("cnpj_dv", DataType::Utf8, false),
        Field::new("matriz_filial", DataType::UInt8, true),
        Field::new("nome_fantasia", DataType::Utf8, true),
        Field::new("situacao_cadastral", DataType::Utf8, false),
        Field::new("data_situacao", DataType::Date32, true),
        Field::new("motivo_situacao", DataType::Utf8, true),
        Field::new("cidade_exterior", DataType::Utf8, true),
        Field::new("pais", DataType::Utf8, true),
        Field::new("data_inicio", DataType::Date32, true),
        Field::new("cnae_principal", DataType::Utf8, true),
        Field::new("cnae_secundaria", DataType::Utf8, true),
        Field::new("tipo_logradouro", DataType::Utf8, true),
        Field::new("logradouro", DataType::Utf8, true),
        Field::new("numero", DataType::Utf8, true),
        Field::new("complemento", DataType::Utf8, true),
        Field::new("bairro", DataType::Utf8, true),
        Field::new("cep", DataType::Utf8, true),
        Field::new("uf", DataType::Utf8, true),
        Field::new("municipio", DataType::Utf8, true),
        Field::new("ddd1", DataType::Utf8, true),
        Field::new("telefone1", DataType::Utf8, true),
        Field::new("ddd2", DataType::Utf8, true),
        Field::new("telefone2", DataType::Utf8, true),
        Field::new("ddd_fax", DataType::Utf8, true),
        Field::new("fax", DataType::Utf8, true),
        Field::new("email", DataType::Utf8, true),
        Field::new("situacao_especial", DataType::Utf8, true),
        Field::new("data_situacao_especial", DataType::Date32, true),
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

    fn linha_base() -> Vec<&'static str> {
        vec![
            "12345678",
            "0001",
            "50",
            "1",
            "ACME",
            "02",
            "20200315",
            "01",
            "",
            "",
            "20150101",
            "4781400",
            "",
            "RUA",
            "DAS FLORES",
            "100",
            "SALA 1",
            "CENTRO",
            "01001000",
            "SP",
            "7107",
            "11",
            "22223333",
            "",
            "",
            "",
            "",
            "acme@test.com",
            "",
            "",
        ]
    }

    async fn consultar(dir: &Path, linhas: &[Vec<&str>]) -> Vec<RecordBatch> {
        fs::write(dir.join("parte.parquet"), parquet_bronze(linhas)).unwrap();
        let spill = dir.join("spill");
        fs::create_dir(&spill).unwrap();
        let mut fluxo: SendableRecordBatchStream = executar_sql(
            dir,
            &spill,
            SQL_CNPJ_ESTABELECIMENTOS,
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
    async fn tipa_matriz_filial_datas_e_preserva_endereco() {
        let dir = tempfile::tempdir().unwrap();
        let lotes = consultar(dir.path(), &[linha_base()]).await;
        let lote = &lotes[0];

        let esperado = esquema_cnpj_estabelecimentos();
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

        let matriz = lote
            .column(3)
            .as_any()
            .downcast_ref::<UInt8Array>()
            .unwrap();
        assert_eq!(matriz.value(0), 1);

        let data_sit = lote
            .column(6)
            .as_any()
            .downcast_ref::<Date32Array>()
            .unwrap();
        let esperada = NaiveDate::from_ymd_opt(2020, 3, 15)
            .unwrap()
            .signed_duration_since(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
            .num_days() as i32;
        assert_eq!(data_sit.value(0), esperada);

        let email = lote
            .column(27)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert_eq!(email.value(0), "acme@test.com");
    }
}
