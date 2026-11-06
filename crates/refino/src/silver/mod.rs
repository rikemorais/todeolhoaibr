//! Transformadores Silver por contrato de tabela.

pub mod bcb;
pub mod ibge_agregados;
mod publicacao;
pub mod senado_parlamentares;
pub mod tarefa_bcb;
pub mod tarefa_ibge;
pub mod tarefa_senado;

pub use ibge_agregados::{ErroIbgeSilver, esquema_ibge_agregados, transformar_ibge_agregados};
pub use senado_parlamentares::{
    ErroSenadoParlamentaresSilver, esquema_senado_parlamentares, transformar_senado_parlamentares,
};
pub use tarefa_bcb::publicar_bcb_sgs;
pub use tarefa_ibge::{TarefaDeSilverIbge, publicar_ibge_agregados};
pub use tarefa_senado::{TarefaDeSilverSenadoParlamentares, publicar_senado_parlamentares};
