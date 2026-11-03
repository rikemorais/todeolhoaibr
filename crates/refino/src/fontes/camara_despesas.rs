//! Decodificador do bronze para a fonte `camara-despesas` (CEAP).
//!
//! O raw guarda um ZIP com um CSV anual (`;` como separador). O bronze só descompacta,
//! reencoda para UTF-8 e põe em colunas — a interpretação dos campos é do silver.

use crate::csv_zip::tabelas_de_zip_csv;
use crate::decodificador::{Decodificador, TabelaNomeada};
use crate::escritor::ErroDeRefino;

/// Separador dos CSVs da CEAP.
const SEPARADOR: u8 = b';';

/// Decodifica o ZIP+CSV da CEAP (despesas da Câmara) em uma tabela por CSV.
pub struct DecodificadorCamaraDespesas;

impl Decodificador for DecodificadorCamaraDespesas {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<TabelaNomeada>, ErroDeRefino> {
        tabelas_de_zip_csv(bytes, SEPARADOR)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    #[test]
    fn decodifica_o_zip_da_ceap_em_tabela() {
        let csv = "txNomeParlamentar;vlrLiquido\nFulano;100,50\n";
        let mut buffer = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut buffer));
            zw.start_file("Ano-2024.csv", SimpleFileOptions::default())
                .unwrap();
            zw.write_all(csv.as_bytes()).unwrap();
            zw.finish().unwrap();
        }
        let tabelas = DecodificadorCamaraDespesas
            .decodificar(&buffer, "application/zip")
            .unwrap();
        assert_eq!(tabelas.len(), 1);
        assert_eq!(
            tabelas[0].tabela.colunas,
            vec!["txNomeParlamentar", "vlrLiquido"]
        );
        assert_eq!(
            tabelas[0].tabela.linhas,
            vec![vec![Some("Fulano".into()), Some("100,50".into())]]
        );
    }
}
