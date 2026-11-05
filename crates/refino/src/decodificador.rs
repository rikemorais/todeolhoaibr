//! O contrato que cada fonte implementa para traduzir seus bytes do raw em Parquet.
//!
//! O [`Decodificador`] é o único lugar que conhece o **formato** de uma fonte (ZIP, CSV
//! em Latin-1, JSON de API) — o conhecimento que o [ADR 0014] mantém fora da ingestão.
//! Ele devolve o Parquet **já pronto** (uma [`ParteBronze`] por tabela), para poder
//! escrever em lotes e não manter arquivos grandes inteiros em memória.
//!
//! [ADR 0014]: ../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md

use std::io::Write;

use tempfile::{NamedTempFile, TempPath};

use crate::escritor::{ErroDeRefino, Tabela};

/// Uma tabela do bronze já serializada em Parquet, gravada em um **arquivo temporário em
/// disco**, com o nome lógico que terá.
///
/// Guardar em disco — e não em memória — mantém o pico baixo mesmo quando um ZIP rende
/// dezenas de partes (ex.: um CSV por UF nos pacotes do TSE); ver issue #164. O arquivo é
/// apagado quando a parte é descartada.
#[derive(Debug)]
pub struct ParteBronze {
    /// Nome lógico (ex.: o CSV interno de um ZIP; vira parte da procedência).
    pub nome: String,
    /// Caminho do Parquet em disco.
    pub parquet: TempPath,
}

impl ParteBronze {
    /// Cria a parte gravando os bytes do Parquet em um arquivo temporário.
    pub fn nova(nome: impl Into<String>, parquet: &[u8]) -> Result<Self, ErroDeRefino> {
        let mut arquivo = NamedTempFile::new()?;
        arquivo.write_all(parquet)?;
        Ok(Self {
            nome: nome.into(),
            parquet: arquivo.into_temp_path(),
        })
    }

    /// Cria a parte a partir de um Parquet **já gravado** em um arquivo temporário (sem passar
    /// os bytes pela memória). Usado pela decodificação em fluxo de arquivos grandes.
    pub fn de_arquivo(nome: impl Into<String>, parquet: TempPath) -> Self {
        Self {
            nome: nome.into(),
            parquet,
        }
    }

    /// Conveniência para decodificadores que montam uma [`Tabela`] pequena (ex.: JSON):
    /// serializa a tabela em Parquet e empacota como parte.
    pub fn de_tabela(nome: impl Into<String>, tabela: &Tabela) -> Result<Self, ErroDeRefino> {
        Self::nova(nome, &tabela.para_parquet()?)
    }
}

/// Traduz os bytes de **um part** do raw em uma ou mais partes de bronze (Parquet).
///
/// Um recurso de API costuma render uma parte; um ZIP com vários CSVs rende uma por CSV.
pub trait Decodificador: Send + Sync {
    /// Decodifica os bytes de um part do raw, usando o `tipo_de_conteudo` registrado no
    /// manifesto quando precisar distinguir formatos.
    fn decodificar(
        &self,
        bytes: &[u8],
        tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino>;

    /// Decodifica um part já baixado para um **arquivo** (sem carregá-lo inteiro na memória).
    /// O padrão lê o arquivo e chama [`Self::decodificar`]; decodificadores de arquivos grandes
    /// (ZIP em fluxo) sobrescrevem para ler direto do disco.
    fn decodificar_arquivo(
        &self,
        caminho: &std::path::Path,
        tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        self.decodificar(&std::fs::read(caminho)?, tipo_de_conteudo)
    }

    /// Decodifica **todas as partes de um recurso** de uma vez. O padrão decodifica cada
    /// arquivo em separado (uma parte Parquet por arquivo); fontes com milhares de arquivos
    /// pequenos (ex.: um processo por arquivo) sobrescrevem para **agregar** tudo em poucas
    /// partes.
    fn decodificar_arquivos(
        &self,
        caminhos: &[std::path::PathBuf],
        tipo_de_conteudo: &str,
    ) -> Result<Vec<ParteBronze>, ErroDeRefino> {
        let mut partes = Vec::new();
        for caminho in caminhos {
            partes.extend(self.decodificar_arquivo(caminho, tipo_de_conteudo)?);
        }
        Ok(partes)
    }
}
