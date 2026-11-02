//! Histórico das execuções: o que cada execução fez, publicado enquanto ela acontece.
//! É o contrato entre a ingestão e o painel, que rodam em processos separados
//! (ver `docs/adr/0008`).

use std::collections::HashMap;
use std::sync::{Arc, Mutex as MutexSincrono};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::warn;

use crate::contexto::ProgressoDaTarefa;
use crate::dag::DagDeIngestao;
use crate::decisao::MotivoDeIngestao;
use crate::erros::{ErroDeDag, ErroDeIngestao};
use crate::executor::{EstadoDaTarefa, ObservadorDeExecucao, ResultadoDaTarefa};

/// Intervalo máximo entre publicações: mesmo sem mudança, republica para sinalizar
/// que o processo segue vivo.
const INTERVALO_DE_BATIMENTO: Duration = Duration::from_secs(30);
/// Com que frequência o batimento confere o progresso parcial e republica se ele
/// avançou, para o painel mostrar o percentual quase em tempo real.
const INTERVALO_DE_PROGRESSO: Duration = Duration::from_secs(2);
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
/// Estado de uma tarefa no registro de uma execução.
pub enum EstadoRegistrado {
    /// Ainda não começou.
    Pendente,
    /// Em execução.
    EmExecucao,
    /// Terminou com sucesso.
    Concluida,
    /// Terminou com falha.
    Falhou,
    /// Não rodou porque uma dependência falhou.
    Ignorada,
}

/// O que se sabe sobre uma tarefa dentro do registro de uma execução.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistroDeTarefa {
    /// Nome da tarefa.
    pub nome: String,
    /// Posição na DAG: tarefas do mesmo nível são independentes entre si.
    pub nivel: usize,
    /// Nomes das tarefas das quais esta depende.
    pub depende_de: Vec<String>,
    /// Estado atual da tarefa.
    pub estado: EstadoRegistrado,
    /// Quando começou, se já começou.
    pub iniciada_em: Option<DateTime<Utc>>,
    /// Duração, em milissegundos, quando concluída.
    pub duracao_ms: Option<u64>,
    /// Quantas partes gravou, quando concluída.
    pub partes: Option<usize>,
    /// Quantos bytes gravou, quando concluída.
    pub bytes: Option<u64>,
    /// Bytes baixados até agora, enquanto em execução (progresso parcial).
    pub bytes_baixados: Option<u64>,
    /// Tamanho total esperado, enquanto em execução, quando a origem o informa.
    pub bytes_total: Option<u64>,
    /// Mensagem da falha, nome da tarefa que causou a ignorada ou observação sobre a conclusão.
    pub detalhe: Option<String>,
}

/// As tarefas de uma fonte dentro do registro de uma execução.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistroDeFonte {
    /// Nome da fonte.
    pub nome: String,
    /// Registros das tarefas da fonte.
    pub tarefas: Vec<RegistroDeTarefa>,
}

/// Situação geral de uma execução, calculada a partir do registro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SituacaoDaExecucao {
    /// Ainda rodando, com batimento recente.
    EmAndamento,
    /// Concluída com todas as tarefas bem-sucedidas.
    Sucesso,
    /// Concluída, mas com ao menos uma tarefa falha ou ignorada.
    ComFalhas,
    /// O processo parou sem concluir a execução.
    Interrompida,
}

