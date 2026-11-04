//! Raiz de composição: único lugar que liga as abstrações às implementações concretas.

mod menu;

use std::env;
use std::io::{self, IsTerminal, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use futures::stream::{self, StreamExt};
use todeolho_fontes::catalogo;
use todeolho_infraestrutura::{
    ArmazenamentoS3, ClienteHttpComRetentativas, ConfiguracaoS3, HistoricoS3,
    PoliticaDeRetentativas, espaco_livre_em,
};
use todeolho_nucleo::{
    ArmazenamentoRaw, AvaliacaoDeEspaco, Camada, ContextoDeExecucao, DagDeIngestao,
    EspacoNecessario, EstadoDaTarefa, ExecutorDeDag, FiltroDeTarefas, FonteDeDados, PerfilHttp,
    PrecisaoDaEstimativa, RegistradorDeExecucao, RegistroDeExecucao, RelatorioDaExecucao, Tarefa,
    TarefaPlanejada, estimar, formatar_bytes,
};
use todeolho_refino::{FonteDeRefino, TarefaDeBronze, catalogo_refino};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

const ERRO_DE_ARMAZENAMENTO: &str = "não foi possível acessar o armazenamento; ele está no ar?";

/// Quantas fontes têm o tamanho estimado ao mesmo tempo ao montar o menu. Hosts distintos
/// trabalham em paralelo sem exagerar na carga do gateway de armazenamento local.
const FONTES_SIMULTANEAS: usize = 4;

#[derive(Parser)]
#[command(
    name = "todeolho",
    version,
    about = "Tô De Olho Aí: ingestão de dados públicos"
)]
struct Cli {
    #[command(subcommand)]
    comando: Comando,
}

#[derive(Subcommand)]
enum Comando {
    /// Abre o menu para escolher o que ingerir, mostrando o espaço necessário
    Selecionar,
    /// Lista as fontes de dados disponíveis
    Listar,
    /// Mostra a ordem de execução da DAG de uma fonte, sem acessar a rede
    Plano {
        /// Nome da fonte (veja `todeolho listar`)
        fonte: String,
    },
    /// Executa a ingestão sem o menu e grava os dados na camada Raw
    Executar(ArgumentosDeExecucao),
    /// Refina a camada Raw para o medalhão (hoje: `bronze`, em Parquet)
    Refinar {
        /// Camada de destino (por ora, apenas `bronze`)
        camada: String,
        /// Fonte de refino (veja o catálogo do refino, ex.: `camara-despesas`)
        fonte: Option<String>,
        /// Refina todas as fontes do catálogo de refino
        #[arg(long, conflicts_with = "fonte")]
        todas: bool,
        /// Repete a reconciliação a cada N segundos (0 ou ausente: roda uma vez).
        /// Como o bronze é idempotente, passadas sem mudança são quase instantâneas.
        #[arg(long, value_name = "SEGUNDOS")]
        intervalo: Option<u64>,
    },
    /// Inicia o painel web com o histórico das execuções
    Painel {
        /// Endereço em que o painel fica disponível
        #[arg(long, default_value = "0.0.0.0:8080")]
        endereco: SocketAddr,
    },
}

#[derive(Args)]
struct ArgumentosDeExecucao {
    /// Nome da fonte a ingerir
    fonte: Option<String>,
    /// Ingere as fontes do catálogo, exceto as muito grandes, que só rodam pelo nome
    #[arg(long)]
    todas: bool,
    /// Restringe a fonte, como --filtro ano=2022,2024 --filtro uf=AC
    #[arg(long = "filtro", value_name = "NOME=VALORES")]
    filtros: Vec<String>,
    /// Não pede confirmação. A verificação de espaço continua valendo
    #[arg(long)]
    sim: bool,
}

