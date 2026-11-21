//! Diagnóstico recuperável e quarentena da camada Silver.
//!
//! Linhas rejeitadas por um contrato SQL que projeta `_tdoa_rejeicao_motivo`
//! são separadas pelo publicador e gravadas em um Parquet de quarentena com
//! referência Bronze e motivo estável, sem copiar dados pessoais.

use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use arrow::record_batch::RecordBatch;

use crate::EscritorParquetTipado;

pub(crate) const COL_REJEICAO_MOTIVO: &str = "_tdoa_rejeicao_motivo";

pub(crate) fn esquema_quarentena() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("motivo", DataType::Utf8, false),
        Field::new("versao_regra", DataType::Utf8, false),
        Field::new("manifesto_bronze", DataType::Utf8, false),
        Field::new("parte_bronze", DataType::Utf8, false),
        Field::new("ordinal", DataType::UInt64, false),
    ]))
}

pub(crate) fn chave_quarentena(fonte: &str, recurso_destino: &str, id_execucao: &str) -> String {
    format!("_quarentena/{fonte}/{recurso_destino}/execucao={id_execucao}/diagnostico.parquet")
}

pub(crate) struct AcumuladorDeRejeicoes {
    regra: String,
    contagens: HashMap<String, u64>,
    lotes: Vec<RecordBatch>,
    total: u64,
}

impl AcumuladorDeRejeicoes {
    pub fn novo(regra: &str) -> Self {
        Self {
            regra: regra.to_owned(),
            contagens: HashMap::new(),
            total: 0,
            lotes: Vec::new(),
        }
    }

    pub fn acumular(
        &mut self,
        motivos: &StringArray,
        manifestos: &StringArray,
        partes: &StringArray,
        ordinais: &UInt64Array,
    ) {
        let n = motivos.len();
        if n == 0 {
            return;
        }
        for i in 0..n {
            let motivo = motivos.value(i);
            *self.contagens.entry(motivo.to_owned()).or_insert(0) += 1;
        }
        self.total += n as u64;

        let esquema = esquema_quarentena();
        let regras = StringArray::from_iter_values(std::iter::repeat_n(self.regra.as_str(), n));
        let lote = RecordBatch::try_new(
            esquema,
            vec![
                Arc::new(motivos.clone()) as _,
                Arc::new(regras) as _,
                Arc::new(manifestos.clone()) as _,
                Arc::new(partes.clone()) as _,
                Arc::new(ordinais.clone()) as _,
            ],
        )
        .expect("schema da quarentena é fixo e válido");
        self.lotes.push(lote);
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn contagens(&self) -> &HashMap<String, u64> {
        &self.contagens
    }

    pub fn esta_vazio(&self) -> bool {
        self.total == 0
    }

    pub fn para_parquet(&self) -> Result<Vec<u8>, crate::ErroDeRefino> {
        let esquema = esquema_quarentena();
        let mut escritor = EscritorParquetTipado::novo(esquema)?;
        for lote in &self.lotes {
            escritor.escrever_lote(lote)?;
        }
        escritor.finalizar()
    }
}

pub(crate) fn verificar_limites(
    contagens: &HashMap<String, u64>,
    limites: &HashMap<String, u64>,
) -> Result<(), String> {
    if limites.is_empty() && !contagens.is_empty() {
        let total: u64 = contagens.values().sum();
        let motivos: Vec<&str> = contagens.keys().map(String::as_str).collect();
        return Err(format!(
            "política falha_total: {total} linhas rejeitadas com motivo(s) {}",
            motivos.join(", ")
        ));
    }
    for (motivo, &contagem) in contagens {
        let limite = limites.get(motivo).copied().unwrap_or(0);
        if contagem > limite {
            return Err(format!(
                "rejeições por '{motivo}' ({contagem}) excedem o limite ({limite})"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn acumulador_conta_por_motivo_e_gera_parquet() {
        let mut acumulador = AcumuladorDeRejeicoes::novo("regra-v1");
        let motivos = StringArray::from(vec!["campo_ausente", "conversao_falhou", "campo_ausente"]);
        let manifestos = StringArray::from(vec!["m1", "m1", "m1"]);
        let partes = StringArray::from(vec!["p1", "p1", "p2"]);
        let ordinais = UInt64Array::from(vec![1, 2, 3]);
        acumulador.acumular(&motivos, &manifestos, &partes, &ordinais);

        assert_eq!(acumulador.total(), 3);
        assert_eq!(acumulador.contagens()["campo_ausente"], 2);
        assert_eq!(acumulador.contagens()["conversao_falhou"], 1);

        let bytes = acumulador.para_parquet().unwrap();
        assert!(!bytes.is_empty());

        let leitor = parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder::try_new(
            bytes::Bytes::from(bytes),
        )
        .unwrap()
        .build()
        .unwrap();
        let lotes: Vec<_> = leitor.map(Result::unwrap).collect();
        let total_linhas: usize = lotes.iter().map(|l| l.num_rows()).sum();
        assert_eq!(total_linhas, 3);
        let regras = lotes[0]
            .column(1)
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();
        assert!((0..regras.len()).all(|i| regras.value(i) == "regra-v1"));
    }

    #[test]
    fn limites_vazios_rejeitam_qualquer_contagem() {
        let mut contagens = HashMap::new();
        contagens.insert("campo_ausente".into(), 1u64);
        let limites = HashMap::new();
        assert!(verificar_limites(&contagens, &limites).is_err());
    }

    #[test]
    fn limites_permitem_contagem_dentro_do_limite() {
        let mut contagens = HashMap::new();
        contagens.insert("campo_ausente".into(), 5u64);
        let mut limites = HashMap::new();
        limites.insert("campo_ausente".into(), 10u64);
        assert!(verificar_limites(&contagens, &limites).is_ok());
    }

    #[test]
    fn limites_rejeitam_contagem_acima_do_limite() {
        let mut contagens = HashMap::new();
        contagens.insert("campo_ausente".into(), 11u64);
        let mut limites = HashMap::new();
        limites.insert("campo_ausente".into(), 10u64);
        assert!(verificar_limites(&contagens, &limites).is_err());
    }

    #[test]
    fn motivo_sem_limite_declarado_tem_limite_zero() {
        let mut contagens = HashMap::new();
        contagens.insert("motivo_desconhecido".into(), 1u64);
        let mut limites = HashMap::new();
        limites.insert("campo_ausente".into(), 10u64);
        let erro = verificar_limites(&contagens, &limites).unwrap_err();
        assert!(erro.contains("motivo_desconhecido"));
    }

    #[test]
    fn sem_rejeicoes_passa_com_qualquer_limite() {
        let contagens = HashMap::new();
        let limites = HashMap::new();
        assert!(verificar_limites(&contagens, &limites).is_ok());
    }
}
