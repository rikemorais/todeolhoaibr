//! Transformadores Silver por contrato de tabela.

pub mod bcb;
pub mod camara_deputados;
pub mod camara_despesas;
pub mod camara_legislaturas;
pub mod camara_partidos;
pub mod camara_tramitacoes;
pub mod camara_votacoes;
pub mod ibge_agregados;
/// Esquema Silver para as unidades federativas do IBGE.
pub mod ibge_estados;
/// Esquema Silver para os municípios do IBGE.
pub mod ibge_municipios;
/// Esquema Silver para as regiões do IBGE.
pub mod ibge_regioes;
/// Esquema Silver para as séries históricas do Ipeadata.
pub mod ipeadata;
mod publicacao;
pub mod senado_despesas;
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
/// Tarefas que publicam localidades do IBGE na camada Silver.
pub mod tarefa_ibge_localidades;
/// Tarefa que publica séries do Ipeadata na camada Silver.
pub mod tarefa_ipeadata;
pub mod tarefa_senado;
pub mod tarefa_senado_despesas;
mod tarefa_senado_processos;
mod tarefa_senado_processos_detalhes;
pub mod tarefa_senado_votacoes;
pub mod tarefa_sql;
pub mod tarefa_tse_bens;
pub mod tarefa_tse_candidaturas;
pub mod tarefa_tse_despesas_contratadas;
pub mod tarefa_tse_despesas_pagas;
pub mod tarefa_tse_receitas_candidatos;
pub mod tarefa_tse_receitas_doador_originario;
pub mod tarefa_tse_receitas_doador_originario_2022;
pub mod tarefa_tse_votacao_secao;
pub mod tse_bens;
pub mod tse_candidaturas;
pub mod tse_despesas_contratadas;
pub mod tse_despesas_pagas;
pub mod tse_receitas_candidatos;
pub mod tse_receitas_doador_originario;
pub mod tse_receitas_doador_originario_2022;
pub mod tse_votacao_secao;