/// Uma fonte com as tarefas que foram selecionadas nela.
struct FonteSelecionada {
    nome: String,
    tarefas: Vec<TarefaPlanejada>,
    perfil_http: PerfilHttp,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("TODEOLHO_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .with_writer(io::stderr)
        .with_ansi(io::stderr().is_terminal())
        .init();

    match rodar(Cli::parse()).await {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(erro) => {
            eprintln!("erro: {erro:#}");
            ExitCode::FAILURE
        }
    }
}

/// Devolve `false` quando alguma tarefa não foi concluída.
async fn rodar(cli: Cli) -> Result<bool> {
    match cli.comando {
        Comando::Selecionar => selecionar().await,
        Comando::Listar => {
            let fontes = catalogo();
            // Alinha a coluna pelo nome mais longo, para nomes de qualquer tamanho.
            let largura = fontes.iter().map(|f| f.nome().len()).max().unwrap_or(0);
            for fonte in &fontes {
                println!("{:<largura$} {}", fonte.nome(), fonte.descricao());
            }
            Ok(true)
        }
        Comando::Plano { fonte } => {
            let fonte = buscar_fonte(&fonte)?;
            for (indice, nivel) in fonte.montar_dag()?.niveis()?.iter().enumerate() {
                println!("nível {}: {}", indice + 1, nivel.join(", "));
            }
            Ok(true)
        }
        Comando::Executar(argumentos) => executar_sem_menu(argumentos).await,
        Comando::Refinar {
            camada,
            fonte,
            todas,
            intervalo,
        } => refinar(camada, fonte, todas, intervalo).await,
        Comando::Painel { endereco } => {
            let historico = historico()?;
            historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
            todeolho_painel::servir(endereco, Arc::new(historico)).await?;
            Ok(true)
        }
    }
}

fn buscar_fonte(nome: &str) -> Result<Box<dyn FonteDeDados>> {
    let fontes = catalogo();
    let disponiveis: Vec<String> = fontes.iter().map(|f| f.nome().to_owned()).collect();
    match fontes.into_iter().find(|fonte| fonte.nome() == nome) {
        Some(fonte) => Ok(fonte),
        None => bail!(
            "fonte desconhecida: {nome}. Disponíveis: {}",
            disponiveis.join(", ")
        ),
    }
}

async fn selecionar() -> Result<bool> {
    if !io::stdin().is_terminal() {
        bail!("o menu precisa de um terminal. Sem terminal, use: todeolho executar <fonte> --sim");
    }

    let contexto = preparar_contexto().await?;
    println!("Consultando o tamanho dos dados...");

    let fontes = catalogo();
    // As fontes consultam hosts diferentes (e lêem o estado na Raw), então estimá-las em
    // paralelo sobrepõe o trabalho em vez de somá-lo; `buffered` preserva a ordem do catálogo.
    let opcoes: Vec<menu::OpcaoDeFonte> = stream::iter(fontes.iter())
        .map(|fonte| {
            let contexto = &contexto;
            async move {
                let tarefas = fonte.criar_tarefas();
                montar_opcao_de_menu(fonte.as_ref(), &tarefas, async {
                    match contexto_para_fonte(contexto, fonte.perfil_http()) {
                        Ok(contexto_da_fonte) => {
                            estimar(
                                &tarefas_de(&tarefas),
                                &contexto_da_fonte,
                                PrecisaoDaEstimativa::Rapida,
                            )
                            .await
                        }
                        Err(_) => vec![None; tarefas.len()],
                    }
                })
                .await
            }
        })
        .buffered(FONTES_SIMULTANEAS)
        .collect()
        .await;

    let mut entrada = io::stdin().lock();
    let Some(escolha) = menu::escolher(&opcoes, &mut entrada, &mut io::stdout())? else {
        println!("Nada foi executado.");
        return Ok(true);
    };
    drop(entrada);

    let mut selecionadas = Vec::with_capacity(escolha.len());
    for (indice, filtro) in escolha {
        let fonte = &fontes[indice];
        selecionadas.push(FonteSelecionada {
            nome: fonte.nome().to_owned(),
            tarefas: filtro.aplicar(fonte.criar_tarefas())?,
            perfil_http: fonte.perfil_http(),
        });
    }
    executar(selecionadas, contexto, false).await
}

async fn montar_opcao_de_menu(
    fonte: &dyn FonteDeDados,
    tarefas: &[TarefaPlanejada],
    consulta: impl std::future::Future<Output = Vec<Option<u64>>>,
) -> menu::OpcaoDeFonte {
    // Estimar é barato mesmo nas fontes sob_demanda: as tarefas HTTP só leem o
    // estado local na Raw e as de arquivo fazem um HEAD. O crawl pesado só ocorre
    // no `executar`, nunca aqui.
    let estimativas = consulta.await;
    menu::OpcaoDeFonte {
        nome: fonte.nome().to_owned(),
        tarefas: tarefas
            .iter()
            .zip(estimativas)
            .map(|(tarefa, bytes)| menu::OpcaoDeTarefa {
                etiquetas: tarefa.etiquetas.clone(),
                bytes,
            })
            .collect(),
    }
}

async fn executar_sem_menu(argumentos: ArgumentosDeExecucao) -> Result<bool> {
    let selecionadas = match (argumentos.fonte, argumentos.todas) {
        (Some(_), true) => bail!("informe uma fonte ou --todas, não os dois"),
        (None, false) => bail!("informe uma fonte ou --todas"),
        (None, true) => {
            if !argumentos.filtros.is_empty() {
                bail!("--filtro só vale para uma fonte específica");
            }
            catalogo()
                .into_iter()
                .filter(|fonte| !fonte.sob_demanda())
                .map(|fonte| FonteSelecionada {
                    nome: fonte.nome().to_owned(),
                    tarefas: fonte.criar_tarefas(),
                    perfil_http: fonte.perfil_http(),
                })
                .collect()
        }
        (Some(nome), false) => {
            let fonte = buscar_fonte(&nome)?;
            let filtro = FiltroDeTarefas::a_partir_de(&argumentos.filtros)?;
            vec![FonteSelecionada {
                nome,
                tarefas: filtro.aplicar(fonte.criar_tarefas())?,
                perfil_http: fonte.perfil_http(),
            }]
        }
    };

    let contexto = preparar_contexto().await?;
    executar(selecionadas, contexto, argumentos.sim).await
}

async fn preparar_contexto() -> Result<ContextoDeExecucao> {
    let armazenamento = Arc::new(ArmazenamentoS3::novo(configuracao_s3("raw")?)?);
    armazenamento
        .preparar()
        .await
        .context(ERRO_DE_ARMAZENAMENTO)?;

    let iniciada_em = Utc::now();
    Ok(ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            &Uuid::new_v4().simple().to_string()[..8]
        ),
        iniciada_em,
        armazenamento,
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    })
}

