//! Tarefa Silver para o consolidado nacional de bens declarados do TSE.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use todeolho_nucleo::{
    ArmazenamentoRaw, ContextoDeExecucao, ErroDeIngestao, ResumoDaTarefa, Tarefa, estado_na_raw,
};

use super::{
    publicacao::{ConfiguracaoDePublicacao, publicar_transformacao_selecionada},
    tarefa_tse_candidaturas::parte_brasil,
    tse_bens::{esquema_tse_bens, transformar_tse_bens},
};
use crate::SnapshotBronze;

const REGRA: &str = "tse-bem-candidato-v1";

/// Tarefa para publicar o consolidado `BRASIL` de bens de um recurso anual Bronze.
pub struct TarefaDeSilverTseBens {
    bronze: Arc<dyn ArmazenamentoRaw>,
    snapshot: Arc<SnapshotBronze>,
    recurso: String,
    ano: u16,
}

impl TarefaDeSilverTseBens {
    /// Cria a tarefa para um recurso `bens/ano=AAAA` válido.
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
impl Tarefa for TarefaDeSilverTseBens {
    fn nome(&self) -> &str {
        &self.recurso
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        publicar_tse_bens(
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
pub async fn publicar_tse_bens(
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

    let mut chaves_vistas = HashSet::new();
    publicar_transformacao_selecionada(
        bronze,
        silver,
        snapshot,
        ConfiguracaoDePublicacao {
            fonte: "tse",
            recurso,
            recurso_destino: None,
            regra: REGRA,
            esquema: Some(esquema_tse_bens()),
            iniciada_em,
            id_execucao,
        },
        parte_brasil,
        |lote| transformar_tse_bens(lote, ano, &mut chaves_vistas).map_err(|erro| erro.to_string()),
    )
    .await
}

fn ano_do_recurso(recurso: &str) -> Option<u16> {
    let ano = recurso.strip_prefix("bens/ano=")?;
    (ano.len() == 4 && ano.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| ano.parse::<u16>().ok())
        .flatten()
}

fn erro_recurso(recurso: &str) -> ErroDeIngestao {
    ErroDeIngestao::Manifesto {
        recurso: recurso.to_owned(),
        detalhe: "snapshot deve corresponder a bens/ano=AAAA".into(),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn recurso_exige_ano_eleitoral_de_quatro_digitos() {
        assert_eq!(ano_do_recurso("bens/ano=2024"), Some(2024));
        assert_eq!(ano_do_recurso("bens/ano=24"), None);
        assert_eq!(ano_do_recurso("bens/ano=2024/uf=AC"), None);
    }
}
