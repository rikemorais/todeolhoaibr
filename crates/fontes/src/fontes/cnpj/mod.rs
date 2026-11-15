//! Cadastro Nacional da Pessoa Jurídica — dados abertos da Receita Federal.
//!
//! Snapshots mensais com empresas, estabelecimentos, sócios, Simples/MEI e
//! tabelas de apoio. CSV com separador `;`, encoding ISO-8859-1, sem cabeçalho.

mod arquivos;

use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

/// Receita Federal — Dados Públicos CNPJ.
pub struct FonteCnpj;

impl FonteDeDados for FonteCnpj {
    fn nome(&self) -> &str {
        "cnpj"
    }

    fn descricao(&self) -> &str {
        "Receita Federal — Cadastro Nacional da Pessoa Jurídica (dados abertos)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        arquivos::criar_tarefas()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn fonte_eh_sob_demanda() {
        assert!(FonteCnpj.sob_demanda());
    }
}
