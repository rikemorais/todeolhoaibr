use std::collections::HashMap;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::{StreamExt, stream};
use tracing::{error, info, warn};

use crate::contexto::ContextoDeExecucao;
use crate::dag::DagDeIngestao;
use crate::erros::ErroDeDag;
use crate::tarefa::ResumoDaTarefa;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EstadoDaTarefa {
    Concluida(ResumoDaTarefa),
    Falhou(String),
    /// Não executada porque a tarefa `causa`, da qual ela depende, falhou.
    Ignorada {
        causa: String,
    },
}

#[derive(Debug, Clone)]
pub struct ResultadoDaTarefa {
    pub nome: String,
    pub estado: EstadoDaTarefa,
    pub duracao: Duration,
}

#[derive(Debug, Clone)]
pub struct RelatorioDaExecucao {
    pub fonte: String,
    pub resultados: Vec<ResultadoDaTarefa>,
}

impl RelatorioDaExecucao {
    pub fn sucesso(&self) -> bool {
        self.resultados
            .iter()
            .all(|r| matches!(r.estado, EstadoDaTarefa::Concluida(_)))
    }
}

/// Recebe os acontecimentos de uma execução (padrão Observer).
///
/// O executor não sabe quem observa nem para quê. Os métodos não devolvem erro de
/// propósito: quem observa não pode interferir na ingestão.
#[async_trait]
pub trait ObservadorDeExecucao: Send + Sync {
    async fn tarefa_iniciada(&self, fonte: &str, tarefa: &str);
    async fn tarefa_finalizada(&self, fonte: &str, resultado: &ResultadoDaTarefa);
}

/// Executa uma DAG nível a nível, com concorrência limitada dentro de cada nível.
///
/// A falha de uma tarefa não interrompe as demais: apenas seus descendentes são
/// ignorados. O Dagron só fornece o grafo; a execução é nossa (ver `docs/adr/0003`).
pub struct ExecutorDeDag {
    concorrencia: usize,
}

impl ExecutorDeDag {
    pub fn novo(concorrencia: usize) -> Self {
        Self {
            concorrencia: concorrencia.max(1),
        }
    }

    pub async fn executar(
        &self,
        dag: &DagDeIngestao,
        contexto: &ContextoDeExecucao,
        observador: &dyn ObservadorDeExecucao,
    ) -> Result<RelatorioDaExecucao, ErroDeDag> {
        let mut resultados = Vec::new();
        let mut causa_de_ignorar: HashMap<String, String> = HashMap::new();

        for nivel in dag.niveis()? {
            let mut a_executar = Vec::new();
            for nome in nivel {
                match causa_de_ignorar.get(&nome) {
                    Some(causa) => {
                        warn!(fonte = dag.fonte(), tarefa = %nome, %causa, "tarefa ignorada");
                        let resultado = ResultadoDaTarefa {
                            nome,
                            estado: EstadoDaTarefa::Ignorada {
                                causa: causa.clone(),
                            },
                            duracao: Duration::ZERO,
                        };
                        observador.tarefa_finalizada(dag.fonte(), &resultado).await;
                        resultados.push(resultado);
                    }
                    None => a_executar.push((dag.tarefa(&nome)?, nome)),
                }
            }

            let concluidas: Vec<ResultadoDaTarefa> = stream::iter(a_executar)
                .map(|(tarefa, nome)| async move {
                    info!(fonte = dag.fonte(), tarefa = %nome, "tarefa iniciada");
                    observador.tarefa_iniciada(dag.fonte(), &nome).await;
                    let inicio = Instant::now();
                    let estado = match tarefa.executar(contexto).await {
                        Ok(resumo) => {
                            info!(
                                fonte = dag.fonte(),
                                tarefa = %nome,
                                partes = resumo.partes,
                                bytes = resumo.bytes,
                                "tarefa concluída"
                            );
                            EstadoDaTarefa::Concluida(resumo)
                        }
                        Err(erro) => {
                            error!(fonte = dag.fonte(), tarefa = %nome, %erro, "tarefa falhou");
                            EstadoDaTarefa::Falhou(erro.to_string())
                        }
                    };
                    let resultado = ResultadoDaTarefa {
                        nome,
                        estado,
                        duracao: inicio.elapsed(),
                    };
                    observador.tarefa_finalizada(dag.fonte(), &resultado).await;
                    resultado
                })
                .buffer_unordered(self.concorrencia)
                .collect()
                .await;

            for resultado in &concluidas {
                if matches!(resultado.estado, EstadoDaTarefa::Falhou(_)) {
                    for descendente in dag.descendentes(&resultado.nome)? {
                        causa_de_ignorar
                            .entry(descendente)
                            .or_insert_with(|| resultado.nome.clone());
                    }
                }
            }
            resultados.extend(concluidas);
        }

        Ok(RelatorioDaExecucao {
            fonte: dag.fonte().to_owned(),
            resultados,
        })
    }
}

#[cfg(test)]
mod testes {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use chrono::Utc;

    use super::*;
    use crate::erros::ErroDeIngestao;
    use crate::portas::{
        ArmazenamentoRaw, ClienteHttp, DownloadHttp, InformacoesDoArquivo, ObjetoRaw, RespostaHttp,
    };
    use crate::tarefa::{Tarefa, TarefaPlanejada};

    struct Inerte;

    #[async_trait]
    impl ArmazenamentoRaw for Inerte {
        async fn preparar(&self) -> Result<(), ErroDeIngestao> {
            Ok(())
        }

        async fn gravar(&self, _: ObjetoRaw) -> Result<(), ErroDeIngestao> {
            Ok(())
        }

