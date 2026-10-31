//! Contrato da camada Raw: onde os dados ficam e como cada ingestão é descrita.
//! As futuras etapas de transformação vão ler a partir deste contrato (ver `docs/adr/0006`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Prefixo de um recurso dentro de uma execução.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalizacaoRaw {
    prefixo: String,
}

impl LocalizacaoRaw {
    pub fn nova(fonte: &str, recurso: &str, iniciada_em: DateTime<Utc>, id_execucao: &str) -> Self {
        let data = iniciada_em.format("%Y-%m-%d");
        Self {
            prefixo: format!("{fonte}/{recurso}/data_ingestao={data}/execucao={id_execucao}"),
        }
    }

    /// Chave que aponta para a última versão ingerida de um recurso (ver `docs/adr/0010`).
    pub fn ultima_versao(fonte: &str, recurso: &str) -> String {
        format!("{fonte}/{recurso}/_ultima_versao.json")
    }

    /// Chave da parte de número `numero` (a partir de 1).
    pub fn parte(&self, numero: usize, extensao: &str) -> String {
        format!("{}/parte-{numero:05}.{extensao}", self.prefixo)
    }

    /// Chave do manifesto, gravado por último: sua presença indica ingestão completa.
    pub fn manifesto(&self) -> String {
        format!("{}/_manifesto.json", self.prefixo)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParteRaw {
    pub chave: String,
    pub url: String,
    pub bytes: u64,
    pub sha256: String,
    pub tipo_de_conteudo: String,
    /// Validador da origem (ETag) desta página, usado na requisição condicional
    /// da próxima rodada. `None` quando a origem não o informou.
    #[serde(default)]
    pub versao: Option<String>,
}

/// Procedência de uma ingestão completa de um recurso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestoRaw {
    pub fonte: String,
    pub recurso: String,
    pub id_execucao: String,
    pub url_inicial: String,
    pub iniciada_em: DateTime<Utc>,
    pub concluida_em: DateTime<Utc>,
    pub partes: Vec<ParteRaw>,
}

/// Ponteiro para a última ingestão completa de um recurso (`_ultima_versao.json`).
/// Gravado por último, depois do manifesto, e lido pela pré-checagem (ver `docs/adr/0010`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UltimaVersao {
    /// Validador da origem (ETag/Last-Modified) daquela ingestão, se houver.
    #[serde(default)]
    pub versao: Option<String>,
    /// Chave do manifesto vigente.
    pub manifesto: String,
}

#[cfg(test)]
mod testes {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn monta_as_chaves_no_layout_documentado() {
        let instante = Utc.with_ymd_and_hms(2026, 10, 31, 23, 59, 0).unwrap();
        let local = LocalizacaoRaw::nova("ibge", "estados", instante, "abc");

        assert_eq!(
            local.parte(1, "json"),
            "ibge/estados/data_ingestao=2026-10-31/execucao=abc/parte-00001.json"
        );
        assert_eq!(
            local.manifesto(),
            "ibge/estados/data_ingestao=2026-10-31/execucao=abc/_manifesto.json"
        );
    }
}
