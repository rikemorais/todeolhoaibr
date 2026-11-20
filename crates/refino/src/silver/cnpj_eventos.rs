//! Contrato Silver para a tabela de eventos do CNPJ (diff entre snapshots).

use std::sync::Arc;

use arrow::datatypes::{DataType, Field, Schema, SchemaRef};

/// SQL do contrato `cnpj.eventos.empresas.v1`.
pub const SQL_CNPJ_EVENTOS_EMPRESAS: &str =
    include_str!("../../sql/silver/cnpj_eventos_empresas.sql");

/// Schema `cnpj.eventos.v1`.
pub fn esquema_cnpj_eventos() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("tipo_evento", DataType::Utf8, false),
        Field::new("tabela", DataType::Utf8, false),
        Field::new("cnpj_basico", DataType::Utf8, false),
        Field::new("campo_alterado", DataType::Utf8, true),
        Field::new("valor_anterior", DataType::Utf8, true),
        Field::new("valor_novo", DataType::Utf8, true),
        Field::new("manifesto_anterior", DataType::Utf8, false),
        Field::new("manifesto_novo", DataType::Utf8, false),
        Field::new("_tdoa_origem_manifesto", DataType::Utf8, false),
        Field::new("_tdoa_origem_parte", DataType::Utf8, false),
        Field::new("_tdoa_origem_ordinal", DataType::UInt64, false),
    ]))
}

#[cfg(test)]
mod testes {
    use std::{fs, path::Path};

    use arrow::{
        array::{Array, StringArray, UInt64Array},
        datatypes::Field,
        record_batch::RecordBatch,
    };
    use datafusion::physical_plan::SendableRecordBatchStream;
    use futures::StreamExt;
    use parquet::arrow::ArrowWriter;

    use super::*;
    use crate::executor_sql::{ConfiguracaoExecutorSql, executar_sql_duplo};

    const COLUNAS_EMPRESAS: &[&str] = &[
        "cnpj_basico",
        "razao_social",
        "natureza_juridica",
        "qualificacao_responsavel",
        "capital_social",
        "porte_empresa",
        "ente_federativo",
    ];

