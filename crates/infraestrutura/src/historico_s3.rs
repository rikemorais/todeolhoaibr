use async_trait::async_trait;
use futures::{StreamExt, stream};
use todeolho_nucleo::{
    ArmazenamentoRaw, ConsultaDeExecucoes, ErroDeIngestao, GravadorDeExecucoes, ObjetoRaw,
    RegistroDeExecucao,
};
use tracing::warn;

use crate::armazenamento_s3::{ArmazenamentoS3, ConfiguracaoS3};

const LEITURAS_SIMULTANEAS: usize = 8;

/// Histórico das execuções em um bucket próprio: um JSON por execução.
///
/// O id da execução começa com o horário, então a ordem das chaves é a ordem cronológica.
pub struct HistoricoS3 {
    destino: ArmazenamentoS3,
}

impl HistoricoS3 {
    pub fn novo(configuracao: ConfiguracaoS3) -> Result<Self, ErroDeIngestao> {
        Ok(Self {
            destino: ArmazenamentoS3::novo(configuracao)?,
        })
    }

    pub async fn preparar(&self) -> Result<(), ErroDeIngestao> {
        self.destino.preparar().await
    }

    async fn ler(&self, chave: String) -> Option<RegistroDeExecucao> {
        let resultado = async {
            let resposta = self
                .destino
                .bucket
                .get_object(&chave)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::from_slice(resposta.bytes()).map_err(|e| e.to_string())
        }
        .await;

        // Um registro ilegível não pode esconder todos os outros.
        resultado
            .inspect_err(|erro: &String| warn!(%chave, %erro, "registro de execução ignorado"))
            .ok()
    }
}

#[async_trait]
impl GravadorDeExecucoes for HistoricoS3 {
    async fn salvar(&self, registro: &RegistroDeExecucao) -> Result<(), ErroDeIngestao> {
        let chave = chave_do_registro(&registro.id);
        let conteudo = serde_json::to_vec(registro).map_err(|e| self.destino.erro(&chave, e))?;
        self.destino
            .gravar(ObjetoRaw {
                chave,
                conteudo,
                tipo_de_conteudo: "application/json".to_owned(),
            })
            .await
    }
}

#[async_trait]
impl ConsultaDeExecucoes for HistoricoS3 {
    async fn recentes(&self, limite: usize) -> Result<Vec<RegistroDeExecucao>, ErroDeIngestao> {
        let paginas = self
            .destino
            .bucket
            .list(String::new(), None)
            .await
            .map_err(|e| self.destino.erro("listagem", e))?;

        let chaves: Vec<String> = paginas
            .into_iter()
            .flat_map(|pagina| pagina.contents)
            .map(|objeto| objeto.key)
            .collect();

        Ok(stream::iter(chaves_recentes(chaves, limite))
            .map(|chave| self.ler(chave))
            .buffered(LEITURAS_SIMULTANEAS)
            .filter_map(|registro| async move { registro })
            .collect()
            .await)
    }
}

fn chave_do_registro(id: &str) -> String {
    format!("{id}.json")
}

fn chaves_recentes(mut chaves: Vec<String>, limite: usize) -> Vec<String> {
    chaves.retain(|chave| chave.ends_with(".json"));
    chaves.sort_unstable_by(|a, b| b.cmp(a));
    chaves.truncate(limite);
    chaves
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn escolhe_as_chaves_mais_recentes_primeiro() {
        let chaves = [
            "20261030T100000Z-a.json",
            "20261031T100000Z-b.json",
            "20261029T100000Z-c.json",
            "lixo.tmp",
        ]
        .map(str::to_owned)
        .to_vec();

        assert_eq!(
            chaves_recentes(chaves, 2),
            vec!["20261031T100000Z-b.json", "20261030T100000Z-a.json"]
        );
    }
}
