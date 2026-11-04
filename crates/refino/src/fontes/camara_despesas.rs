//! Decodificador do bronze para a fonte `camara-despesas` (CEAP).
//!
//! O raw guarda um ZIP com um CSV anual (`;` como separador). O bronze só descompacta,
//! reencoda para UTF-8 e põe em colunas — a interpretação dos campos é do silver.

use crate::csv_zip::partes_de_zip_csv;
use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::ErroDeRefino;

/// Separador dos CSVs da CEAP.
const SEPARADOR: u8 = b';';

/// Decodifica o ZIP+CSV da CEAP (despesas da Câmara) em uma parte Parquet por CSV.
pub struct DecodificadorCamaraDespesas;

impl Decodificador for DecodificadorCamaraDespesas {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        partes_de_zip_csv(bytes, SEPARADOR)
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::csv_zip::testes::ler_parquet;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    #[test]
    fn decodifica_o_zip_da_ceap_em_parquet() {
        let csv = "txNomeParlamentar;vlrLiquido\nFulano;100,50\n";
        let mut buffer = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut buffer));
            zw.start_file("Ano-2024.csv", SimpleFileOptions::default())
                .unwrap();
            zw.write_all(csv.as_bytes()).unwrap();
            zw.finish().unwrap();
        }
        let partes = DecodificadorCamaraDespesas
            .decodificar(&buffer, "application/zip")
            .unwrap();
        assert_eq!(partes.len(), 1);
        let (colunas, linhas) = ler_parquet(&partes[0].parquet);
        assert_eq!(colunas, vec!["txNomeParlamentar", "vlrLiquido"]);
        assert_eq!(
            linhas,
            vec![vec![Some("Fulano".to_owned()), Some("100,50".to_owned())]]
        );
    }
}
