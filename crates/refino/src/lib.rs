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
pub mod executor_sql;
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
pub use silver::TarefaDeSilverIpeadata;
pub use silver::TarefaDeSilverSql;
pub use silver::camara_deputados::esquema_camara_deputados;
#[cfg(test)]
pub use silver::camara_deputados::{ErroCamaraDeputadosSilver, transformar_camara_deputados};
pub use silver::camara_despesas::esquema_camara_despesas;
#[cfg(test)]
pub use silver::camara_despesas::{ErroCamaraDespesasSilver, transformar_camara_despesas};
pub use silver::camara_legislaturas::esquema_camara_legislaturas;
#[cfg(test)]
pub use silver::camara_legislaturas::{
    ErroCamaraLegislaturasSilver, transformar_camara_legislaturas,
};
pub use silver::camara_partidos::esquema_camara_partidos;
#[cfg(test)]
pub use silver::camara_partidos::{ErroCamaraPartidosSilver, transformar_camara_partidos};
#[cfg(test)]
pub use silver::camara_tramitacoes::{ErroCamaraTramitacoesSilver, transformar_camara_tramitacoes};
pub use silver::camara_tramitacoes::{ano_do_recurso, esquema_camara_tramitacoes};
pub use silver::camara_votacoes::ConjuntoCamaraVotacoes;
#[cfg(test)]
pub use silver::camara_votacoes::{ErroCamaraVotacoesSilver, validar_camara_votacoes};
pub use silver::publicar_bcb_sgs;
pub use silver::senado_despesas::esquema_senado_despesas;
#[cfg(test)]
pub use silver::senado_despesas::{ErroSenadoDespesasSilver, transformar_senado_despesas};
pub use silver::senado_parlamentares::esquema_senado_parlamentares;
#[cfg(test)]
pub use silver::senado_parlamentares::{
    ErroSenadoParlamentaresSilver, transformar_senado_parlamentares,
};
pub use silver::senado_processos::esquema_senado_processos;
#[cfg(test)]
pub use silver::senado_processos::{ErroSenadoProcessosSilver, transformar_senado_processos};
pub use silver::senado_processos_detalhes::esquema_senado_processos_detalhes;
#[cfg(test)]
pub use silver::senado_processos_detalhes::{
    ErroSenadoProcessosDetalhesSilver, transformar_senado_processos_detalhes,
};
#[cfg(test)]
pub use silver::senado_votacoes::{ErroSenadoVotacoesSilver, transformar_senado_votacoes};
pub use silver::senado_votos::esquema_senado_votos;
#[cfg(test)]
pub use silver::senado_votos::{ErroSenadoVotosSilver, transformar_senado_votos};
pub use silver::tarefa_bcb::TarefaDeSilverBcb;
pub use silver::tarefa_tse_despesas_candidatos_2016::{
    TarefaDeSilverTseDespesasCandidatos2016, publicar_tse_despesas_candidatos_2016,
};
pub use silver::tarefa_tse_despesas_contratadas::{
    TarefaDeSilverTseDespesasContratadas, publicar_tse_despesas_contratadas_2024,
};
pub use silver::tarefa_tse_despesas_contratadas_2018::{
    TarefaDeSilverTseDespesasContratadas2018, publicar_tse_despesas_contratadas_2018,
};
pub use silver::tarefa_tse_despesas_contratadas_2022::{
    TarefaDeSilverTseDespesasContratadas2022, publicar_tse_despesas_contratadas_2022,
};
pub use silver::tarefa_tse_receitas_candidatos::{
    TarefaDeSilverTseReceitasCandidatos, publicar_tse_receitas_candidatos_2024,
};
pub use silver::tarefa_tse_receitas_candidatos_2018::{
    TarefaDeSilverTseReceitasCandidatos2018, publicar_tse_receitas_candidatos_2018,
};
pub use silver::tarefa_tse_receitas_candidatos_2020::{
    TarefaDeSilverTseReceitasCandidatos2020, publicar_tse_receitas_candidatos_2020,
};
pub use silver::tarefa_tse_receitas_candidatos_2022::{
    TarefaDeSilverTseReceitasCandidatos2022, publicar_tse_receitas_candidatos_2022,
};
pub use silver::tarefa_tse_receitas_doador_originario::{
    TarefaDeSilverTseReceitasDoadorOriginario, publicar_tse_receitas_doador_originario_2024,
};
pub use silver::tarefa_tse_receitas_doador_originario_2018::{
    TarefaDeSilverTseReceitasDoadorOriginario2018, publicar_tse_receitas_doador_originario_2018,
};
pub use silver::tarefa_tse_receitas_doador_originario_2020::{
    TarefaDeSilverTseReceitasDoadorOriginario2020, publicar_tse_receitas_doador_originario_2020,
};
pub use silver::tarefa_tse_receitas_doador_originario_2022::{
    TarefaDeSilverTseReceitasDoadorOriginario2022, publicar_tse_receitas_doador_originario_2022,
};
pub use silver::tse_bens::esquema_tse_bens;
#[cfg(test)]
pub use silver::tse_bens::{ErroTseBensSilver, transformar_tse_bens};
pub use silver::tse_candidaturas::esquema_tse_candidaturas;
#[cfg(test)]
pub use silver::tse_candidaturas::{ErroTseCandidaturasSilver, transformar_tse_candidaturas};
pub use silver::tse_despesas_contratadas::esquema_tse_despesas_contratadas_2024;
pub use silver::tse_despesas_contratadas_2018::esquema_tse_despesas_contratadas_2018;
pub use silver::tse_despesas_contratadas_2020::esquema_tse_despesas_contratadas_2020;
pub use silver::tse_despesas_contratadas_2022::esquema_tse_despesas_contratadas_2022;
pub use silver::tse_despesas_pagas::esquema_tse_despesas_pagas_2024;
#[cfg(test)]
pub use silver::tse_despesas_pagas::{
    ErroTseDespesasPagasSilver, EstadoDeChavesDeDespesasPagas, transformar_tse_despesas_pagas_2024,
};
pub use silver::tse_despesas_pagas_2018::esquema_tse_despesas_pagas_2018;
pub use silver::tse_despesas_pagas_2020::esquema_tse_despesas_pagas_2020;
pub use silver::tse_despesas_pagas_2022::esquema_tse_despesas_pagas_2022;
pub use silver::tse_receitas_candidatos::esquema_tse_receitas_candidatos_2024;
pub use silver::tse_receitas_candidatos_2018::esquema_tse_receitas_candidatos_2018;
pub use silver::tse_receitas_candidatos_2020::esquema_tse_receitas_candidatos_2020;
pub use silver::tse_receitas_candidatos_2022::esquema_tse_receitas_candidatos_2022;
pub use silver::tse_receitas_doador_originario::esquema_tse_receitas_doador_originario_2024;
pub use silver::tse_receitas_doador_originario_2018::esquema_tse_receitas_doador_originario_2018;
pub use silver::tse_receitas_doador_originario_2020::esquema_tse_receitas_doador_originario_2020;
pub use silver::tse_receitas_doador_originario_2022::esquema_tse_receitas_doador_originario_2022;
pub use silver::tse_votacao_secao::esquema_tse_votacao_secao;
#[cfg(test)]
pub use silver::tse_votacao_secao::{ErroTseVotacaoSecaoSilver, transformar_tse_votacao_secao};
pub use silver::{TarefaDeSilverCamaraDeputados, publicar_camara_deputados};
pub use silver::{TarefaDeSilverCamaraDespesas, publicar_camara_despesas};
pub use silver::{TarefaDeSilverCamaraLegislaturas, publicar_camara_legislaturas};
pub use silver::{TarefaDeSilverCamaraPartidos, publicar_camara_partidos};
pub use silver::{TarefaDeSilverCamaraTramitacoes, publicar_camara_tramitacoes};
pub use silver::{TarefaDeSilverCamaraVotacoes, publicar_camara_votacoes};
pub use silver::{TarefaDeSilverIbge, publicar_ibge_agregados};
pub use silver::{TarefaDeSilverIbgeLocalidades, TipoLocalidadeIbge};
pub use silver::{TarefaDeSilverSenadoDespesas, publicar_senado_despesas};
pub use silver::{TarefaDeSilverSenadoParlamentares, publicar_senado_parlamentares};
pub use silver::{TarefaDeSilverSenadoProcessos, publicar_senado_processos};
pub use silver::{TarefaDeSilverSenadoProcessosDetalhes, publicar_senado_processos_detalhes};
pub use silver::{TarefaDeSilverSenadoVotacoes, publicar_senado_votacoes};
pub use silver::{TarefaDeSilverTseBens, publicar_tse_bens};
pub use silver::{TarefaDeSilverTseCandidaturas, publicar_tse_candidaturas};
pub use silver::{
    TarefaDeSilverTseDespesasContratadas2020, publicar_tse_despesas_contratadas_2020,
};
pub use silver::{TarefaDeSilverTseDespesasPagas, publicar_tse_despesas_pagas_2024};
pub use silver::{TarefaDeSilverTseDespesasPagas2018, publicar_tse_despesas_pagas_2018};
pub use silver::{TarefaDeSilverTseDespesasPagas2020, publicar_tse_despesas_pagas_2020};
pub use silver::{TarefaDeSilverTseDespesasPagas2022, publicar_tse_despesas_pagas_2022};
pub use silver::{TarefaDeSilverTseVotacaoSecao, publicar_tse_votacao_secao};
pub use snapshot_bronze::{ErroDeSnapshotBronze, ParteBronzeBaixada, SnapshotBronze};
pub use tarefa::TarefaDeBronze;
