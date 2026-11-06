//! Transformadores Silver por contrato de tabela.

pub mod bcb;
pub mod camara_deputados;
pub mod camara_despesas;
pub mod camara_legislaturas;
pub mod camara_partidos;
pub mod camara_tramitacoes;
pub mod camara_votacoes;
pub mod ibge_agregados;
mod publicacao;
pub mod senado_parlamentares;
/// Contrato Silver para o catálogo anual de processos legislativos do Senado.
pub mod senado_processos;
/// Contratos Silver para detalhes e histórico dos processos legislativos do Senado.
pub mod senado_processos_detalhes;
pub mod senado_votacoes;
pub mod senado_votos;
pub mod tarefa_bcb;
pub mod tarefa_camara;
pub mod tarefa_camara_deputados;
pub mod tarefa_camara_despesas;
pub mod tarefa_camara_legislaturas;
pub mod tarefa_camara_tramitacoes;
pub mod tarefa_camara_votacoes;
pub mod tarefa_ibge;
pub mod tarefa_senado;
mod tarefa_senado_processos;
mod tarefa_senado_processos_detalhes;
pub mod tarefa_senado_votacoes;

pub use camara_deputados::{
    ErroCamaraDeputadosSilver, esquema_camara_deputados, transformar_camara_deputados,
};
pub use camara_despesas::{
    ErroCamaraDespesasSilver, esquema_camara_despesas, transformar_camara_despesas,
};
pub use camara_legislaturas::{
    ErroCamaraLegislaturasSilver, esquema_camara_legislaturas, transformar_camara_legislaturas,
};
pub use camara_partidos::{
    ErroCamaraPartidosSilver, esquema_camara_partidos, transformar_camara_partidos,
};
pub use camara_tramitacoes::{
    ErroCamaraTramitacoesSilver, ano_do_recurso, esquema_camara_tramitacoes,
    transformar_camara_tramitacoes,
};
pub use ibge_agregados::{ErroIbgeSilver, esquema_ibge_agregados, transformar_ibge_agregados};
pub use senado_parlamentares::{
    ErroSenadoParlamentaresSilver, esquema_senado_parlamentares, transformar_senado_parlamentares,
};
pub use senado_processos::{
    ErroSenadoProcessosSilver, esquema_senado_processos, transformar_senado_processos,
};
pub use senado_processos_detalhes::{
    ErroSenadoProcessosDetalhesSilver, esquema_senado_processos_detalhes,
    transformar_senado_processos_detalhes,
};
pub use senado_votacoes::{ErroSenadoVotacoesSilver, transformar_senado_votacoes};
pub use senado_votos::{ErroSenadoVotosSilver, esquema_senado_votos, transformar_senado_votos};
pub use tarefa_bcb::publicar_bcb_sgs;
pub use tarefa_camara::{TarefaDeSilverCamaraPartidos, publicar_camara_partidos};
pub use tarefa_camara_deputados::{TarefaDeSilverCamaraDeputados, publicar_camara_deputados};
pub use tarefa_camara_despesas::{TarefaDeSilverCamaraDespesas, publicar_camara_despesas};
pub use tarefa_camara_legislaturas::{
    TarefaDeSilverCamaraLegislaturas, publicar_camara_legislaturas,
};
pub use tarefa_camara_tramitacoes::{TarefaDeSilverCamaraTramitacoes, publicar_camara_tramitacoes};
pub use tarefa_camara_votacoes::{TarefaDeSilverCamaraVotacoes, publicar_camara_votacoes};
pub use tarefa_ibge::{TarefaDeSilverIbge, publicar_ibge_agregados};
pub use tarefa_senado::{TarefaDeSilverSenadoParlamentares, publicar_senado_parlamentares};
pub use tarefa_senado_processos::{TarefaDeSilverSenadoProcessos, publicar_senado_processos};
pub use tarefa_senado_processos_detalhes::{
    TarefaDeSilverSenadoProcessosDetalhes, publicar_senado_processos_detalhes,
};
pub use tarefa_senado_votacoes::{TarefaDeSilverSenadoVotacoes, publicar_senado_votacoes};
