//! Decodificador do bronze para fontes que gravam um **CSV "solto"** no raw (não zipado).
//!
//! Serve, por exemplo, para as despesas do Senado (CEAPS, Latin-1, com uma linha de
//! preâmbulo antes do cabeçalho) e para as votações da Câmara (UTF-8 com BOM). O bronze só
//! reencoda para UTF-8 e põe em colunas — a interpretação dos campos é do silver.

use crate::csv_zip::partes_de_csv;
use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::ErroDeRefino;

/// Decodifica um CSV solto em uma parte Parquet, com o nome lógico e o separador da fonte.
pub struct DecodificadorCsv {
    nome: &'static str,
    separador: u8,
    /// Linhas de preâmbulo a descartar antes do cabeçalho (ex.: 1 no CEAPS do Senado).
    linhas_a_pular: usize,
}

impl DecodificadorCsv {
    /// Cria o decodificador. `nome` é o nome lógico da tabela registrado na procedência.
    pub const fn novo(nome: &'static str, separador: u8, linhas_a_pular: usize) -> Self {
        Self {
            nome,
            separador,
            linhas_a_pular,
        }
    }
}

impl Decodificador for DecodificadorCsv {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        partes_de_csv(bytes, self.nome, self.separador, self.linhas_a_pular)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::csv_zip::testes::ler_parquet;

    fn c(s: &str) -> Option<String> {
        Some(s.to_owned())
    }

    #[test]
    fn decodifica_csv_solto_pulando_o_preambulo() {
        // Como o CEAPS do Senado: uma linha de preâmbulo antes do cabeçalho real.
        let csv = "\"ULTIMA ATUALIZACAO\";\"hoje\"\nano;valor\n2014;52,38\n";
        let partes = DecodificadorCsv::novo("despesas", b';', 1)
            .decodificar(csv.as_bytes(), "application/octet-stream")
            .unwrap();
        assert_eq!(partes.len(), 1);
        assert_eq!(partes[0].nome, "despesas");
        let (colunas, linhas) = ler_parquet(&partes[0].parquet);
        assert_eq!(colunas, vec!["ano", "valor"]);
        assert_eq!(linhas, vec![vec![c("2014"), c("52,38")]]);
    }

    #[test]
    fn remove_o_bom_de_utf8_da_primeira_coluna() {
        // Como as votações da Câmara: UTF-8 com BOM no início.
        let mut csv = vec![0xEF, 0xBB, 0xBF];
        csv.extend_from_slice("idVotacao;orientacao\n1;Sim\n".as_bytes());
        let partes = DecodificadorCsv::novo("votacoes", b';', 0)
            .decodificar(&csv, "application/octet-stream")
            .unwrap();
        let (colunas, linhas) = ler_parquet(&partes[0].parquet);
        assert_eq!(colunas, vec!["idVotacao", "orientacao"]);
        assert_eq!(linhas, vec![vec![c("1"), c("Sim")]]);
    }
}
