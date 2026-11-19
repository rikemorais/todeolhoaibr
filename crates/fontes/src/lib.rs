//! Catálogo das APIs públicas ingeridas pelo projeto.
//!
//! Para adicionar uma fonte: crie uma pasta em `fontes/` com um módulo por
//! recurso que implemente `FonteDeDados` e registre-a em [`catalogo`]. Nada
//! mais precisa mudar.

#![warn(missing_docs)]

mod fontes;
pub mod ingestao_arquivo;
pub mod ingestao_http;
pub mod paginacao;

use todeolho_nucleo::FonteDeDados;

pub use fontes::ana::FonteAna;
pub use fontes::b3::FonteB3;
pub use fontes::bcb::FonteBcb;
pub use fontes::camara::{
    FonteCamara, FonteCamaraDespesas, FonteCamaraTramitacoes, FonteCamaraVotacoes,
};
pub use fontes::cnpj::FonteCnpj;
pub use fontes::comex::FonteComex;
pub use fontes::conab::FonteConab;
pub use fontes::cvm::FonteCvm;
pub use fontes::datasus::FonteDataSus;
pub use fontes::emendas::FonteEmendas;
pub use fontes::ibama::FonteIbama;
pub use fontes::ibge::{
    FonteIbge, FonteIbgeAgropecuaria, FonteIbgeDemografiaEmpresas, FonteIbgePibMunicipios,
    FonteIbgeRendaPerCapita,
};
pub use fontes::incra::FonteIncra;
pub use fontes::inep::FonteInep;
pub use fontes::inpe::FonteInpe;
pub use fontes::ipeadata::FonteIpeadata;
pub use fontes::mapa_empresas::FonteMapaEmpresas;
pub use fontes::mapbiomas::FonteMapBiomas;
pub use fontes::previdencia::FontePrevidencia;
pub use fontes::remuneracao_estadual_go::FonteRemuneracaoEstadualGo;
pub use fontes::remuneracao_estadual_mg::FonteRemuneracaoEstadualMg;
pub use fontes::seeg::FonteSeeg;
pub use fontes::senado::{
    FonteSenadoDespesas, FonteSenadoEmendas, FonteSenadoParlamentares, FonteSenadoProcessos,
    FonteSenadoProcessosDetalhes, FonteSenadoVotacoes,
};
pub use fontes::tce_ce::FonteRemuneracaoMunicipalTceCe;
pub use fontes::trabalho::FonteTrabalho;
pub use fontes::transparencia::FonteTransparenciaBeneficios;
pub use fontes::transparencia_servidores::FonteTransparenciaServidoresAtivos;
pub use fontes::tse::{FonteTse, FonteTseCandidaturas, FonteTsePrestacaoContas};
pub use fontes::undp::FonteUndp;
pub use fontes::world_bank::FonteWorldBank;

/// Todas as fontes disponíveis. Único ponto a alterar ao adicionar uma nova.
///
/// # Exemplos
///
/// ```
/// let fontes = todeolho_fontes::catalogo();
/// assert!(!fontes.is_empty());
/// ```
pub fn catalogo() -> Vec<Box<dyn FonteDeDados>> {
    vec![
        Box::new(FonteAna),
        Box::new(FonteB3),
        Box::new(FonteIbge),
        Box::new(FonteIbgeDemografiaEmpresas),
        Box::new(FonteIbgeAgropecuaria),
        Box::new(FonteIbgePibMunicipios),
        Box::new(FonteIbgeRendaPerCapita),
        Box::new(FonteMapaEmpresas),
        Box::new(FonteRemuneracaoEstadualMg),
        Box::new(FonteRemuneracaoEstadualGo),
        Box::new(FonteBcb),
        Box::new(FonteCnpj),
        Box::new(FonteComex),
        Box::new(FonteConab),
        Box::new(FonteIbama),
        Box::new(FonteIncra),
        Box::new(FonteInpe),
        Box::new(FonteMapBiomas),
        Box::new(FonteCvm),
        Box::new(FonteEmendas),
        Box::new(FonteIpeadata),
        Box::new(FonteCamara),
        Box::new(FonteCamaraDespesas),
        Box::new(FonteCamaraTramitacoes),
        Box::new(FonteCamaraVotacoes),
        Box::new(FonteSenadoDespesas),
        Box::new(FonteSenadoEmendas),
        Box::new(FonteSenadoVotacoes),
        Box::new(FonteSenadoProcessos),
        Box::new(FonteSenadoProcessosDetalhes),
        Box::new(FonteSenadoParlamentares),
        Box::new(FonteSeeg),
        Box::new(FonteTrabalho),
        Box::new(FontePrevidencia),
        Box::new(FonteInep),
        Box::new(FonteDataSus),
        Box::new(FonteRemuneracaoMunicipalTceCe),
        Box::new(FonteTransparenciaBeneficios),
        Box::new(FonteTransparenciaServidoresAtivos),
        Box::new(FonteTse),
        Box::new(FonteTsePrestacaoContas),
        Box::new(FonteTseCandidaturas),
        Box::new(FonteUndp),
        Box::new(FonteWorldBank),
    ]
}

#[cfg(test)]
mod testes {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn toda_fonte_monta_uma_dag_valida() {
        for fonte in catalogo() {
            let dag = fonte.montar_dag().unwrap_or_else(|erro| panic!("{erro}"));
            assert!(!dag.niveis().unwrap().is_empty(), "{}", fonte.nome());
        }
    }

    #[test]
    fn nomes_das_fontes_sao_unicos() {
        let fontes = catalogo();
        let nomes: HashSet<_> = fontes.iter().map(|f| f.nome()).collect();
        assert_eq!(nomes.len(), fontes.len());
    }
}