pub use camara_deputados::esquema_camara_deputados;
#[cfg(test)]
pub use camara_deputados::{ErroCamaraDeputadosSilver, transformar_camara_deputados};
pub use camara_despesas::esquema_camara_despesas;
#[cfg(test)]
pub use camara_despesas::{ErroCamaraDespesasSilver, transformar_camara_despesas};
pub use camara_legislaturas::esquema_camara_legislaturas;
#[cfg(test)]
pub use camara_legislaturas::{ErroCamaraLegislaturasSilver, transformar_camara_legislaturas};
pub use camara_partidos::esquema_camara_partidos;
#[cfg(test)]
pub use camara_partidos::{ErroCamaraPartidosSilver, transformar_camara_partidos};
#[cfg(test)]
pub use camara_tramitacoes::{ErroCamaraTramitacoesSilver, transformar_camara_tramitacoes};
pub use camara_tramitacoes::{ano_do_recurso, esquema_camara_tramitacoes};
pub use ibge_agregados::esquema_ibge_agregados;
#[cfg(test)]
pub use ibge_agregados::{ErroIbgeSilver, transformar_ibge_agregados};
pub use ibge_estados::esquema_ibge_estados;
pub use ibge_municipios::esquema_ibge_municipios;
pub use ibge_regioes::esquema_ibge_regioes;
pub use ipeadata::esquema_ipeadata;
pub use senado_despesas::esquema_senado_despesas;
#[cfg(test)]
pub use senado_despesas::{ErroSenadoDespesasSilver, transformar_senado_despesas};
pub use senado_parlamentares::esquema_senado_parlamentares;
#[cfg(test)]
pub use senado_parlamentares::{ErroSenadoParlamentaresSilver, transformar_senado_parlamentares};
pub use senado_processos::esquema_senado_processos;
#[cfg(test)]
pub use senado_processos::{ErroSenadoProcessosSilver, transformar_senado_processos};
pub use senado_processos_detalhes::esquema_senado_processos_detalhes;
#[cfg(test)]
pub use senado_processos_detalhes::{
    ErroSenadoProcessosDetalhesSilver, transformar_senado_processos_detalhes,
};
#[cfg(test)]
pub use senado_votacoes::{ErroSenadoVotacoesSilver, transformar_senado_votacoes};
pub use senado_votos::esquema_senado_votos;
#[cfg(test)]
pub use senado_votos::{ErroSenadoVotosSilver, transformar_senado_votos};
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
pub use tarefa_ibge_localidades::{TarefaDeSilverIbgeLocalidades, TipoLocalidadeIbge};
pub use tarefa_ipeadata::TarefaDeSilverIpeadata;
pub use tarefa_senado::{TarefaDeSilverSenadoParlamentares, publicar_senado_parlamentares};
pub use tarefa_senado_despesas::{TarefaDeSilverSenadoDespesas, publicar_senado_despesas};
pub use tarefa_senado_processos::{TarefaDeSilverSenadoProcessos, publicar_senado_processos};
pub use tarefa_senado_processos_detalhes::{
    TarefaDeSilverSenadoProcessosDetalhes, publicar_senado_processos_detalhes,
};
pub use tarefa_senado_votacoes::{TarefaDeSilverSenadoVotacoes, publicar_senado_votacoes};
pub use tarefa_sql::TarefaDeSilverSql;
pub use tarefa_tse_bens::{TarefaDeSilverTseBens, publicar_tse_bens};
pub use tarefa_tse_candidaturas::{TarefaDeSilverTseCandidaturas, publicar_tse_candidaturas};
pub use tarefa_tse_despesas_contratadas::{
    TarefaDeSilverTseDespesasContratadas, publicar_tse_despesas_contratadas_2024,
};
pub use tarefa_tse_despesas_pagas::{
    TarefaDeSilverTseDespesasPagas, publicar_tse_despesas_pagas_2024,
};
pub use tarefa_tse_receitas_candidatos::{
    TarefaDeSilverTseReceitasCandidatos, publicar_tse_receitas_candidatos_2024,
};
pub use tarefa_tse_receitas_doador_originario::{
    TarefaDeSilverTseReceitasDoadorOriginario, publicar_tse_receitas_doador_originario_2024,
};
pub use tarefa_tse_receitas_doador_originario_2022::{
    TarefaDeSilverTseReceitasDoadorOriginario2022, publicar_tse_receitas_doador_originario_2022,
};
pub use tarefa_tse_votacao_secao::{TarefaDeSilverTseVotacaoSecao, publicar_tse_votacao_secao};
pub use tse_bens::esquema_tse_bens;
#[cfg(test)]
pub use tse_bens::{ErroTseBensSilver, transformar_tse_bens};
pub use tse_candidaturas::esquema_tse_candidaturas;
#[cfg(test)]
pub use tse_candidaturas::{ErroTseCandidaturasSilver, transformar_tse_candidaturas};
pub use tse_despesas_contratadas::esquema_tse_despesas_contratadas_2024;
pub use tse_despesas_pagas::esquema_tse_despesas_pagas_2024;
#[cfg(test)]
pub use tse_despesas_pagas::{
    ErroTseDespesasPagasSilver, EstadoDeChavesDeDespesasPagas, transformar_tse_despesas_pagas_2024,
};
pub use tse_receitas_candidatos::esquema_tse_receitas_candidatos_2024;
pub use tse_receitas_doador_originario::esquema_tse_receitas_doador_originario_2024;
pub use tse_receitas_doador_originario_2022::esquema_tse_receitas_doador_originario_2022;
pub use tse_votacao_secao::esquema_tse_votacao_secao;
#[cfg(test)]
pub use tse_votacao_secao::{ErroTseVotacaoSecaoSilver, transformar_tse_votacao_secao};
