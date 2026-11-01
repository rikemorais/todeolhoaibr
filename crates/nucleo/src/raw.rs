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
    /// Monta o prefixo de um recurso dentro de uma execução, particionado por data.
    pub fn nova(fonte: &str, recurso: &str, iniciada_em: DateTime<Utc>, id_execucao: &str) -> Self {
        let data = iniciada_em.format("%Y-%m-%d");
        Self {
            prefixo: format!("{fonte}/{recurso}/data_ingestao={data}/execucao={id_execucao}"),
        }
    }

    /// Chave que aponta para a última versão ingerida de um recurso (ver `docs/adr/0010`).
    ///
    /// # Exemplos
    ///
    /// ```
    /// use todeolho_nucleo::LocalizacaoRaw;
    ///
    /// assert_eq!(
    ///     LocalizacaoRaw::ultima_versao("ibge", "estados"),
    ///     "ibge/estados/_ultima_versao.json",
    /// );
    /// ```
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

/// Uma parte gravada de um recurso (uma página de API ou o arquivo baixado).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParteRaw {
    /// Chave do objeto na camada Raw.
    pub chave: String,
    /// URL de origem desta parte.
    pub url: String,
    /// Tamanho do conteúdo, em bytes.
    pub bytes: u64,
    /// Assinatura `sha256` do conteúdo (procedência; ver `docs/adr/0012`).
    pub sha256: String,
    /// Tipo de conteúdo (`Content-Type`) como veio da origem.
    pub tipo_de_conteudo: String,
    /// Validador da origem (ETag) desta página, usado na requisição condicional
    /// da próxima rodada. `None` quando a origem não o informou.
    #[serde(default)]
    pub versao: Option<String>,
}

/// Procedência de uma ingestão completa de um recurso.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestoRaw {
    /// Fonte à qual o recurso pertence (ex.: `ibge`).
    pub fonte: String,
    /// Recurso ingerido (ex.: `estados`).
    pub recurso: String,
    /// Execução que produziu esta ingestão.
    pub id_execucao: String,
    /// Primeira URL buscada (início da paginação ou o próprio arquivo).
    pub url_inicial: String,
    /// Quando a ingestão começou.
    pub iniciada_em: DateTime<Utc>,
    /// Quando a ingestão terminou (o manifesto é gravado por último).
    pub concluida_em: DateTime<Utc>,
    /// Partes que compõem o recurso, na ordem em que foram buscadas.
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
    /// Início da execução que publicou este ponteiro. Usado para não deixar uma
    /// rodada mais antiga sobrescrever uma mais nova (ver issue #16). `None` nos
    /// ponteiros gravados antes deste campo existir, tratados como os mais antigos.
    #[serde(default)]
    pub iniciada_em: Option<DateTime<Utc>>,
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