fn contexto_para_fonte(
    contexto: &ContextoDeExecucao,
    perfil: PerfilHttp,
) -> Result<ContextoDeExecucao> {
    let segredo = perfil
        .credencial
        .map(|credencial| obrigatoria(credencial.variavel))
        .transpose()?;
    Ok(ContextoDeExecucao {
        http: Arc::new(ClienteHttpComRetentativas::com_perfil(
            PoliticaDeRetentativas::default(),
            perfil,
            segredo.as_deref(),
        )?),
        ..contexto.clone()
    })
}

/// Mostra o espaço necessário, recusa o que não cabe, pede confirmação e executa.
async fn executar(
    selecionadas: Vec<FonteSelecionada>,
    contexto: ContextoDeExecucao,
    sem_confirmacao: bool,
) -> Result<bool> {
    let mut dags = Vec::with_capacity(selecionadas.len());
    let mut contextos = Vec::with_capacity(selecionadas.len());
    let mut estimativas = Vec::new();
    for fonte in selecionadas {
        let contexto_da_fonte = contexto_para_fonte(&contexto, fonte.perfil_http)?;
        estimativas.extend(
            estimar(
                &tarefas_de(&fonte.tarefas),
                &contexto_da_fonte,
                PrecisaoDaEstimativa::Exata,
            )
            .await,
        );
        dags.push(DagDeIngestao::construir(&fonte.nome, fonte.tarefas)?);
        contextos.push(contexto_da_fonte);
    }

    let quantidade = estimativas.len();
    let espaco = EspacoNecessario::somar(&estimativas);
    let livre = env::var("TODEOLHO_DIR_DADOS")
        .ok()
        .and_then(|caminho| espaco_livre_em(Path::new(&caminho)));

    if !avisar_sobre_espaco(quantidade, &espaco, livre) {
        return Ok(false);
    }
    if !sem_confirmacao && !confirmar()? {
        println!("Nada foi executado.");
        return Ok(true);
    }

    match contexto.armazenamento.descartar_envios_incompletos().await {
        Ok(0) => {}
        Ok(quantidade) => {
            println!("Espaço recuperado de {quantidade} transferência(s) interrompida(s).")
        }
        Err(erro) => {
            eprintln!("Atenção: não foi possível limpar transferências interrompidas: {erro}")
        }
    }

    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;

    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(
            &contexto.id_execucao,
            Camada::Raw,
            contexto.iniciada_em,
            &dags,
        )?,
        historico,
    ));
    registrador.publicar().await;
    tokio::spawn({
        let registrador = Arc::clone(&registrador);
        async move { registrador.manter_batimento().await }
    });

    let executor = ExecutorDeDag::novo(concorrencia()?);

    let mut sucesso = true;
    for (dag, contexto_da_fonte) in dags.iter().zip(&contextos) {
        let relatorio = executor
            .executar(dag, contexto_da_fonte, registrador.as_ref())
            .await?;
        imprimir(&relatorio);
        sucesso &= relatorio.sucesso();
    }
    registrador.concluir().await;

    println!("\nexecução: {}", contexto.id_execucao);
    Ok(sucesso)
}

