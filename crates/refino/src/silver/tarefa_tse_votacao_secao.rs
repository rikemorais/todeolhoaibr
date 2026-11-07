//! Tarefa Silver para cada recurso eleitoral `ano × UF` de votação do TSE.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_sql_com_json_string_original},
    tse_votacao_secao::esquema_tse_votacao_secao,
};
use crate::{SnapshotBronze, executor_sql::ConfiguracaoExecutorSql};

const REGRA: &str = "tse-votacao-secao-v2";
const SQL: &str = include_str!("../../sql/silver/tse_votacao_secao.sql");

/// Tarefa para um arquivo de votos por seção de uma eleição e unidade federativa.
pub struct TarefaDeSilverTseVotacaoSecao {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    ano: u16,
    uf: String,
}

impl TarefaDeSilverTseVotacaoSecao {
    /// Cria a tarefa se o recurso tiver ano e UF válidos e layout suportado.
    pub fn nova(
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: impl Into<String>,
    ) -> Option<Self> {
        let recurso = recurso.into();
        let (ano, uf) = ano_uf_do_recurso(&recurso)?;
        if recurso_excluido(&recurso) {
            return None;
        }
        Some(Self {
            bronze,
            snapshot,
            recurso,
            ano,
            uf,
        })
    }

    /// Indica se a Silver ainda precisa acompanhar o snapshot Bronze e a regra atuais.
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
impl Tarefa for TarefaDeSilverTseVotacaoSecao {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_tse_votacao_secao(
            self.bronze.as_ref(),
            contexto.armazenamento.as_ref(),
            &self.snapshot,
            &self.recurso,
            (self.ano, &self.uf),
            contexto.iniciada_em,
            &contexto.id_execucao,
        )
        .await
    }
}

/// Publica um recurso eleitoral `ano × UF`, preservando o grão de voto por seção e votável.
pub async fn publicar_tse_votacao_secao(
    bronze: &dyn ArmazenamentoRaw,
    silver: &dyn ArmazenamentoRaw,
    snapshot: &SnapshotBronze,
    recurso: &str,
    identidade: (u16, &str),
    iniciada_em: DateTime<Utc>,
    id_execucao: &str,
) -> Result<ResumoDaTarefa, ErroDeIngestao> {
    let (ano, uf) = identidade;
    if snapshot.manifesto().fonte != "tse"
        || snapshot.manifesto().recurso != recurso
        || ano_uf_do_recurso(recurso) != Some((ano, uf.to_owned()))
        || recurso_excluido(recurso)
    {
        return Err(ErroDeIngestao::Manifesto {
            recurso: recurso.to_owned(),
            detalhe: "snapshot deve corresponder a um recurso ano × UF suportado".into(),
        });
    }

    let sql = SQL
        .replace("{{ANO_RECURSO}}", &ano.to_string())
        .replace("{{UF_RECURSO}}", uf);
    let executor = ConfiguracaoExecutorSql {
        limite_memoria_bytes: 256 * 1024 * 1024,
        limite_spill_bytes: 8 * 1024 * 1024 * 1024,
        ..ConfiguracaoExecutorSql::default()
    };
    publicar_transformacao_sql_com_json_string_original(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "tse",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_tse_votacao_secao()),
            iniciada_em,
            id_execucao,
        },
        &sql,
        &executor,
    )
    .await
}

/// Extrai as partições `ano=AAAA/uf=UF` do recurso.
pub fn ano_uf_do_recurso(recurso: &str) -> Option<(u16, String)> {
    let particao = recurso.strip_prefix("votacao_secao/")?;
    let (ano, uf) = particao.split_once("/uf=")?;
    let ano = ano.strip_prefix("ano=")?;
    if ano.len() != 4 || !ano.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if uf.len() != 2 || !uf.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return None;
    }
    Some((ano.parse().ok()?, uf.to_owned()))
}

/// Recursos cujos Parquets Bronze perderam os nomes das colunas ao ler o cabeçalho.
///
/// Os votos presidenciais de 1998 são publicados pelo TSE como arquivo próprio e têm um
/// layout sem nomes de coluna, ainda não mapeado. Em 2010, os recursos externos `VT` e `ZZ`
/// também chegam sem schema; seus registros válidos estão presentes no consolidado `BR`.
fn recurso_excluido(recurso: &str) -> bool {
    matches!(
        ano_uf_do_recurso(recurso),
        Some((1998, ref uf)) if uf == "BR"
    ) || matches!(
        ano_uf_do_recurso(recurso),
        Some((2010, ref uf)) if uf == "VT" || uf == "ZZ"
    )
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn recurso_exige_ano_uf_validos_e_exclui_layouts_sem_schema() {
        assert_eq!(
            ano_uf_do_recurso("votacao_secao/ano=2024/uf=AC"),
            Some((2024, "AC".into()))
        );
        assert_eq!(ano_uf_do_recurso("votacao_secao/ano=24/uf=AC"), None);
        assert_eq!(
            ano_uf_do_recurso("votacao_secao/ano=2024/uf=AC/parte=1"),
            None
        );
        assert!(recurso_excluido("votacao_secao/ano=1998/uf=BR"));
        assert!(recurso_excluido("votacao_secao/ano=2010/uf=VT"));
        assert!(recurso_excluido("votacao_secao/ano=2010/uf=ZZ"));
        assert!(!recurso_excluido("votacao_secao/ano=2010/uf=BR"));
        assert!(!recurso_excluido("votacao_secao/ano=2010/uf=AC"));
    }
}
