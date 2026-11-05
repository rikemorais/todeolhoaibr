//! Decodificador do bronze para fontes que gravam um **ZIP de CSVs** no raw.
//!
//! Cobre a CEAP da Câmara (um CSV anual) e os pacotes do TSE (prestação de contas,
//! candidaturas, bens), que trazem vários CSVs por ZIP — tipicamente um por UF, em Latin-1
//! e com `;` como separador. O bronze só descompacta, reencoda para UTF-8 e põe em colunas;
//! a interpretação dos campos é do silver.

use crate::csv_zip::{partes_de_zip_csv, partes_de_zip_csv_em_fluxo};
use crate::decodificador::{Decodificador, ParteBronze};
use crate::escritor::ErroDeRefino;

/// Decodifica um ZIP de CSVs em uma parte Parquet por CSV (entradas não-CSV são ignoradas).
pub struct DecodificadorZipCsv {
    separador: u8,
    /// Decodifica cada CSV em fluxo (para arquivos grandes do TSE; assume Latin-1). Ver #171.
    em_fluxo: bool,
}

impl DecodificadorZipCsv {
    /// CSVs lidos inteiros na memória (auto-detecta UTF-8/Latin-1). Bom para arquivos modestos.
    pub const fn novo(separador: u8) -> Self {
        Self {
            separador,
            em_fluxo: false,
        }
    }

    /// CSVs lidos **em fluxo** (Latin-1), para pacotes grandes do TSE que estourariam a memória.
    pub const fn em_fluxo(separador: u8) -> Self {
        Self {
            separador,
            em_fluxo: true,
        }
    }
}

impl Decodificador for DecodificadorZipCsv {
    fn decodificar(
        &self,
        bytes: &[u8],
        _tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        if self.em_fluxo {
            partes_de_zip_csv_em_fluxo(bytes, self.separador)
        } else {
            partes_de_zip_csv(bytes, self.separador)
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::csv_zip::testes::ler_parte;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    fn zip_com_varios(arquivos: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(Cursor::new(&mut buffer));
            for (nome, conteudo) in arquivos {
                zw.start_file(*nome, SimpleFileOptions::default()).unwrap();
                zw.write_all(conteudo).unwrap();
            }
            zw.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn decodifica_zip_com_um_csv() {
        let csv = "txNomeParlamentar;vlrLiquido\nFulano;100,50\n";
        let partes = DecodificadorZipCsv::novo(b';')
            .decodificar(
                &zip_com_varios(&[("Ano-2024.csv", csv.as_bytes())]),
                "application/zip",
            )
            .unwrap();
        assert_eq!(partes.len(), 1);
        let (colunas, linhas) = ler_parte(&partes[0]);
        assert_eq!(colunas, vec!["txNomeParlamentar", "vlrLiquido"]);
        assert_eq!(
            linhas,
            vec![vec![Some("Fulano".to_owned()), Some("100,50".to_owned())]]
        );
    }

    #[test]
    fn decodifica_zip_com_varios_csv_por_uf_ignorando_pdf() {
        // Como os pacotes do TSE: um CSV por UF + um leiame.pdf.
        let ac = "SG_UF;NM\nAC;Acre\n";
        let am = "SG_UF;NM\nAM;Amazonas\n";
        let zip = zip_com_varios(&[
            ("leiame.pdf", b"ignore"),
            ("bem_candidato_2026_AC.csv", ac.as_bytes()),
            ("bem_candidato_2026_AM.csv", am.as_bytes()),
        ]);
        let partes = DecodificadorZipCsv::novo(b';')
            .decodificar(&zip, "application/zip")
            .unwrap();
        assert_eq!(partes.len(), 2, "um Parquet por CSV, sem o PDF");
        let nomes: Vec<&str> = partes.iter().map(|p| p.nome.as_str()).collect();
        assert!(nomes.contains(&"bem_candidato_2026_AC"));
        assert!(nomes.contains(&"bem_candidato_2026_AM"));
    }
}
