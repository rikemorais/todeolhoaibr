//! O contrato que cada fonte implementa para traduzir seus bytes do raw em Parquet.
//!
//! O [`Decodificador`] é o único lugar que conhece o **formato** de uma fonte (ZIP, CSV
//! em Latin-1, JSON de API) — o conhecimento que o [ADR 0014] mantém fora da ingestão.
//! Ele devolve o Parquet **já pronto** (uma [`ParteBronze`] por tabela), para poder
//! escrever em lotes e não manter arquivos grandes inteiros em memória.
//!
//! [ADR 0014]: ../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md

use crate::escritor::{ErroDeRefino, Tabela};

/// Uma tabela do bronze já serializada em Parquet, com o nome lógico que terá.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParteBronze {
    /// Nome lógico (ex.: o CSV interno de um ZIP; vira parte da procedência).
    pub nome: String,
    /// Bytes do Parquet.
    pub parquet: Vec<u8>,
}

impl ParteBronze {
    /// Cria a parte a partir de bytes de Parquet já prontos.
    pub fn nova(nome: impl Into<String>, parquet: Vec<u8>) -> Self {
        Self {
            nome: nome.into(),
            parquet,
        }
    }

    /// Conveniência para decodificadores que montam uma [`Tabela`] pequena (ex.: JSON):
    /// serializa a tabela em Parquet e empacota como parte.
    pub fn de_tabela(nome: impl Into<String>, tabela: &Tabela) -> Result<Self, ErroDeRefino> {
        Ok(Self::nova(nome, tabela.para_parquet()?))
    }
}

/// Traduz os bytes de **um part** do raw em uma ou mais partes de bronze (Parquet).
///
/// Um recurso de API costuma render uma parte; um ZIP com vários CSVs rende uma por CSV.
pub trait Decodificador: Send + Sync {
    /// Decodifica os bytes de um part do raw, usando o `tipo_de_conteudo` registrado no
    /// manifesto quando precisar distinguir formatos.
    fn decodificar(
        &self,
        bytes: &[u8],
        tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino>;
}
