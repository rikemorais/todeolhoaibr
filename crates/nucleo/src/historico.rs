//! Histórico das execuções: o que cada execução fez, publicado enquanto ela acontece.
//! É o contrato entre a ingestão e o painel, que rodam em processos separados
//! (ver `docs/adr/0008`).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::warn;

use crate::dag::DagDeIngestao;
use crate::decisao::MotivoDeIngestao;
use crate::erros::{ErroDeDag, ErroDeIngestao};
use crate::executor::{EstadoDaTarefa, ObservadorDeExecucao, ResultadoDaTarefa};

/// Intervalo entre publicações enquanto nenhuma tarefa muda de estado.
const INTERVALO_DE_BATIMENTO: Duration = Duration::from_secs(30);
/// Sem publicações por este tempo, a execução é considerada interrompida.
const TOLERANCIA_SEM_BATIMENTO: TimeDelta = TimeDelta::minutes(5);

/// Texto para o painel explicando por que a tarefa foi reingerida.
fn motivo_texto(motivo: MotivoDeIngestao) -> String {
    match motivo {
        MotivoDeIngestao::AusenteNaRaw => "Primeira ingestão.",
        MotivoDeIngestao::IncompletaOuCorrompida => {
            "A ingestão anterior estava incompleta ou corrompida; refeita."
        }
        MotivoDeIngestao::Menor => "A origem ficou maior que o guardado; reingerida.",
        MotivoDeIngestao::Desatualizada => "A origem mudou desde a última ingestão; reingerida.",
    }
    .to_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EstadoRegistrado {
    Pendente,
    EmExecucao,
    Concluida,
    Falhou,
    Ignorada,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistroDeTarefa {
    pub nome: String,
    /// Posição na DAG: tarefas do mesmo nível são independentes entre si.
    pub nivel: usize,
    pub depende_de: Vec<String>,
    pub estado: EstadoRegistrado,
    pub iniciada_em: Option<DateTime<Utc>>,
    pub duracao_ms: Option<u64>,
    pub partes: Option<usize>,
    pub bytes: Option<u64>,
    /// Mensagem da falha, nome da tarefa que causou a ignorada ou observação sobre a conclusão.
    pub detalhe: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistroDeFonte {
    pub nome: String,
    pub tarefas: Vec<RegistroDeTarefa>,
}

/// Situação geral de uma execução, calculada a partir do registro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SituacaoDaExecucao {
    EmAndamento,
    Sucesso,
    ComFalhas,
    /// O processo parou sem concluir a execução.
    Interrompida,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistroDeExecucao {
    pub id: String,
    pub iniciada_em: DateTime<Utc>,
    pub atualizada_em: DateTime<Utc>,
    pub concluida_em: Option<DateTime<Utc>>,
    pub fontes: Vec<RegistroDeFonte>,
}

impl RegistroDeExecucao {
    /// Cria o registro com todas as tarefas pendentes, para que apareçam desde o início.
    pub fn novo(
        id: &str,
        iniciada_em: DateTime<Utc>,
        dags: &[DagDeIngestao],
    ) -> Result<Self, ErroDeDag> {
        let mut fontes = Vec::with_capacity(dags.len());
        for dag in dags {
            let mut tarefas = Vec::new();
            for (nivel, nomes) in dag.niveis()?.into_iter().enumerate() {
                for nome in nomes {
                    tarefas.push(RegistroDeTarefa {
                        depende_de: dag.dependencias(&nome)?,
                        nome,
                        nivel,
                        estado: EstadoRegistrado::Pendente,
                        iniciada_em: None,
                        duracao_ms: None,
                        partes: None,
                        bytes: None,
                        detalhe: None,
                    });
                }
            }
            fontes.push(RegistroDeFonte {
                nome: dag.fonte().to_owned(),
                tarefas,
            });
        }

        Ok(Self {
            id: id.to_owned(),
            iniciada_em,
            atualizada_em: iniciada_em,
            concluida_em: None,
            fontes,
        })
    }

    pub fn situacao(&self, agora: DateTime<Utc>) -> SituacaoDaExecucao {
        if self.concluida_em.is_none() {
            return if agora - self.atualizada_em > TOLERANCIA_SEM_BATIMENTO {
                SituacaoDaExecucao::Interrompida
            } else {
                SituacaoDaExecucao::EmAndamento
            };
        }

        let concluiu_tudo = self
            .fontes
            .iter()
            .flat_map(|fonte| &fonte.tarefas)
            .all(|tarefa| tarefa.estado == EstadoRegistrado::Concluida);
        if concluiu_tudo {
            SituacaoDaExecucao::Sucesso
        } else {
            SituacaoDaExecucao::ComFalhas
        }
    }

    fn tarefa_mut(&mut self, fonte: &str, tarefa: &str) -> Option<&mut RegistroDeTarefa> {
        self.fontes
            .iter_mut()
            .find(|f| f.nome == fonte)?
            .tarefas
            .iter_mut()
            .find(|t| t.nome == tarefa)
    }

    fn registrar_inicio(&mut self, fonte: &str, tarefa: &str, agora: DateTime<Utc>) {
        if let Some(registro) = self.tarefa_mut(fonte, tarefa) {
            registro.estado = EstadoRegistrado::EmExecucao;
            registro.iniciada_em = Some(agora);
        }
    }

    fn registrar_resultado(&mut self, fonte: &str, resultado: &ResultadoDaTarefa) {
        let Some(registro) = self.tarefa_mut(fonte, &resultado.nome) else {
            return;
        };
        match &resultado.estado {
            EstadoDaTarefa::Concluida(resumo) => {
                registro.estado = EstadoRegistrado::Concluida;
                registro.partes = Some(resumo.partes);
                registro.bytes = Some(resumo.bytes);
                registro.detalhe = if resumo.reaproveitada {
                    Some("Sem alterações na origem; nada foi baixado.".to_owned())
                } else {
                    resumo.motivo.map(motivo_texto)
                };
            }
            EstadoDaTarefa::Falhou(erro) => {
                registro.estado = EstadoRegistrado::Falhou;
                registro.detalhe = Some(erro.clone());
            }
            EstadoDaTarefa::Ignorada { causa } => {
                registro.estado = EstadoRegistrado::Ignorada;
                registro.detalhe = Some(causa.clone());
                return;
            }
        }
        registro.duracao_ms = Some(resultado.duracao.as_millis() as u64);
    }
}

/// Onde a ingestão publica o histórico.
#[async_trait]
pub trait GravadorDeExecucoes: Send + Sync {
    async fn salvar(&self, registro: &RegistroDeExecucao) -> Result<(), ErroDeIngestao>;
}

/// De onde o painel lê o histórico.
#[async_trait]
pub trait ConsultaDeExecucoes: Send + Sync {
    /// Execuções mais recentes primeiro.
    async fn recentes(&self, limite: usize) -> Result<Vec<RegistroDeExecucao>, ErroDeIngestao>;
}

/// Mantém o histórico atualizado durante a execução.
///
/// O histórico serve para observar a ingestão, não pode derrubá-la: falhas ao publicar
/// só geram um aviso no log. A trava cobre também a gravação, para que uma publicação
/// mais antiga nunca sobrescreva uma mais nova.
pub struct RegistradorDeExecucao {
    registro: Mutex<RegistroDeExecucao>,
    gravador: Arc<dyn GravadorDeExecucoes>,
}

impl RegistradorDeExecucao {
    pub fn novo(registro: RegistroDeExecucao, gravador: Arc<dyn GravadorDeExecucoes>) -> Self {
        Self {
            registro: Mutex::new(registro),
            gravador,
        }
    }

    /// Publica o estado inicial.
    pub async fn publicar(&self) {
        self.alterar(|_| {}).await;
    }

    /// Republica periodicamente até a conclusão, indicando que o processo segue vivo.
    pub async fn manter_batimento(&self) {
        loop {
            tokio::time::sleep(INTERVALO_DE_BATIMENTO).await;
            if self.registro.lock().await.concluida_em.is_some() {
                return;
            }
            self.publicar().await;
        }
    }

    pub async fn concluir(&self) {
        self.alterar(|registro| registro.concluida_em = Some(Utc::now()))
            .await;
    }

    async fn alterar(&self, alteracao: impl FnOnce(&mut RegistroDeExecucao)) {
        let mut registro = self.registro.lock().await;
        alteracao(&mut registro);
        registro.atualizada_em = Utc::now();
        if let Err(erro) = self.gravador.salvar(&registro).await {
            warn!(%erro, "não foi possível atualizar o histórico da execução");
        }
    }
}

#[async_trait]
impl ObservadorDeExecucao for RegistradorDeExecucao {
    async fn tarefa_iniciada(&self, fonte: &str, tarefa: &str) {
        self.alterar(|registro| registro.registrar_inicio(fonte, tarefa, Utc::now()))
            .await;
    }

    async fn tarefa_finalizada(&self, fonte: &str, resultado: &ResultadoDaTarefa) {
        self.alterar(|registro| registro.registrar_resultado(fonte, resultado))
            .await;
    }
}

#[cfg(test)]
mod testes {
    use std::sync::Mutex as MutexSincrono;

    use chrono::TimeZone;

    use super::*;
    use crate::tarefa::ResumoDaTarefa;

    fn instante(minuto: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 10, 31, 12, minuto, 0).unwrap()
    }

    fn registro(estados: &[EstadoRegistrado], concluida: bool) -> RegistroDeExecucao {
        RegistroDeExecucao {
            id: "exec".into(),
            iniciada_em: instante(0),
            atualizada_em: instante(0),
            concluida_em: concluida.then(|| instante(1)),
            fontes: vec![RegistroDeFonte {
                nome: "fonte".into(),
                tarefas: estados
                    .iter()
                    .enumerate()
                    .map(|(indice, estado)| RegistroDeTarefa {
                        nome: format!("t{indice}"),
                        nivel: 0,
                        depende_de: Vec::new(),
                        estado: *estado,
                        iniciada_em: None,
                        duracao_ms: None,
                        partes: None,
                        bytes: None,
                        detalhe: None,
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn sucesso_exige_todas_as_tarefas_concluidas() {
        use EstadoRegistrado::{Concluida, Falhou, Ignorada};

        let agora = instante(2);
        assert_eq!(
            registro(&[Concluida, Concluida], true).situacao(agora),
            SituacaoDaExecucao::Sucesso
        );
        assert_eq!(
            registro(&[Concluida, Falhou, Ignorada], true).situacao(agora),
            SituacaoDaExecucao::ComFalhas
        );
    }

    #[test]
    fn sem_batimento_alem_da_tolerancia_e_interrompida() {
        let registro = registro(&[EstadoRegistrado::EmExecucao], false);

        assert_eq!(
            registro.situacao(instante(5)),
            SituacaoDaExecucao::EmAndamento
        );
        assert_eq!(
            registro.situacao(instante(6)),
            SituacaoDaExecucao::Interrompida
        );
    }

    #[test]
    fn execucao_concluida_nunca_vira_interrompida() {
        let registro = registro(&[EstadoRegistrado::Concluida], true);
        assert_eq!(registro.situacao(instante(59)), SituacaoDaExecucao::Sucesso);
    }

    #[derive(Default)]
    struct GravadorEmMemoria {
        versoes: MutexSincrono<Vec<RegistroDeExecucao>>,
    }

    #[async_trait]
    impl GravadorDeExecucoes for GravadorEmMemoria {
        async fn salvar(&self, registro: &RegistroDeExecucao) -> Result<(), ErroDeIngestao> {
            self.versoes.lock().unwrap().push(registro.clone());
            Ok(())
        }
    }

    #[tokio::test]
    async fn publica_cada_mudanca_de_estado() {
        let gravador = Arc::new(GravadorEmMemoria::default());
        let registrador = RegistradorDeExecucao::novo(
            registro(&[EstadoRegistrado::Pendente], false),
            gravador.clone(),
        );

        registrador.publicar().await;
        registrador.tarefa_iniciada("fonte", "t0").await;
        registrador
            .tarefa_finalizada(
                "fonte",
                &ResultadoDaTarefa {
                    nome: "t0".into(),
                    estado: EstadoDaTarefa::Concluida(ResumoDaTarefa {
                        partes: 2,
                        bytes: 10,
                        reaproveitada: false,
                        motivo: None,
                    }),
                    duracao: Duration::from_millis(1500),
                },
            )
            .await;
        registrador.concluir().await;

        let versoes = gravador.versoes.lock().unwrap();
        let estados: Vec<_> = versoes
            .iter()
            .map(|v| v.fontes[0].tarefas[0].estado)
            .collect();
        assert_eq!(
            estados,
            vec![
                EstadoRegistrado::Pendente,
                EstadoRegistrado::EmExecucao,
                EstadoRegistrado::Concluida,
                EstadoRegistrado::Concluida,
            ]
        );

        let ultima = versoes.last().unwrap();
        let tarefa = &ultima.fontes[0].tarefas[0];
        assert_eq!(
            (tarefa.partes, tarefa.bytes, tarefa.duracao_ms),
            (Some(2), Some(10), Some(1500))
        );
        assert!(ultima.concluida_em.is_some());
    }
}
