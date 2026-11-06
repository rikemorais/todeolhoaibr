//! Refino do Tô De Olho Aí: decodifica a Raw opaca para o medalhão (ver ADR 0014/0015).
//!
//! A camada **bronze** pega os bytes fiéis da Raw e só os **decodifica e estrutura** em
//! Parquet, mantendo correspondência 1:1 com a origem, sem regra de negócio (isso é
//! silver). Este crate guarda o conhecimento de formato que o [ADR 0014] mantém fora da
//! ingestão.
//!
//! Os decodificadores e a [`tarefa::TarefaDeBronze`] publicam o bronze. O módulo
//! [`snapshot_bronze`] seleciona e verifica partes do manifesto vigente; o módulo
//! [`linhagem`] lê essas partes em lotes e fornece referências de linha para os
//! futuros contratos silver, sem alterar o conteúdo armazenado no bronze. O
//! [`EscritorParquetTipado`] grava os lotes Arrow produzidos por esses contratos.
//!
//! [ADR 0014]: ../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md

#![warn(missing_docs)]

pub mod catalogo;
pub mod csv_zip;
pub mod decodificador;
pub mod escritor;
pub mod fontes;
pub mod linhagem;
pub mod silver;
pub mod snapshot_bronze;
pub mod tarefa;

pub use catalogo::{FonteDeRefino, catalogo_refino};
pub use decodificador::{Decodificador, ParteBronze};
pub use escritor::{ErroDeRefino, EscritorParquet, EscritorParquetTipado, Tabela};
pub use fontes::{
    DecodificadorCsv, DecodificadorIbgeAgregados, DecodificadorJson,
    DecodificadorSenadoProcessosDetalhes, DecodificadorZipCsv,
};
pub use linhagem::{LeitorDeParteBronze, LoteBronze, OrigemParteBronze, ReferenciaLinhaBronze};
pub use silver::publicar_bcb_sgs;
pub use silver::tarefa_bcb::TarefaDeSilverBcb;
pub use snapshot_bronze::{ErroDeSnapshotBronze, ParteBronzeBaixada, SnapshotBronze};
pub use tarefa::TarefaDeBronze;