/// Decodifica a camada Raw de uma fonte para o bronze (Parquet), reusando o executor e o painel.
async fn refinar(
    camada: String,
    fonte: Option<String>,
    todas: bool,
    intervalo: Option<u64>,
) -> Result<bool> {
    if camada != "bronze" {
        bail!("camada desconhecida: {camada}. Hoje o refino só produz 'bronze'.");
    }
    let fontes = fontes_de_refino(fonte, todas)?;

    // Um cliente S3 concreto para listar os recursos (`listar_chaves` não está no trait) e
    // um por porta para as tarefas; o bronze é a camada de escrita.
    let raw_s3 = ArmazenamentoS3::novo(configuracao_s3("raw")?)?;
    let raw: Arc<dyn ArmazenamentoRaw> = Arc::new(ArmazenamentoS3::novo(configuracao_s3("raw")?)?);
    let bronze: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("bronze")?)?);
    bronze.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;

    match intervalo.filter(|segundos| *segundos > 0) {
        // Rodada única: refina tudo que existe (as reaproveitadas aparecem no painel).
        None => passada(&fontes, &raw_s3, &raw, &bronze, false).await,
        // Reconciliação: repete e, para não poluir o painel, só registra execução nas
        // passadas em que há trabalho (pré-checagem por recurso).
        Some(segundos) => {
            println!(
                "Agendador do bronze: reconciliando {} fonte(s) a cada {segundos}s. Ctrl+C para sair.",
                fontes.len()
            );
            loop {
                if let Err(erro) = passada(&fontes, &raw_s3, &raw, &bronze, true).await {
                    eprintln!("Atenção: falha na passada de reconciliação: {erro:#}");
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

/// Resolve quais fontes do catálogo de refino serão processadas.
fn fontes_de_refino(fonte: Option<String>, todas: bool) -> Result<Vec<FonteDeRefino>> {
    match (fonte, todas) {
        (Some(_), true) => bail!("informe uma fonte ou --todas, não os dois"),
        (None, false) => bail!("informe uma fonte ou --todas"),
        (None, true) => Ok(catalogo_refino()),
        (Some(nome), false) => {
            let catalogo = catalogo_refino();
            let disponiveis = catalogo
                .iter()
                .map(|f| f.nome)
                .collect::<Vec<_>>()
                .join(", ");
            match catalogo.into_iter().find(|f| f.nome == nome) {
                Some(fonte) => Ok(vec![fonte]),
                None => bail!("fonte de refino desconhecida: {nome}. Disponíveis: {disponiveis}"),
            }
        }
    }
}

/// Lista os recursos já ingeridos no raw de uma fonte (os que têm ponteiro vigente).
async fn recursos_no_raw(raw_s3: &ArmazenamentoS3, fonte: &FonteDeRefino) -> Result<Vec<String>> {
    let prefixo = format!("{}/{}", fonte.fonte_raw, fonte.prefixo_recurso);
    let chaves = raw_s3
        .listar_chaves(&prefixo)
        .await
        .context(ERRO_DE_ARMAZENAMENTO)?;
    let prefixo_fonte = format!("{}/", fonte.fonte_raw);
    let mut recursos: Vec<String> = chaves
        .iter()
        .filter_map(|chave| chave.strip_suffix("/_ultima_versao.json"))
        .filter_map(|recurso| recurso.strip_prefix(&prefixo_fonte))
        .map(str::to_owned)
        .collect();
    recursos.sort();
    recursos.dedup();
    Ok(recursos)
}

/// Uma passada de refino sobre as `fontes`. Com `so_pendentes`, pula os recursos cujo
/// bronze já está na versão vigente do raw e, se nada sobrar, nem registra execução.
async fn passada(
    fontes: &[FonteDeRefino],
    raw_s3: &ArmazenamentoS3,
    raw: &Arc<dyn ArmazenamentoRaw>,
    bronze: &Arc<dyn ArmazenamentoRaw>,
    so_pendentes: bool,
) -> Result<bool> {
    let mut dags = Vec::new();
    for fonte in fontes {
        let decodificador = fonte.decodificador();
        let mut tarefas = Vec::new();
        for recurso in recursos_no_raw(raw_s3, fonte).await? {
            let tarefa = TarefaDeBronze::nova(
                Arc::clone(raw),
                fonte.fonte_raw,
                recurso,
                Arc::clone(&decodificador),
            );
            if so_pendentes
                && !tarefa
                    .pendente(bronze.as_ref())
                    .await
                    .context(ERRO_DE_ARMAZENAMENTO)?
            {
                continue;
            }
            tarefas.push(TarefaPlanejada::nova(tarefa));
        }
        if !tarefas.is_empty() {
            dags.push(DagDeIngestao::construir(fonte.nome, tarefas)?);
        }
    }

    if dags.is_empty() {
        println!("Nada a refinar (recursos em dia ou não ingeridos).");
        return Ok(true);
    }

    let iniciada_em = Utc::now();
    let contexto = ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            &Uuid::new_v4().simple().to_string()[..8]
        ),
        iniciada_em,
        armazenamento: Arc::clone(bronze),
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    };

    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(&contexto.id_execucao, Camada::Bronze, iniciada_em, &dags)?,
        historico,
    ));
    registrador.publicar().await;
    tokio::spawn({
        let registrador = Arc::clone(&registrador);
        async move { registrador.manter_batimento().await }
    });

    let executor = ExecutorDeDag::novo(concorrencia()?);
    let mut sucesso = true;
    for dag in &dags {
        let relatorio = executor
            .executar(dag, &contexto, registrador.as_ref())
            .await?;
        imprimir(&relatorio);
        sucesso &= relatorio.sucesso();
    }
    registrador.concluir().await;
    println!("\nexecução: {}", contexto.id_execucao);
    Ok(sucesso)
}