        async fn gravar_fluxo(
            &self,
            _: &str,
            _: &str,
            _: &mut (dyn tokio::io::AsyncRead + Send + Unpin),
        ) -> Result<(), ErroDeIngestao> {
            Ok(())
        }

        async fn ler(&self, _: &str) -> Result<Option<Vec<u8>>, ErroDeIngestao> {
            Ok(None)
        }
    }

    #[async_trait]
    impl ClienteHttp for Inerte {
        async fn obter(&self, _: &str, _: &str) -> Result<RespostaHttp, ErroDeIngestao> {
            unreachable!("as tarefas de teste não usam HTTP")
        }

        async fn informacoes(&self, _: &str) -> Result<InformacoesDoArquivo, ErroDeIngestao> {
            unreachable!("as tarefas de teste não usam HTTP")
        }

        async fn baixar(&self, _: &str) -> Result<DownloadHttp, ErroDeIngestao> {
            unreachable!("as tarefas de teste não usam HTTP")
        }
    }

    fn contexto() -> ContextoDeExecucao {
        ContextoDeExecucao {
            id_execucao: "teste".into(),
            iniciada_em: Utc::now(),
            armazenamento: Arc::new(Inerte),
            http: Arc::new(Inerte),
        }
    }

    #[derive(Default)]
    struct ObservadorDeTeste {
        eventos: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl ObservadorDeExecucao for ObservadorDeTeste {
        async fn tarefa_iniciada(&self, _: &str, tarefa: &str) {
            self.eventos
                .lock()
                .unwrap()
                .push(format!("inicio:{tarefa}"));
        }

        async fn tarefa_finalizada(&self, _: &str, resultado: &ResultadoDaTarefa) {
            self.eventos
                .lock()
                .unwrap()
                .push(format!("fim:{}", resultado.nome));
        }
    }

    #[tokio::test]
    async fn avisa_o_observador_inclusive_sobre_tarefas_ignoradas() {
        let cenario = Cenario::novo();
        let dag = DagDeIngestao::construir(
            "teste",
            vec![
                cenario.tarefa("quebrada", true),
                cenario.tarefa("filha", false).depois_de(["quebrada"]),
            ],
        )
        .unwrap();
        let observador = ObservadorDeTeste::default();

        ExecutorDeDag::novo(1)
            .executar(&dag, &contexto(), &observador)
            .await
            .unwrap();

        assert_eq!(
            *observador.eventos.lock().unwrap(),
            vec!["inicio:quebrada", "fim:quebrada", "fim:filha"]
        );
    }

    struct TarefaFalsa {
        nome: &'static str,
        falha: bool,
        ordem: Arc<Mutex<Vec<&'static str>>>,
    }

    #[async_trait]
    impl Tarefa for TarefaFalsa {
        fn nome(&self) -> &str {
            self.nome
        }

        async fn executar(&self, _: &ContextoDeExecucao) -> Result<ResumoDaTarefa, ErroDeIngestao> {
            self.ordem.lock().unwrap().push(self.nome);
            if self.falha {
                return Err(ErroDeIngestao::Http {
                    url: "http://exemplo".into(),
                    detalhe: "fora do ar".into(),
                });
            }
            Ok(ResumoDaTarefa::default())
        }
    }

    struct Cenario {
        ordem: Arc<Mutex<Vec<&'static str>>>,
    }

    impl Cenario {
        fn novo() -> Self {
            Self {
                ordem: Arc::default(),
            }
        }

        fn tarefa(&self, nome: &'static str, falha: bool) -> TarefaPlanejada {
            TarefaPlanejada::nova(TarefaFalsa {
                nome,
                falha,
                ordem: Arc::clone(&self.ordem),
            })
        }
    }

    fn estado<'a>(relatorio: &'a RelatorioDaExecucao, nome: &str) -> &'a EstadoDaTarefa {
        &relatorio
            .resultados
            .iter()
            .find(|r| r.nome == nome)
            .unwrap()
            .estado
    }

    #[tokio::test]
    async fn executa_dependencias_antes_dos_dependentes() {
        let cenario = Cenario::novo();
        let dag = DagDeIngestao::construir(
            "teste",
            vec![
                cenario.tarefa("c", false).depois_de(["b"]),
                cenario.tarefa("b", false).depois_de(["a"]),
                cenario.tarefa("a", false),
            ],
        )
        .unwrap();

        let relatorio = ExecutorDeDag::novo(2)
            .executar(&dag, &contexto(), &ObservadorDeTeste::default())
            .await
            .unwrap();

        assert!(relatorio.sucesso());
        assert_eq!(*cenario.ordem.lock().unwrap(), vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn falha_ignora_somente_os_descendentes() {
        let cenario = Cenario::novo();
        let dag = DagDeIngestao::construir(
            "teste",
            vec![
                cenario.tarefa("quebrada", true),
                cenario.tarefa("filha", false).depois_de(["quebrada"]),
                cenario.tarefa("neta", false).depois_de(["filha"]),
                cenario.tarefa("independente", false),
            ],
        )
        .unwrap();

        let relatorio = ExecutorDeDag::novo(2)
            .executar(&dag, &contexto(), &ObservadorDeTeste::default())
            .await
            .unwrap();

        assert!(!relatorio.sucesso());
        assert!(matches!(
            estado(&relatorio, "quebrada"),
            EstadoDaTarefa::Falhou(_)
        ));
        assert!(matches!(
            estado(&relatorio, "independente"),
            EstadoDaTarefa::Concluida(_)
        ));
        for nome in ["filha", "neta"] {
            assert_eq!(
                estado(&relatorio, nome),
                &EstadoDaTarefa::Ignorada {
                    causa: "quebrada".into()
                }
            );
        }
        assert!(!cenario.ordem.lock().unwrap().contains(&"filha"));
    }
}
