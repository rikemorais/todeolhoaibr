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
pub use silver::camara_deputados::{
    ErroCamaraDeputadosSilver, esquema_camara_deputados, transformar_camara_deputados,
};
pub use silver::camara_despesas::{
    ErroCamaraDespesasSilver, esquema_camara_despesas, transformar_camara_despesas,
};
pub use silver::camara_legislaturas::{
    ErroCamaraLegislaturasSilver, esquema_camara_legislaturas, transformar_camara_legislaturas,
};
pub use silver::camara_partidos::{
    ErroCamaraPartidosSilver, esquema_camara_partidos, transformar_camara_partidos,
};
pub use silver::camara_tramitacoes::{
    ErroCamaraTramitacoesSilver, ano_do_recurso, esquema_camara_tramitacoes,
    transformar_camara_tramitacoes,
};
pub use silver::camara_votacoes::{
    ConjuntoCamaraVotacoes, ErroCamaraVotacoesSilver, validar_camara_votacoes,
};
pub use silver::publicar_bcb_sgs;
pub use silver::senado_despesas::{
    ErroSenadoDespesasSilver, esquema_senado_despesas, transformar_senado_despesas,
};
pub use silver::senado_parlamentares::{
    ErroSenadoParlamentaresSilver, esquema_senado_parlamentares, transformar_senado_parlamentares,
};
pub use silver::senado_processos::{
    ErroSenadoProcessosSilver, esquema_senado_processos, transformar_senado_processos,
};
pub use silver::senado_processos_detalhes::{
    ErroSenadoProcessosDetalhesSilver, esquema_senado_processos_detalhes,
    transformar_senado_processos_detalhes,
};
pub use silver::senado_votacoes::{ErroSenadoVotacoesSilver, transformar_senado_votacoes};
pub use silver::senado_votos::{
    ErroSenadoVotosSilver, esquema_senado_votos, transformar_senado_votos,
};
pub use silver::tarefa_bcb::TarefaDeSilverBcb;
pub use silver::tse_bens::{ErroTseBensSilver, esquema_tse_bens, transformar_tse_bens};
pub use silver::tse_candidaturas::{
    ErroTseCandidaturasSilver, esquema_tse_candidaturas, transformar_tse_candidaturas,
};
pub use silver::tse_despesas_pagas::{
    ErroTseDespesasPagasSilver, EstadoDeChavesDeDespesasPagas, esquema_tse_despesas_pagas_2024,
    transformar_tse_despesas_pagas_2024,
};
pub use silver::tse_votacao_secao::{
    ErroTseVotacaoSecaoSilver, esquema_tse_votacao_secao, transformar_tse_votacao_secao,
};
pub use silver::{TarefaDeSilverCamaraDeputados, publicar_camara_deputados};
pub use silver::{TarefaDeSilverCamaraDespesas, publicar_camara_despesas};
pub use silver::{TarefaDeSilverCamaraLegislaturas, publicar_camara_legislaturas};
pub use silver::{TarefaDeSilverCamaraPartidos, publicar_camara_partidos};
pub use silver::{TarefaDeSilverCamaraTramitacoes, publicar_camara_tramitacoes};
pub use silver::{TarefaDeSilverCamaraVotacoes, publicar_camara_votacoes};
pub use silver::{TarefaDeSilverIbge, publicar_ibge_agregados};
pub use silver::{TarefaDeSilverSenadoDespesas, publicar_senado_despesas};
pub use silver::{TarefaDeSilverSenadoParlamentares, publicar_senado_parlamentares};
pub use silver::{TarefaDeSilverSenadoProcessos, publicar_senado_processos};
pub use silver::{TarefaDeSilverSenadoProcessosDetalhes, publicar_senado_processos_detalhes};
pub use silver::{TarefaDeSilverSenadoVotacoes, publicar_senado_votacoes};
pub use silver::{TarefaDeSilverTseBens, publicar_tse_bens};
pub use silver::{TarefaDeSilverTseCandidaturas, publicar_tse_candidaturas};
pub use silver::{TarefaDeSilverTseDespesasPagas, publicar_tse_despesas_pagas_2024};
pub use silver::{TarefaDeSilverTseVotacaoSecao, publicar_tse_votacao_secao};
pub use snapshot_bronze::{ErroDeSnapshotBronze, ParteBronzeBaixada, SnapshotBronze};
pub use tarefa::TarefaDeBronze;
