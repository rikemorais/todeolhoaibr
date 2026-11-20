//! Contrato Silver das estatísticas mensais do Mapa de Empresas por UF.

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// Schema `mapa_empresas.movimentacao.uf.v1`.
pub fn esquema_mapa_empresas() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("periodo", DataType::Utf8, false),
        Field::new("data_referencia", DataType::Date32, false),
        Field::new("data_geracao", DataType::Date32, false),
        Field::new("codigo_ibge", DataType::Utf8, false),
        Field::new("uf", DataType::Utf8, false),
        Field::new("estado", DataType::Utf8, false),
        Field::new("empresas_abertas", DataType::Int64, false),
        Field::new("empresas_fechadas", DataType::Int64, false),
        Field::new("empresas_ativas", DataType::Int64, false),
        Field::new("saldo", DataType::Int64, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

#[cfg(test)]
mod testes {
    use std::{fs, path::Path};

    use arrow::{
        array::{Array, Date32Array, Int64Array, StringArray},
        datatypes::Field,
        record_batch::RecordBatch,
    };
    use datafusion::physical_plan::SendableRecordBatchStream;
    use futures::StreamExt;
    use parquet::arrow::ArrowWriter;

    use super::*;
    use crate::executor_sql::{ConfiguracaoExecutorSql, executar_sql};

    const SQL: &str = include_str!("../../sql/silver/mapa_empresas.sql");

    fn parquet(rows: &[Vec<&str>]) -> Vec<u8> {
        let mut colunas = vec![
            "dt_referencia",
            "dt_geracao",
            "cod_ibge",
            "UF",
            "Estado",
            "Empresas Abertas",
            "Empresas Fechadas",
            "Empresas Ativas",
        ]
        .into_iter()
        .map(|nome| Field::new(nome, DataType::Utf8, false))
        .collect::<Vec<_>>();
        colunas.extend([
            Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
            Field::new("_tdoa_origem_parte", DataType::Utf8, false),
            Field::new("_tdoa_origem_url", DataType::Utf8, false),
            Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
        ]);
        let schema = Arc::new(Schema::new(colunas));
        let mut arrays = (0..8)
            .map(|indice| {
                Arc::new(StringArray::from(
                    rows.iter().map(|row| row[indice]).collect::<Vec<_>>(),
                )) as _
            })
            .collect::<Vec<_>>();
        arrays.extend([
            Arc::new(StringArray::from(vec!["mapa_empresas/movimentacao/periodo=2026-08/data_ingestao=2026-09-04/execucao=teste/_manifesto.json"; rows.len()])) as _,
            Arc::new(StringArray::from(vec!["parte"; rows.len()])) as _,
            Arc::new(StringArray::from(vec!["url"; rows.len()])) as _,
            Arc::new(arrow::array::UInt64Array::from_iter_values(
                (0..rows.len()).map(|indice| indice as u64),
            )) as _,
        ]);
        let batch = RecordBatch::try_new(schema.clone(), arrays).unwrap();
        let mut bytes = Vec::new();
        let mut writer = ArrowWriter::try_new(&mut bytes, schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
        bytes
    }

    async fn consultar(
        diretorio: &Path,
        rows: &[Vec<&str>],
    ) -> datafusion::common::Result<Vec<RecordBatch>> {
        fs::write(diretorio.join("parte.parquet"), parquet(rows)).unwrap();
        let spill = diretorio.join("spill");
        fs::create_dir(&spill).unwrap();
        let mut fluxo: SendableRecordBatchStream = executar_sql(
            diretorio,
            &spill,
            SQL,
            &ConfiguracaoExecutorSql {
                limite_entrada_bytes: 1024 * 1024,
                limite_memoria_bytes: 64 * 1024 * 1024,
                limite_spill_bytes: 64 * 1024 * 1024,
                particoes: 1,
                tamanho_lote: 8192,
            },
        )
        .await
        .map_err(|erro| datafusion::common::DataFusionError::Execution(erro.to_string()))?;
        let mut saida = Vec::new();
        while let Some(lote) = fluxo.next().await {
            saida.push(lote?);
        }
        Ok(saida)
    }

    fn linha(uf: &str) -> Vec<&str> {
        vec![
            "46265",
            "46269",
            "31",
            uf,
            "Minas Gerais - MG",
            "48864",
            "27993",
            "2731200",
        ]
    }

    #[tokio::test]
    async fn tipa_as_metricas_calcula_saldo_e_converte_datas_excel() {
        let diretorio = tempfile::tempdir().unwrap();
        let lotes = consultar(diretorio.path(), &[linha("MG")]).await.unwrap();
        let lote = &lotes[0];

        let esperado = esquema_mapa_empresas();
        assert_eq!(
            lote.schema()
                .fields()
                .iter()
                .map(|campo| (campo.name().as_str(), campo.data_type()))
                .collect::<Vec<_>>(),
            esperado
                .fields()
                .iter()
                .map(|campo| (campo.name().as_str(), campo.data_type()))
                .collect::<Vec<_>>()
        );
        assert_eq!(lote.num_rows(), 1);
        assert_eq!(
            lote.column_by_name("periodo")
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "2026-08"
        );
        assert_eq!(
            lote.column_by_name("data_referencia")
                .unwrap()
                .as_any()
                .downcast_ref::<Date32Array>()
                .unwrap()
                .value(0),
            20_696 // 2026-08-31, serial Excel 46265.
        );
        assert_eq!(
            lote.column_by_name("data_geracao")
                .unwrap()
                .as_any()
                .downcast_ref::<Date32Array>()
                .unwrap()
                .value(0),
            20_700 // 2026-09-04, serial Excel 46269.
        );
        assert_eq!(
            lote.column_by_name("saldo")
                .unwrap()
                .as_any()
                .downcast_ref::<Int64Array>()
                .unwrap()
                .value(0),
            20_871
        );
        assert_eq!(
            lote.column_by_name("codigo_ibge")
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .value(0),
            "31"
        );
    }

    #[tokio::test]
    async fn rejeita_uf_duplicada_no_mesmo_periodo() {
        let diretorio = tempfile::tempdir().unwrap();
        let erro = consultar(diretorio.path(), &[linha("MG"), linha("MG")])
            .await
            .unwrap_err();

        assert!(erro.to_string().contains("chave_periodo_uf_unica"));
    }

    #[tokio::test]
    async fn rejeita_metricas_fora_do_tipo_declarado() {
        let diretorio = tempfile::tempdir().unwrap();
        let mut invalida = linha("MG");
        invalida[5] = "muitos";
        let erro = consultar(diretorio.path(), &[invalida]).await.unwrap_err();

        assert!(erro.to_string().contains("empresas_abertas_obrigatorio"));
    }
}
