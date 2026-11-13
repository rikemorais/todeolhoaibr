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
pub use fontes::ibge::{FonteIbge, FonteIbgePibMunicipios, FonteIbgeRendaPerCapita};
pub use fontes::ipeadata::FonteIpeadata;
pub use fontes::remuneracao_estadual_go::FonteRemuneracaoEstadualGo;
pub use fontes::remuneracao_estadual_mg::FonteRemuneracaoEstadualMg;
pub use fontes::senado::{
    FonteSenadoDespesas, FonteSenadoParlamentares, FonteSenadoProcessos,
    FonteSenadoProcessosDetalhes, FonteSenadoVotacoes,
};
pub use fontes::tce_ce::FonteRemuneracaoMunicipalTceCe;
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
        Box::new(FonteIbgePibMunicipios),
        Box::new(FonteIbgeRendaPerCapita),
        Box::new(FonteRemuneracaoEstadualMg),
        Box::new(FonteRemuneracaoEstadualGo),
        Box::new(FonteBcb),
        Box::new(FonteIpeadata),
        Box::new(FonteCamara),
        Box::new(FonteCamaraDespesas),
        Box::new(FonteCamaraTramitacoes),
        Box::new(FonteCamaraVotacoes),
        Box::new(FonteSenadoDespesas),
        Box::new(FonteSenadoVotacoes),
        Box::new(FonteSenadoProcessos),
        Box::new(FonteSenadoProcessosDetalhes),
        Box::new(FonteSenadoParlamentares),
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