/// Tudo o que uma execução fez, publicado enquanto ela acontece.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistroDeExecucao {
    /// Identificador da execução.
    pub id: String,
    /// Quando a execução começou.
    pub iniciada_em: DateTime<Utc>,
    /// Último batimento: quando o registro foi atualizado pela última vez.
    pub atualizada_em: DateTime<Utc>,
    /// Quando terminou, se já terminou.
    pub concluida_em: Option<DateTime<Utc>>,
    /// Registros por fonte.
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
                        bytes_baixados: None,
                        bytes_total: None,
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

    /// Calcula a situação atual: em andamento, interrompida (sem batimento),
    /// sucesso ou com falhas.
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

    /// Atualiza o progresso parcial de uma tarefa ainda em execução. Ignora tarefas
    /// que já terminaram: o resultado final é mais autoritativo que o parcial.
    fn registrar_progresso(
        &mut self,
        fonte: &str,
        tarefa: &str,
        baixados: u64,
        total: Option<u64>,
    ) {
        if let Some(registro) = self.tarefa_mut(fonte, tarefa)
            && registro.estado == EstadoRegistrado::EmExecucao
        {
            registro.bytes_baixados = Some(baixados);
            registro.bytes_total = total;
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
    /// Persiste o registro atual da execução.
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
    /// Acompanhadores de progresso por tarefa `(fonte, tarefa)`, alimentados pela
    /// tarefa em execução e lidos a cada publicação.
    progresso: MutexSincrono<HashMap<(String, String), Arc<ProgressoDaTarefa>>>,
}

impl RegistradorDeExecucao {
    /// Cria o registrador a partir do registro inicial e de onde publicá-lo.
    pub fn novo(registro: RegistroDeExecucao, gravador: Arc<dyn GravadorDeExecucoes>) -> Self {
        Self {
            registro: Mutex::new(registro),
            gravador,
            progresso: MutexSincrono::new(HashMap::new()),
        }
    }

    /// Fotografa o progresso de cada tarefa: `(fonte, tarefa, baixados, total)`.
    fn instantaneos_de_progresso(&self) -> Vec<(String, String, u64, Option<u64>)> {
        self.progresso
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .map(|((fonte, tarefa), p)| (fonte.clone(), tarefa.clone(), p.baixados(), p.total()))
            .collect()
    }

    /// Publica o estado inicial.
    pub async fn publicar(&self) {
        self.alterar(|_| {}).await;
    }

    /// Republica até a conclusão: a cada `INTERVALO_DE_PROGRESSO` confere o progresso
    /// parcial e, se ele avançou, republica para o painel mostrar o percentual quase
    /// em tempo real; de toda forma republica ao menos a cada `INTERVALO_DE_BATIMENTO`
    /// para sinalizar que o processo segue vivo.
    pub async fn manter_batimento(&self) {
        let mut ultima_publicacao = Instant::now();
        let mut ultimo_baixado = 0u64;
        loop {
            tokio::time::sleep(INTERVALO_DE_PROGRESSO).await;
            if self.registro.lock().await.concluida_em.is_some() {
                return;
            }
            let baixado: u64 = self
                .instantaneos_de_progresso()
                .iter()
                .map(|(_, _, baixados, _)| baixados)
                .sum();
            if baixado != ultimo_baixado || ultima_publicacao.elapsed() >= INTERVALO_DE_BATIMENTO {
                self.publicar().await;
                ultima_publicacao = Instant::now();
                ultimo_baixado = baixado;
            }
        }
    }

    /// Marca a execução como concluída e publica o estado final.
    pub async fn concluir(&self) {
        self.alterar(|registro| registro.concluida_em = Some(Utc::now()))
            .await;
    }

    async fn alterar(&self, alteracao: impl FnOnce(&mut RegistroDeExecucao)) {
        // Fotografa o progresso antes de travar o registro, para não segurar as duas
        // travas ao mesmo tempo.
        let instantaneos = self.instantaneos_de_progresso();
        let mut registro = self.registro.lock().await;
        alteracao(&mut registro);
        for (fonte, tarefa, baixados, total) in instantaneos {
            registro.registrar_progresso(&fonte, &tarefa, baixados, total);
        }
        registro.atualizada_em = Utc::now();
        if let Err(erro) = self.gravador.salvar(&registro).await {
            warn!(%erro, "não foi possível atualizar o histórico da execução");
        }
    }
}

#[async_trait]
impl ObservadorDeExecucao for RegistradorDeExecucao {
    async fn tarefa_iniciada(&self, fonte: &str, tarefa: &str, progresso: Arc<ProgressoDaTarefa>) {
        self.progresso
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((fonte.to_owned(), tarefa.to_owned()), progresso);
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
                        bytes_baixados: None,
                        bytes_total: None,
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
        registrador
            .tarefa_iniciada("fonte", "t0", Arc::new(ProgressoDaTarefa::default()))
            .await;
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

    #[tokio::test]
    async fn publica_o_progresso_parcial_da_tarefa_em_execucao() {
        let gravador = Arc::new(GravadorEmMemoria::default());
        let registrador = RegistradorDeExecucao::novo(
            registro(&[EstadoRegistrado::Pendente], false),
            gravador.clone(),
        );

        let progresso = Arc::new(ProgressoDaTarefa::default());
        registrador
            .tarefa_iniciada("fonte", "t0", Arc::clone(&progresso))
            .await;

        // A tarefa anuncia o total e vai baixando; cada publicação reflete o parcial.
        progresso.definir_total(200);
        progresso.avancar(50);
        registrador.publicar().await;
        progresso.avancar(30);
        registrador.publicar().await;

        let tarefa = |versao: &RegistroDeExecucao| versao.fontes[0].tarefas[0].clone();
        let versoes = gravador.versoes.lock().unwrap();
        let penultima = tarefa(&versoes[versoes.len() - 2]);
        assert_eq!(
            (penultima.bytes_baixados, penultima.bytes_total),
            (Some(50), Some(200))
        );
        let ultima = tarefa(versoes.last().unwrap());
        assert_eq!(
            (ultima.bytes_baixados, ultima.bytes_total),
            (Some(80), Some(200))
        );
    }

    #[tokio::test]
    async fn o_progresso_nao_sobrescreve_a_tarefa_ja_concluida() {
        let gravador = Arc::new(GravadorEmMemoria::default());
        let registrador = RegistradorDeExecucao::novo(
            registro(&[EstadoRegistrado::Pendente], false),
            gravador.clone(),
        );

        let progresso = Arc::new(ProgressoDaTarefa::default());
        registrador
            .tarefa_iniciada("fonte", "t0", Arc::clone(&progresso))
            .await;
        progresso.definir_total(200);
        progresso.avancar(200);
        registrador
            .tarefa_finalizada(
                "fonte",
                &ResultadoDaTarefa {
                    nome: "t0".into(),
                    estado: EstadoDaTarefa::Concluida(ResumoDaTarefa {
                        partes: 1,
                        bytes: 200,
                        reaproveitada: false,
                        motivo: None,
                    }),
                    duracao: Duration::from_millis(10),
                },
            )
            .await;
        // Uma publicação depois da conclusão não deve reabrir o parcial.
        registrador.publicar().await;

        let versoes = gravador.versoes.lock().unwrap();
        let tarefa = versoes.last().unwrap().fontes[0].tarefas[0].clone();
        assert_eq!(tarefa.estado, EstadoRegistrado::Concluida);
        assert_eq!(tarefa.bytes, Some(200));
    }
}