/// Devolve `false` quando a seleção não cabe no disco.
fn avisar_sobre_espaco(quantidade: usize, espaco: &EspacoNecessario, livre: Option<u64>) -> bool {
    println!("\nSelecionado: {quantidade} tarefa(s)");
    println!("Falta baixar (estimado): {}", formatar_bytes(espaco.bytes));
    if espaco.maior_arquivo > 0 {
        println!(
            "Pico durante a gravação: {} (o maior arquivo ocupa o dobro enquanto é gravado)",
            formatar_bytes(espaco.pico())
        );
    }
    if espaco.sem_estimativa > 0 {
        println!(
            "Atenção: {} tarefa(s) sem tamanho conhecido. O espaço real pode ser maior.",
            espaco.sem_estimativa
        );
    }
    if let Some(livre) = livre {
        println!("Espaço livre: {}", formatar_bytes(livre));
    }

    match espaco.avaliar(livre) {
        AvaliacaoDeEspaco::Cabe => true,
        AvaliacaoDeEspaco::Desconhecido => {
            println!("Atenção: não foi possível verificar o espaço livre em disco.");
            true
        }
        AvaliacaoDeEspaco::NaoCabe { faltam } => {
            eprintln!(
                "\nNão cabe: faltam {}. Escolha menos dados ou libere espaço em disco.",
                formatar_bytes(faltam)
            );
            false
        }
    }
}

