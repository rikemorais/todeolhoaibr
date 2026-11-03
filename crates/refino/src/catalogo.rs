//! Catálogo de refino: liga cada fonte do bronze ao seu decodificador e ao recurso no raw.
//!
//! Espelha o papel do catálogo da ingestão (`fontes::catalogo`), mas para a decodificação:
//! diz de qual fonte/prefixo do raw cada refino lê e qual [`Decodificador`] usa.

use std::sync::Arc;

use crate::decodificador::Decodificador;
use crate::fontes::DecodificadorCamaraDespesas;

/// Uma fonte do bronze: o nome no catálogo, de onde lê no raw e como decodifica.
pub struct FonteDeRefino {
    /// Nome usado na CLI e no painel (ex.: `camara-despesas`).
    pub nome: &'static str,
    /// Fonte no raw de onde os recursos são lidos (ex.: `camara`).
    pub fonte_raw: &'static str,
    /// Prefixo dos recursos desta fonte dentro do raw (ex.: `despesas/`).
    pub prefixo_recurso: &'static str,
    cria_decodificador: fn() -> Arc<dyn Decodificador>,
}

impl FonteDeRefino {
    /// Cria o decodificador desta fonte.
    pub fn decodificador(&self) -> Arc<dyn Decodificador> {
        (self.cria_decodificador)()
    }
}

/// Fontes do bronze disponíveis. Cresce conforme novos decodificadores entram.
pub fn catalogo_refino() -> Vec<FonteDeRefino> {
    vec![FonteDeRefino {
        nome: "camara-despesas",
        fonte_raw: "camara",
        prefixo_recurso: "despesas/",
        cria_decodificador: || Arc::new(DecodificadorCamaraDespesas),
    }]
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_catalogo_tem_a_camara_despesas_apontando_para_o_raw_camara() {
        let fonte = catalogo_refino()
            .into_iter()
            .find(|f| f.nome == "camara-despesas")
            .expect("camara-despesas no catálogo");
        assert_eq!(fonte.fonte_raw, "camara");
        assert_eq!(fonte.prefixo_recurso, "despesas/");
        // O decodificador é construível.
        let _ = fonte.decodificador();
    }
}
