//! Tarefa Silver para o consolidado nacional de candidaturas de cada eleição.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ParteRaw, ResumoDaTarefa, Tarefa,
    estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql_com_json_string_original},
    tse_candidaturas::esquema_tse_candidaturas,
};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

const REGRA: &str = "tse-candidatura-registro-v2";
const SQL: &str = include_str!("../../sql/silver/tse_candidaturas.sql");

/// Tarefa para publicar o consolidado `BRASIL` de um recurso anual Bronze.
pub struct TarefaDeSilverTseCandidaturas {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    ano: u16,
}

impl TarefaDeSilverTseCandidaturas {
    /// Cria a tarefa para um recurso `candidaturas/ano=AAAA` válido.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Option<Self> {
        let recurso = recurso.into();
        let ano = ano_do_recurso(&recurso)?;
        Some(Self {
            bronze,
            snapshot,
            recurso,
            ano,
        })
    }

    /// Indica se a Silver ainda precisa acompanhar a versão Bronze e a regra atuais.
    pub async fn pendente(&self, silver: &dyn ArmazenamentoRaw) -> Result<bool, ErroDeIngestao> {
        let versao = format!("{}|regra={REGRA}", self.snapshot.manifesto().id_execucao);
        Ok(!matches!(
            estado_na_raw(silver, "tse", &self.recurso).await?,
            Some(ref estado)
                if estado.completa && estado.integra && estado.versao.as_deref() == Some(versao.as_str())
        ))
    }
}

#[async_trait]
impl Tarefa for TarefaDeSilverTseCandidaturas {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_tse_candidaturas(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            &self.recurso,
            self.ano,
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Publica somente a parte nacional `BRASIL`, sem duplicar as partes por UF.
pub async fn publicar_tse_candidaturas(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    ano: u16,
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    if snapshot.manifesto().fonte != "tse"
        || snapshot.manifesto().recurso != recurso
        || ano_do_recurso(recurso) != Some(ano)
    {
        return Err(erro_recurso(recurso));
    }
    let partes_brasil = snapshot
        .manifesto()
        .partes
        .iter()
        .filter(|parte| parte_brasil(parte))
        .count();
    if partes_brasil != 1 {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: format!("esperada uma parte BRASIL no manifesto, encontradas {partes_brasil}"),
        });
    }

    let sql = SQL.replace("{{ANO_RECURSO}}", &ano.to_string());
    let snapshot_brasil = snapshot.filtrar_partes(parte_brasil);
    publicar_transformacao_sql_com_json_string_original(
        bronze,
        silver,
        &snapshot_brasil,
        ConfiguracaoDePublicacao {
            fonte: "tse",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_tse_candidaturas()),
            iniciada_em,
            id_execucao,
        },
        &sql,
        &ConfiguracaoExecutorSql::default(),
    )
    .await
}

fn ano_do_recurso(recurso: &str) -> Option<u16> {
    let ano = recurso.strip_prefix("candidaturas/ano=")?;
    (ano.len() == 4 && ano.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| ano.parse::<u16>().ok())
        .flatten()
}

pub(super) fn parte_brasil(parte: &ParteRaw) -> bool {
    let nome = parte.url.rsplit('/').next().unwrap_or(&parte.url);
    let nome = nome
        .trim_end_matches(".csv")
        .trim_end_matches(".txt")
        .to_ascii_uppercase();
    nome.ends_with("_BRASIL")
}

fn erro_recurso(recurso: &str) -> ErroDeIngestao {
    ErroDeIngestao::Manifesto {
        recurso: recurso.to_owned(),
        detalhe: "snapshot deve corresponder a candidaturas/ano=AAAA".into(),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn parte(url: &str) -> ParteRaw {
        ParteRaw {
            chave: "tse/candidaturas/ano=2024/parte.parquet".into(),
            url: url.into(),
            bytes: 0,
            sha256: String::new(),
            tipo_de_conteudo: "application/vnd.apache.parquet".into(),
            versao: None,
        }
    }

    #[test]
    fn recurso_exige_ano_de_quatro_digitos() {
        assert_eq!(ano_do_recurso("candidaturas/ano=2024"), Some(2024));
        assert_eq!(ano_do_recurso("candidaturas/ano=24"), None);
        assert_eq!(ano_do_recurso("candidaturas/ano=2024/uf=AC"), None);
    }

    #[test]
    fn seleciona_apenas_o_consolidado_brasil() {
        assert!(parte_brasil(&parte("consulta_cand_2024_BRASIL")));
        assert!(parte_brasil(&parte("consulta_cand_2024_BRASIL.csv")));
        assert!(!parte_brasil(&parte("consulta_cand_2024_AC")));
    }
}