fn confirmar() -> Result<bool> {
    if !io::stdin().is_terminal() {
        bail!("sem terminal para confirmar. Use --sim para executar sem a pergunta");
    }

    print!("\nContinuar? [s/N] ");
    io::stdout().flush()?;
    let mut resposta = String::new();
    io::stdin().read_line(&mut resposta)?;
    Ok(matches!(
        resposta.trim().to_lowercase().as_str(),
        "s" | "sim"
    ))
}

fn tarefas_de(tarefas: &[TarefaPlanejada]) -> Vec<Arc<dyn Tarefa>> {
    tarefas
        .iter()
        .map(|tarefa| Arc::clone(&tarefa.tarefa))
        .collect()
}

fn historico() -> Result<HistoricoS3> {
    Ok(HistoricoS3::novo(configuracao_s3("execucoes")?)?)
}

fn imprimir(relatorio: &RelatorioDaExecucao) {
    println!("\nfonte: {}", relatorio.fonte);
    for resultado in &relatorio.resultados {
        let situacao = match &resultado.estado {
            EstadoDaTarefa::Concluida(resumo) if resumo.reaproveitada => {
                "ok (sem alterações na origem)".to_owned()
            }
            EstadoDaTarefa::Concluida(resumo) => {
                format!("ok ({} parte(s), {} bytes)", resumo.partes, resumo.bytes)
            }
            EstadoDaTarefa::Falhou(erro) => format!("FALHOU: {erro}"),
            EstadoDaTarefa::Ignorada { causa } => format!("IGNORADA: depende de {causa}"),
        };
        println!(
            "  {:<14} {:>6.1}s  {situacao}",
            resultado.nome,
            resultado.duracao.as_secs_f64()
        );
    }
}

/// Configuração do armazenamento para um dos buckets: `raw` ou `execucoes`.
fn configuracao_s3(bucket: &str) -> Result<ConfiguracaoS3> {
    let variavel_do_bucket = format!("TODEOLHO_S3_BUCKET_{}", bucket.to_uppercase());
    Ok(ConfiguracaoS3 {
        endpoint: obrigatoria("TODEOLHO_S3_ENDPOINT")?,
        regiao: env::var("TODEOLHO_S3_REGIAO").unwrap_or_else(|_| "sa-east-1".to_owned()),
        bucket: env::var(variavel_do_bucket).unwrap_or_else(|_| bucket.to_owned()),
        chave_de_acesso: obrigatoria("TODEOLHO_S3_CHAVE_DE_ACESSO")?,
        chave_secreta: obrigatoria("TODEOLHO_S3_CHAVE_SECRETA")?,
    })
}

fn obrigatoria(nome: &str) -> Result<String> {
    env::var(nome).with_context(|| format!("variável de ambiente ausente: {nome}"))
}

fn concorrencia() -> Result<usize> {
    match env::var("TODEOLHO_CONCORRENCIA") {
        Ok(valor) => valor
            .parse()
            .with_context(|| format!("TODEOLHO_CONCORRENCIA inválida: {valor}")),
        Err(_) => Ok(2),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use todeolho_fontes::{FonteIbge, FonteTse};

    #[tokio::test]
    async fn fonte_sob_demanda_tambem_estima_tamanho_ao_abrir_menu() {
        let fonte = FonteTse;
        let tarefas = fonte.criar_tarefas();
        let estimativas = vec![Some(7); tarefas.len()];
        let opcao = montar_opcao_de_menu(&fonte, &tarefas, async { estimativas }).await;
        assert_eq!(opcao.tarefas.len(), tarefas.len());
        assert!(opcao.tarefas.iter().all(|tarefa| tarefa.bytes == Some(7)));
    }

    #[tokio::test]
    async fn fonte_pequena_mostra_estimativas_no_menu() {
        let fonte = FonteIbge;
        let tarefas = fonte.criar_tarefas();
        let estimativas = vec![Some(42); tarefas.len()];
        let opcao = montar_opcao_de_menu(&fonte, &tarefas, async { estimativas }).await;
        assert_eq!(opcao.tarefas.len(), tarefas.len());
        assert!(opcao.tarefas.iter().all(|tarefa| tarefa.bytes == Some(42)));
    }
}
