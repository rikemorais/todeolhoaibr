//! Leitura sequencial do bronze com referência à linha de origem para a silver.
//!
//! O manifesto identifica a versão bronze e cada parte identifica a tabela interna
//! de um ZIP. O ordinal conta registros de dados no Parquet, a partir de 1, e
//! continua entre lotes. Ele só é estável dentro da versão indicada pelo manifesto;
//! não é uma chave de negócio entre versões.

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::{ParquetRecordBatchReader, ParquetRecordBatchReaderBuilder};
use todeolho_nucleo::raw::ParteRaw;

use crate::escritor::ErroDeRefino;

/// Proveniência de uma parte bronze, compartilhada entre as linhas de seus lotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrigemParteBronze {
    /// Chave do manifesto vigente, que também identifica a versão bronze.
    pub manifesto: String,
    /// Chave do arquivo Parquet no bucket bronze.
    pub parte: String,
    /// Nome da tabela interna ou do arquivo de origem registrado no manifesto.
    pub tabela: String,
}

/// Posição recuperável de um registro dentro de uma versão bronze.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenciaLinhaBronze {
    /// Proveniência da parte que contém o registro.
    pub origem: Arc<OrigemParteBronze>,
    /// Ordinal do registro de dados na parte, começando em 1.
    pub ordinal: u64,
}

/// Lote lido do Parquet, com a posição do primeiro registro preservada.
#[derive(Debug)]
pub struct LoteBronze {
    /// Registros originais, sem projeção ou filtragem.
    pub registros: RecordBatch,
    /// Proveniência da parte que contém os registros.
    pub origem: Arc<OrigemParteBronze>,
    /// Ordinal do primeiro registro do lote, começando em 1.
    pub primeiro_ordinal: u64,
}

impl LoteBronze {
    /// Referência de uma linha do lote; `indice` é baseado em zero.
    pub fn referencia(&self, indice: usize) -> Option<ReferenciaLinhaBronze> {
        if indice >= self.registros.num_rows() {
            return None;
        }
        Some(ReferenciaLinhaBronze {
            origem: Arc::clone(&self.origem),
            ordinal: self
                .primeiro_ordinal
                .checked_add(u64::try_from(indice).ok()?)?,
        })
    }
}

/// Leitor em lotes de uma parte bronze, sem carregar o Parquet inteiro na memória.
pub struct LeitorDeParteBronze {
    leitor: Option<ParquetRecordBatchReader>,
    origem: Arc<OrigemParteBronze>,
    proximo_ordinal: u64,
}

impl LeitorDeParteBronze {
    /// Abre o Parquet local de uma parte listada no manifesto bronze vigente.
    ///
    /// `tamanho_lote` limita os registros devolvidos por chamada a [`Iterator::next`].
    /// O chamador deve fornecer o arquivo baixado da `parte` correspondente.
    pub fn abrir(
        caminho: &Path,
        chave_manifesto: &str,
        parte: &ParteRaw,
        tamanho_lote: usize,
    ) -> Result<Self, ErroDeRefino> {
        if tamanho_lote == 0 {
            return Err(ErroDeRefino::Decodificacao(
                "o tamanho do lote bronze deve ser maior que zero".to_owned(),
            ));
        }
        let builder = ParquetRecordBatchReaderBuilder::try_new(File::open(caminho)?)?;
        let sem_colunas = builder.schema().fields().is_empty();
        let linhas = builder.metadata().file_metadata().num_rows();
        if sem_colunas && linhas != 0 {
            return Err(ErroDeRefino::Decodificacao(
                "o Parquet bronze tem linhas, mas nenhuma coluna".to_owned(),
            ));
        }
        // Alguns Parquets antigos representam respostas vazias com schema raiz sem
        // colunas. Não há linhas a emitir; o publicador usa o schema Silver do contrato.
        let leitor = if sem_colunas {
            None
        } else {
            Some(builder.with_batch_size(tamanho_lote).build()?)
        };
        Ok(Self {
            leitor,
            origem: Arc::new(OrigemParteBronze {
                manifesto: chave_manifesto.to_owned(),
                parte: parte.chave.clone(),
                tabela: parte.url.clone(),
            }),
            proximo_ordinal: 1,
        })
    }
}

impl Iterator for LeitorDeParteBronze {
    type Item = Result<LoteBronze, ErroDeRefino>;