    fn parquet_empresas(linhas: &[Vec<&str>]) -> Vec<u8> {
        let mut campos: Vec<Field> = COLUNAS_EMPRESAS
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
        let mut arrays: Vec<Arc<dyn Array>> = (0..COLUNAS_EMPRESAS.len())
            .map(|i| {
                Arc::new(StringArray::from(
                    linhas.iter().map(|r| r[i]).collect::<Vec<_>>(),
                )) as _
            })
            .collect();
        arrays.extend([
            Arc::new(StringArray::from(vec!["m"; n])) as _,
            Arc::new(StringArray::from(vec!["p"; n])) as _,
            Arc::new(UInt64Array::from_iter_values(0..n as u64)) as _,
        ]);
        let batch = RecordBatch::try_new(schema.clone(), arrays).unwrap();
        let mut bytes = Vec::new();
        let mut writer = ArrowWriter::try_new(&mut bytes, schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
        bytes
    }

    fn sql_com_manifestos(manifesto_vig: &str, manifesto_ant: &str) -> String {
        SQL_CNPJ_EVENTOS_EMPRESAS
            .replace("{{MANIFESTO_VIGENTE}}", manifesto_vig)
            .replace("{{MANIFESTO_ANTERIOR}}", manifesto_ant)
    }

    async fn consultar(
        dir: &Path,
        vigente: &[Vec<&str>],
        anterior: &[Vec<&str>],
    ) -> Vec<RecordBatch> {
        let dir_vig = dir.join("vigente");
        let dir_ant = dir.join("anterior");
        let dir_spill = dir.join("spill");
        fs::create_dir_all(&dir_vig).unwrap();
        fs::create_dir_all(&dir_ant).unwrap();
        fs::create_dir_all(&dir_spill).unwrap();
        fs::write(dir_vig.join("parte.parquet"), parquet_empresas(vigente)).unwrap();
        fs::write(dir_ant.join("parte.parquet"), parquet_empresas(anterior)).unwrap();
        let sql = sql_com_manifestos("manifesto-vig", "manifesto-ant");
        let mut fluxo: SendableRecordBatchStream = executar_sql_duplo(
            &dir_vig,
            &dir_ant,
            &dir_spill,
            &sql,
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

    fn extrair_eventos(lotes: &[RecordBatch]) -> Vec<(String, String, Option<String>)> {
        let mut eventos = Vec::new();
        for lote in lotes {
            let tipos = lote
                .column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let cnpjs = lote
                .column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            let campos = lote
                .column(3)
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap();
            for i in 0..lote.num_rows() {
                eventos.push((
                    tipos.value(i).to_owned(),
                    cnpjs.value(i).to_owned(),
                    if campos.is_null(i) {
                        None
                    } else {
                        Some(campos.value(i).to_owned())
                    },
                ));
            }
        }
        eventos
    }

    #[tokio::test]
    async fn detecta_abertura() {
        let dir = tempfile::tempdir().unwrap();
        let anterior = vec![vec!["111", "A", "2062", "50", "1000", "05", "SP"]];
        let vigente = vec![
            vec!["111", "A", "2062", "50", "1000", "05", "SP"],
            vec!["222", "B", "2062", "50", "2000", "03", "RJ"],
        ];
        let lotes = consultar(dir.path(), &vigente, &anterior).await;
        let eventos = extrair_eventos(&lotes);
        let aberturas: Vec<_> = eventos.iter().filter(|e| e.0 == "abertura").collect();
        assert_eq!(aberturas.len(), 1);
        assert_eq!(aberturas[0].1, "222");
    }

    #[tokio::test]
    async fn detecta_fechamento() {
        let dir = tempfile::tempdir().unwrap();
        let anterior = vec![
            vec!["111", "A", "2062", "50", "1000", "05", "SP"],
            vec!["333", "C", "2062", "50", "3000", "01", "MG"],
        ];
        let vigente = vec![vec!["111", "A", "2062", "50", "1000", "05", "SP"]];
        let lotes = consultar(dir.path(), &vigente, &anterior).await;
        let eventos = extrair_eventos(&lotes);
        let fechamentos: Vec<_> = eventos.iter().filter(|e| e.0 == "fechamento").collect();
        assert_eq!(fechamentos.len(), 1);
        assert_eq!(fechamentos[0].1, "333");
    }

    #[tokio::test]
    async fn detecta_alteracao_campo_a_campo() {
        let dir = tempfile::tempdir().unwrap();
        let anterior = vec![vec!["111", "A", "2062", "50", "1000", "05", "SP"]];
        let vigente = vec![vec!["111", "B", "2062", "50", "2000", "05", "SP"]];
        let lotes = consultar(dir.path(), &vigente, &anterior).await;
        let eventos = extrair_eventos(&lotes);
        let alteracoes: Vec<_> = eventos.iter().filter(|e| e.0 == "alteracao").collect();
        assert_eq!(alteracoes.len(), 2);
        let campos: Vec<_> = alteracoes.iter().map(|e| e.2.as_deref().unwrap()).collect();
        assert!(campos.contains(&"razao_social"));
        assert!(campos.contains(&"capital_social"));
    }

    #[tokio::test]
    async fn nao_emite_evento_para_linhas_iguais() {
        let dir = tempfile::tempdir().unwrap();
        let dados = vec![vec!["111", "A", "2062", "50", "1000", "05", "SP"]];
        let lotes = consultar(dir.path(), &dados, &dados).await;
        let eventos = extrair_eventos(&lotes);
        assert!(eventos.is_empty());
    }

    #[tokio::test]
    async fn schema_confere() {
        let dir = tempfile::tempdir().unwrap();
        let dados = vec![vec!["111", "A", "2062", "50", "1000", "05", "SP"]];
        let vigente = vec![
            vec!["111", "A", "2062", "50", "1000", "05", "SP"],
            vec!["222", "B", "2062", "50", "2000", "03", "RJ"],
        ];
        let lotes = consultar(dir.path(), &vigente, &dados).await;
        let lote = &lotes[0];
        let esperado = esquema_cnpj_eventos();
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
    }

    #[tokio::test]
    async fn null_em_ambos_nao_gera_falso_positivo() {
        let dir = tempfile::tempdir().unwrap();
        let anterior = vec![vec!["111", "A", "2062", "50", "1000", "05", ""]];
        let vigente = vec![vec!["111", "A", "2062", "50", "1000", "05", ""]];
        let lotes = consultar(dir.path(), &vigente, &anterior).await;
        let eventos = extrair_eventos(&lotes);
        assert!(eventos.is_empty());
    }
}
