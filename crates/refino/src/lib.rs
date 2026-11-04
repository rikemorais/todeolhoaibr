//! Refino do Tô De Olho Aí: decodifica a Raw opaca para o medalhão (ver ADR 0014/0015).
//!
//! A camada **bronze** pega os bytes fiéis da Raw e só os **decodifica e estrutura** em
//! Parquet, mantendo correspondência 1:1 com a origem, sem regra de negócio (isso é
//! silver). Este crate guarda o conhecimento de formato que o [ADR 0014] mantém fora da
//! ingestão.
//!
//! Nesta primeira fase (issue #152) o crate oferece o [`escritor`] de Parquet; os
//! decodificadores por fonte e a `TarefaDeBronze` vêm nas fases seguintes.
//!
//! [ADR 0014]: ../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md

#![warn(missing_docs)]

pub mod catalogo;
pub mod csv_zip;
pub mod decodificador;
pub mod escritor;
pub mod fontes;
pub mod tarefa;

pub use catalogo::{FonteDeRefino, catalogo_refino};
pub use decodificador::{Decodificador, ParteBronze};
pub use escritor::{ErroDeRefino, EscritorParquet, Tabela};
pub use fontes::{DecodificadorCsv, DecodificadorJson, DecodificadorZipCsv};
pub use tarefa::TarefaDeBronze;