    fn next(&mut self) -> Option<Self::Item> {
        let registros = match self.leitor.as_mut()?.next()? {
            Ok(registros) => registros,
            Err(erro) => return Some(Err(erro.into())),
        };
        let primeiro_ordinal = self.proximo_ordinal;
        let quantidade = match u64::try_from(registros.num_rows()) {
            Ok(quantidade) => quantidade,
            Err(_) => {
                return Some(Err(ErroDeRefino::Decodificacao(
                    "a parte bronze tem mais linhas que o ordinal suporta".to_owned(),
                )));
            }
        };
        self.proximo_ordinal = match primeiro_ordinal.checked_add(quantidade) {
            Some(proximo) => proximo,
            None => {
                return Some(Err(ErroDeRefino::Decodificacao(
                    "a parte bronze tem mais linhas que o ordinal suporta".to_owned(),
                )));
            }
        };
        Some(Ok(LoteBronze {
            registros,
            origem: Arc::clone(&self.origem),
            primeiro_ordinal,
        }))
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::escritor::EscritorParquet;
    use arrow::array::StringArray;

    fn parte() -> ParteRaw {
        ParteRaw {
            chave: "tse/prestacao-contas/ano=2024/execucao=abc/parte-00001.parquet".into(),
            url: "despesas_contratadas_candidatos_2024_AC".into(),
            bytes: 0,
            sha256: String::new(),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: None,
        }
    }

    fn arquivo_com_lotes() -> tempfile::NamedTempFile {
        let arquivo = tempfile::NamedTempFile::new().unwrap();
        let mut escritor = EscritorParquet::novo_em(
            arquivo.reopen().unwrap(),
            &["SQ_DESPESA".to_owned(), "VR_DESPESA".to_owned()],
        )
        .unwrap();
        escritor
            .escrever_lote(&[
                vec![Some("10".into()), Some("100".into())],
                vec![Some("10".into()), Some("200".into())],
            ])
            .unwrap();
        escritor
            .escrever_lote(&[
                vec![Some("10".into()), Some("200".into())],
                vec![Some("11".into()), Some("300".into())],
            ])
            .unwrap();
        escritor.fechar().unwrap();
        arquivo
    }

    #[test]
    fn preserva_ordinais_e_variantes_entre_lotes() {
        let arquivo = arquivo_com_lotes();
        let parte = parte();
        let manifesto = "tse/prestacao-contas/ano=2024/execucao=abc/_manifesto.json";
        let lotes: Vec<LoteBronze> =
            LeitorDeParteBronze::abrir(arquivo.path(), manifesto, &parte, 2)
                .unwrap()
                .map(Result::unwrap)
                .collect();

        assert_eq!(lotes.len(), 2);
        assert_eq!(lotes[0].primeiro_ordinal, 1);
        assert_eq!(lotes[1].primeiro_ordinal, 3);
        let valores: Vec<String> = lotes
            .iter()
            .flat_map(|lote| {
                let coluna = lote.registros.column(1);
                let coluna = coluna.as_any().downcast_ref::<StringArray>().unwrap();
                (0..lote.registros.num_rows()).map(|indice| coluna.value(indice).to_owned())
            })
            .collect();
        assert_eq!(valores, ["100", "200", "200", "300"]);

        let referencias: Vec<ReferenciaLinhaBronze> = lotes
            .iter()
            .flat_map(|lote| (0..lote.registros.num_rows()).map(|i| lote.referencia(i).unwrap()))
            .collect();
        assert_eq!(
            referencias.iter().map(|r| r.ordinal).collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
        assert!(referencias.iter().all(|r| r.origem.manifesto == manifesto));
        assert!(referencias.iter().all(|r| r.origem.parte == parte.chave));
        assert!(referencias.iter().all(|r| r.origem.tabela == parte.url));
        assert!(lotes[1].referencia(2).is_none());

        let novamente: Vec<u64> = LeitorDeParteBronze::abrir(arquivo.path(), manifesto, &parte, 3)
            .unwrap()
            .flat_map(|lote| {
                let lote = lote.unwrap();
                (0..lote.registros.num_rows())
                    .map(|i| lote.referencia(i).unwrap().ordinal)
                    .collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(novamente, [1, 2, 3, 4]);
    }

    #[test]
    fn rejeita_lote_vazio() {
        let arquivo = arquivo_com_lotes();
        assert!(LeitorDeParteBronze::abrir(arquivo.path(), "manifesto", &parte(), 0).is_err());
    }

    #[test]
    fn trata_parquet_legado_sem_colunas_e_sem_linhas_como_vazio() {
        let arquivo = tempfile::NamedTempFile::new().unwrap();
        let esquema = Arc::new(arrow::datatypes::Schema::empty());
        let escritor =
            parquet::arrow::ArrowWriter::try_new(arquivo.reopen().unwrap(), esquema, None).unwrap();
        escritor.close().unwrap();

        let lotes = LeitorDeParteBronze::abrir(arquivo.path(), "manifesto", &parte(), 10)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(lotes.is_empty());
    }
}
