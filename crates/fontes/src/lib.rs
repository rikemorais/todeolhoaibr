//! Catálogo das APIs públicas ingeridas pelo projeto.
//!
//! Para adicionar uma fonte: crie um módulo em `fontes/` que implemente
//! `FonteDeDados` e registre-a em [`catalogo`]. Nada mais precisa mudar.

#![warn(missing_docs)]

mod fontes;
pub mod ingestao_arquivo;
pub mod ingestao_http;
pub mod paginacao;

use todeolho_nucleo::FonteDeDados;

pub use fontes::camara::FonteCamara;
pub use fontes::ibge::FonteIbge;
pub use fontes::tse::FonteTse;

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
        Box::new(FonteIbge),
        Box::new(FonteCamara),
        Box::new(FonteTse),
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
