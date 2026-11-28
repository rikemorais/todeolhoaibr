//! Decodificador que detecta automaticamente se o part é um ZIP (com CSVs dentro)
//! ou um CSV solto. Serve para fontes como IBAMA que misturam os dois formatos.

use crate::csv_zip::{partes_de_csv, partes_de_zip_csv_em_fluxo};
use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::ErroDeRefino;

/// Detecta automaticamente se o part é ZIP (magic bytes `PK`) ou CSV solto.
pub struct DecodificadorCsvOuZip {
    nome: &'static str,
    separador: u8,
}

impl DecodificadorCsvOuZip {
    /// Cria o decodificador com nome lógico e separador de CSV.
    pub const fn novo(nome: &'static str, separador: u8) -> Self {
        Self { nome, separador }
    }
}

impl Decodificador for DecodificadorCsvOuZip {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        if bytes.starts_with(b"PK") {
            partes_de_zip_csv_em_fluxo(std::io::Cursor::new(bytes), self.separador)
        } else {
            partes_de_csv(bytes, self.nome, self.separador, 0)
        }
    }
}
