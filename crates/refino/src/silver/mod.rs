//! Transformadores Silver por contrato de tabela.

pub mod bcb;
pub mod ibge_agregados;
pub mod tarefa_bcb;
pub mod tarefa_ibge;

pub use ibge_agregados::{ErroIbgeSilver, esquema_ibge_agregados, transformar_ibge_agregados};
pub use tarefa_bcb::publicar_bcb_sgs;
pub use tarefa_ibge::{TarefaDeSilverIbge, publicar_ibge_agregados};
