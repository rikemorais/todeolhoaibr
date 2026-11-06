//! Transformadores Silver por contrato de tabela.

pub mod bcb;
pub mod camara_deputados;
pub mod camara_legislaturas;
pub mod camara_partidos;
pub mod camara_votacoes;
pub mod ibge_agregados;
mod publicacao;
pub mod senado_parlamentares;
pub mod tarefa_bcb;
pub mod tarefa_camara;
pub mod tarefa_camara_deputados;
pub mod tarefa_camara_legislaturas;
pub mod tarefa_camara_votacoes;
pub mod tarefa_ibge;
pub mod tarefa_senado;

pub use camara_deputados::{
    ErroCamaraDeputadosSilver, esquema_camara_deputados, transformar_camara_deputados,
};
pub use camara_legislaturas::{
    ErroCamaraLegislaturasSilver, esquema_camara_legislaturas, transformar_camara_legislaturas,
};
pub use camara_partidos::{
    ErroCamaraPartidosSilver, esquema_camara_partidos, transformar_camara_partidos,
};
pub use ibge_agregados::{ErroIbgeSilver, esquema_ibge_agregados, transformar_ibge_agregados};
pub use senado_parlamentares::{
    ErroSenadoParlamentaresSilver, esquema_senado_parlamentares, transformar_senado_parlamentares,
};
pub use tarefa_bcb::publicar_bcb_sgs;
pub use tarefa_camara::{TarefaDeSilverCamaraPartidos, publicar_camara_partidos};
pub use tarefa_camara_deputados::{TarefaDeSilverCamaraDeputados, publicar_camara_deputados};
pub use tarefa_camara_legislaturas::{
    TarefaDeSilverCamaraLegislaturas, publicar_camara_legislaturas,
};
pub use tarefa_camara_votacoes::{TarefaDeSilverCamaraVotacoes, publicar_camara_votacoes};
pub use tarefa_ibge::{TarefaDeSilverIbge, publicar_ibge_agregados};
pub use tarefa_senado::{TarefaDeSilverSenadoParlamentares, publicar_senado_parlamentares};
