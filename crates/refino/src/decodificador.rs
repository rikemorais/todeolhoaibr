//! O contrato que cada fonte implementa para traduzir seus bytes do raw em tabelas.
//!
//! O [`Decodificador`] é o único lugar que conhece o **formato** de uma fonte (ZIP, CSV
//! em Latin-1, JSON de API) — o conhecimento que o [ADR 0014] mantém fora da ingestão.
//! Ele não grava nada: só transforma bytes em [`Tabela`]s; a `TarefaDeBronze` cuida de
//! ler o raw e gravar o Parquet.
//!
//! [ADR 0014]: ../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md

use crate::escritor::{ErroDeRefino, Tabela};

/// Uma tabela com o nome lógico que ela terá no bronze (ex.: o CSV interno de um ZIP).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabelaNomeada {
    /// Nome lógico (vira parte da procedência; um CSV por tabela, por exemplo).
    pub nome: String,
    /// A tabela colunar em si.
    pub tabela: Tabela,
}

impl TabelaNomeada {
    /// Cria uma tabela nomeada.
    pub fn nova(nome: impl Into<String>, tabela: Tabela) -> Self {
        Self {
            nome: nome.into(),
            tabela,
        }
    }
}

/// Traduz os bytes de **um part** do raw em uma ou mais tabelas.
///
/// Um recurso de API costuma render uma tabela; um ZIP com vários CSVs rende uma por CSV.
pub trait Decodificador: Send + Sync {
    /// Decodifica os bytes de um part do raw, usando o `tipo_de_conteudo` registrado no
    /// manifesto quando precisar distinguir formatos.
    fn decodificar(
        &self,
        bytes: &[u8],
        tipo_de_conteudo: &str,
    ) -> Result<Vec<TabelaNomeada>, ErroDeRefino>;
}
