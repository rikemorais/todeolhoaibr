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
use todeolho_refino::{
    FonteDeRefino, SnapshotBronze, TarefaDeBronze, TarefaDeSilverBcb,
    TarefaDeSilverCamaraDeputados, TarefaDeSilverCamaraDespesas, TarefaDeSilverCamaraLegislaturas,
    TarefaDeSilverCamaraPartidos, TarefaDeSilverCamaraTramitacoes, TarefaDeSilverCamaraVotacoes,
    TarefaDeSilverIbge, TarefaDeSilverIbgeLocalidades, TarefaDeSilverIpeadata,
    TarefaDeSilverSenadoDespesas, TarefaDeSilverSenadoParlamentares, TarefaDeSilverSenadoProcessos,
    TarefaDeSilverSenadoProcessosDetalhes, TarefaDeSilverSenadoVotacoes, TarefaDeSilverTseBens,
    TarefaDeSilverTseCandidaturas, TarefaDeSilverTseDespesasContratadas,
    TarefaDeSilverTseDespesasContratadas2020, TarefaDeSilverTseDespesasContratadas2022,
    TarefaDeSilverTseDespesasPagas, TarefaDeSilverTseDespesasPagas2020,
    TarefaDeSilverTseDespesasPagas2022, TarefaDeSilverTseReceitasCandidatos,
    TarefaDeSilverTseReceitasCandidatos2020, TarefaDeSilverTseReceitasCandidatos2022,
    TarefaDeSilverTseReceitasDoadorOriginario, TarefaDeSilverTseReceitasDoadorOriginario2020,
    TarefaDeSilverTseReceitasDoadorOriginario2022, TarefaDeSilverTseVotacaoSecao,
    TipoLocalidadeIbge, catalogo_refino,
};
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
    /// Refina dados para o medalhão (`bronze` ou `silver bcb`, em Parquet)
    Refinar {
        /// Camada de destino (`bronze` ou `silver`)
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
        /// Restringe a votação por seção, como --filtro ano=2024 --filtro uf=AC,RO
        #[arg(long = "filtro", value_name = "NOME=VALORES")]
        filtros: Vec<String>,
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
            filtros,
        } => refinar(camada, fonte, todas, intervalo, filtros).await,
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
            Uuid::new_v4().simple()
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
    filtros: Vec<String>,
) -> Result<bool> {
    if camada == "silver" {
        if fonte.as_deref() == Some("tse-prestacao-contas") {
            if !filtros.is_empty() {
                bail!("--filtro não está disponível para `silver tse-prestacao-contas`");
            }
            return refinar_tse_prestacao_contas_silver(fonte, todas, intervalo).await;
        }
        if fonte.as_deref() == Some("tse-votacao-secao") {
            return refinar_tse_votacao_secao_silver(fonte, todas, intervalo, filtros).await;
        }
        if !filtros.is_empty() {
            bail!("--filtro só vale para `silver tse-votacao-secao`");
        }
        if todas {
            if fonte.is_some() {
                bail!("informe uma fonte ou --todas, não os dois");
            }
            return refinar_todas_silver(intervalo).await;
        }
        if fonte.as_deref() == Some("senado-parlamentares") {
            return refinar_senado_silver(fonte, todas, intervalo, TipoSilverSenado::Parlamentares)
                .await;
        }
        if fonte.as_deref() == Some("senado-votacoes") {
            return refinar_senado_silver(fonte, todas, intervalo, TipoSilverSenado::Votacoes)
                .await;
        }
        if fonte.as_deref() == Some("senado-processos-detalhes") {
            return refinar_senado_silver(
                fonte,
                todas,
                intervalo,
                TipoSilverSenado::ProcessosDetalhes,
            )
            .await;
        }
        if fonte.as_deref() == Some("senado-processos") {
            return refinar_senado_silver(fonte, todas, intervalo, TipoSilverSenado::Processos)
                .await;
        }
        if fonte.as_deref() == Some("senado-despesas") {
            return refinar_senado_silver(fonte, todas, intervalo, TipoSilverSenado::Despesas)
                .await;
        }
        if fonte.as_deref() == Some("tse-candidaturas") {
            return refinar_tse_silver(fonte, todas, intervalo, TipoSilverTse::Candidaturas).await;
        }
        if fonte.as_deref() == Some("tse-bens") {
            return refinar_tse_silver(fonte, todas, intervalo, TipoSilverTse::Bens).await;
        }
        if fonte.as_deref() == Some("camara-partidos") {
            return refinar_camara_silver(fonte, todas, intervalo, TipoSilverCamara::Partidos)
                .await;
        }
        if fonte.as_deref() == Some("camara-despesas") {
            return refinar_camara_silver(fonte, todas, intervalo, TipoSilverCamara::Despesas)
                .await;
        }
        if fonte.as_deref() == Some("camara-legislaturas") {
            return refinar_camara_silver(fonte, todas, intervalo, TipoSilverCamara::Legislaturas)
                .await;
        }
        if fonte.as_deref() == Some("camara-deputados") {
            return refinar_camara_silver(fonte, todas, intervalo, TipoSilverCamara::Deputados)
                .await;
        }
        if fonte.as_deref() == Some("camara-votacoes") {
            return refinar_camara_silver(fonte, todas, intervalo, TipoSilverCamara::Votacoes)
                .await;
        }
        if fonte.as_deref() == Some("camara-tramitacoes") {
            return refinar_camara_silver(fonte, todas, intervalo, TipoSilverCamara::Tramitacoes)
                .await;
        }
        if fonte.as_deref() == Some("ibge-estados") {
            return refinar_ibge_localidade_silver(
                fonte,
                todas,
                intervalo,
                TipoLocalidadeIbge::Estados,
            )
            .await;
        }
        if fonte.as_deref() == Some("ibge-regioes") {
            return refinar_ibge_localidade_silver(
                fonte,
                todas,
                intervalo,
                TipoLocalidadeIbge::Regioes,
            )
            .await;
        }
        if fonte.as_deref() == Some("ibge-municipios") {
            return refinar_ibge_localidade_silver(
                fonte,
                todas,
                intervalo,
                TipoLocalidadeIbge::Municipios,
            )
            .await;
        }
        if fonte.as_deref() == Some("ipeadata") {
            return refinar_ipeadata_silver(fonte, todas, intervalo).await;
        }
        if fonte
            .as_deref()
            .is_some_and(|nome| nome.starts_with("ibge-"))
        {
            return refinar_ibge_silver(fonte, todas, intervalo).await;
        }
        return refinar_bcb_silver(fonte, todas, intervalo).await;
    }
    if !filtros.is_empty() {
        bail!("--filtro só vale para `silver tse-votacao-secao`");
    }
    if camada != "bronze" {
        bail!(
            "camada desconhecida: {camada}. Disponíveis: bronze e silver (BCB, IBGE, Câmara, Senado e TSE)."
        );
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

/// Publica as tabelas de prestação de contas do TSE que têm contrato Silver validado.
async fn refinar_tse_prestacao_contas_silver(
    fonte: Option<String>,
    todas: bool,
    intervalo: Option<u64>,
) -> Result<bool> {
    if todas || fonte.as_deref() != Some("tse-prestacao-contas") {
        bail!(
            "prestação de contas deve ser selecionada explicitamente: silver tse-prestacao-contas"
        );
    }
    let bronze: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("bronze")?)?);
    let silver: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("silver")?)?);
    bronze.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    silver.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    match intervalo.filter(|segundos| *segundos > 0) {
        None => passada_tse_prestacao_contas_silver(&bronze, &silver, false).await,
        Some(segundos) => {
            println!(
                "Agendador Silver prestação de contas TSE: reconciliando a cada {segundos}s. Ctrl+C para sair."
            );
            loop {
                if let Err(erro) = passada_tse_prestacao_contas_silver(&bronze, &silver, true).await
                {
                    eprintln!(
                        "Atenção: falha na reconciliação Silver da prestação de contas: {erro:#}"
                    );
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

async fn passada_tse_prestacao_contas_silver(
    bronze: &Arc<dyn ArmazenamentoRaw>,
    silver: &Arc<dyn ArmazenamentoRaw>,
    somente_pendentes: bool,
) -> Result<bool> {
    let mut tarefas = Vec::new();

    const RECURSO_2020: &str = "prestacao-contas/ano=2020";
    if let Some(snapshot) = SnapshotBronze::vigente(bronze.as_ref(), "tse", RECURSO_2020)
        .await
        .context("não foi possível ler snapshot Bronze da prestação de contas de 2020")?
    {
        let snapshot = Arc::new(snapshot);
        let tarefa_despesas_contratadas = TarefaDeSilverTseDespesasContratadas2020::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2020,
        )
        .expect("o contrato aceita somente a prestação de contas de 2020");
        if !somente_pendentes
            || tarefa_despesas_contratadas
                .pendente(silver.as_ref())
                .await?
        {
            tarefas.push(TarefaPlanejada::nova(tarefa_despesas_contratadas));
        }
        let tarefa_despesas_pagas = TarefaDeSilverTseDespesasPagas2020::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2020,
        )
        .expect("o contrato aceita somente a prestação de contas de 2020");
        if !somente_pendentes || tarefa_despesas_pagas.pendente(silver.as_ref()).await? {
            tarefas.push(TarefaPlanejada::nova(tarefa_despesas_pagas));
        }
        let tarefa_receitas_candidatos = TarefaDeSilverTseReceitasCandidatos2020::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2020,
        )
        .expect("o contrato aceita somente a prestação de contas de 2020");
        if !somente_pendentes || tarefa_receitas_candidatos.pendente(silver.as_ref()).await? {
            tarefas.push(TarefaPlanejada::nova(tarefa_receitas_candidatos));
        }
        let tarefa_doador_originario = TarefaDeSilverTseReceitasDoadorOriginario2020::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2020,
        )
        .expect("o contrato aceita somente a prestação de contas de 2020");
        if !somente_pendentes || tarefa_doador_originario.pendente(silver.as_ref()).await? {
            tarefas.push(TarefaPlanejada::nova(tarefa_doador_originario));
        }
    } else {
        println!("Bronze 2020 ausente; prestação de contas de 2020 não será refinada.");
    }

    const RECURSO_2022: &str = "prestacao-contas/ano=2022";
    if let Some(snapshot) = SnapshotBronze::vigente(bronze.as_ref(), "tse", RECURSO_2022)
        .await
        .context("não foi possível ler snapshot Bronze da prestação de contas de 2022")?
    {
        let snapshot = Arc::new(snapshot);
        let tarefa_despesas_contratadas = TarefaDeSilverTseDespesasContratadas2022::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2022,
        )
        .expect("o contrato aceita somente a prestação de contas de 2022");
        if !somente_pendentes
            || tarefa_despesas_contratadas
                .pendente(silver.as_ref())
                .await?
        {
            tarefas.push(TarefaPlanejada::nova(tarefa_despesas_contratadas));
        }
        let tarefa_despesas_pagas = TarefaDeSilverTseDespesasPagas2022::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2022,
        )
        .expect("o contrato aceita somente a prestação de contas de 2022");
        if !somente_pendentes || tarefa_despesas_pagas.pendente(silver.as_ref()).await? {
            tarefas.push(TarefaPlanejada::nova(tarefa_despesas_pagas));
        }
        let tarefa_receitas_candidatos = TarefaDeSilverTseReceitasCandidatos2022::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2022,
        )
        .expect("o contrato aceita somente a prestação de contas de 2022");
        if !somente_pendentes || tarefa_receitas_candidatos.pendente(silver.as_ref()).await? {
            tarefas.push(TarefaPlanejada::nova(tarefa_receitas_candidatos));
        }
        let tarefa = TarefaDeSilverTseReceitasDoadorOriginario2022::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2022,
        )
        .expect("o contrato aceita somente a prestação de contas de 2022");
        if !somente_pendentes || tarefa.pendente(silver.as_ref()).await? {
            tarefas.push(TarefaPlanejada::nova(tarefa));
        }
    } else {
        println!(
            "Bronze 2022 ausente; contratos de prestação de contas de 2022 não serão refinados."
        );
    }

    const RECURSO_2024: &str = "prestacao-contas/ano=2024";
    if let Some(snapshot) = SnapshotBronze::vigente(bronze.as_ref(), "tse", RECURSO_2024)
        .await
        .context("não foi possível ler snapshot Bronze da prestação de contas de 2024")?
    {
        let snapshot = Arc::new(snapshot);
        let tarefa_despesas = TarefaDeSilverTseDespesasPagas::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2024,
        )
        .expect("o contrato aceita somente a prestação de contas de 2024");
        let tarefa_despesas_contratadas = TarefaDeSilverTseDespesasContratadas::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2024,
        )
        .expect("o contrato aceita somente a prestação de contas de 2024");
        let tarefa_receitas = TarefaDeSilverTseReceitasCandidatos::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2024,
        )
        .expect("o contrato aceita somente a prestação de contas de 2024");
        let tarefa_doador_originario = TarefaDeSilverTseReceitasDoadorOriginario::nova(
            Arc::clone(bronze),
            Arc::clone(&snapshot),
            RECURSO_2024,
        )
        .expect("o contrato aceita somente a prestação de contas de 2024");
        if !somente_pendentes || tarefa_despesas.pendente(silver.as_ref()).await? {
            tarefas.push(TarefaPlanejada::nova(tarefa_despesas));
        }
        if !somente_pendentes
            || tarefa_despesas_contratadas
                .pendente(silver.as_ref())
                .await?
        {
            tarefas.push(TarefaPlanejada::nova(tarefa_despesas_contratadas));
        }
        if !somente_pendentes || tarefa_receitas.pendente(silver.as_ref()).await? {
            tarefas.push(TarefaPlanejada::nova(tarefa_receitas));
        }
        if !somente_pendentes || tarefa_doador_originario.pendente(silver.as_ref()).await? {
            tarefas.push(TarefaPlanejada::nova(tarefa_doador_originario));
        }
    } else {
        println!(
            "Bronze 2024 ausente; tabelas de prestação de contas de 2024 não serão refinadas."
        );
    }
    if tarefas.is_empty() {
        println!("Nada a refinar na Silver da prestação de contas TSE (recursos em dia).");
        return Ok(true);
    }
    let dag = DagDeIngestao::construir("tse-prestacao-contas", tarefas)?;
    let iniciada_em = Utc::now();
    let contexto = ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4().simple()
        ),
        iniciada_em,
        armazenamento: Arc::clone(silver),
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    };
    let dags = vec![dag];
    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(&contexto.id_execucao, Camada::Silver, iniciada_em, &dags)?,
        historico,
    ));
    registrador.publicar().await;
    tokio::spawn({
        let registrador = Arc::clone(&registrador);
        async move { registrador.manter_batimento().await }
    });
    let relatorio = ExecutorDeDag::novo(concorrencia()?)
        .executar(&dags[0], &contexto, registrador.as_ref())
        .await?;
    imprimir(&relatorio);
    registrador.concluir().await;
    println!("\nexecução Silver: {}", contexto.id_execucao);
    Ok(relatorio.sucesso())
}

/// Refina votos por seção de forma explícita, pois os recursos somam mais de 16 GB.
async fn refinar_tse_votacao_secao_silver(
    fonte: Option<String>,
    todas: bool,
    intervalo: Option<u64>,
    filtros: Vec<String>,
) -> Result<bool> {
    if todas || fonte.as_deref() != Some("tse-votacao-secao") {
        bail!("votação por seção deve ser selecionada explicitamente: silver tse-votacao-secao");
    }
    let (anos, ufs) = filtros_tse_votacao_secao(&filtros)?;
    let definicao = catalogo_refino()
        .into_iter()
        .find(|item| item.nome == "tse-votacao-secao")
        .context("fonte tse-votacao-secao ausente do catálogo de refino")?;
    let raw_s3 = ArmazenamentoS3::novo(configuracao_s3("raw")?)?;
    let bronze: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("bronze")?)?);
    let silver: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("silver")?)?);
    bronze.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    silver.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    match intervalo.filter(|segundos| *segundos > 0) {
        None => {
            passada_tse_votacao_secao(&definicao, &raw_s3, &bronze, &silver, false, &anos, &ufs)
                .await
        }
        Some(segundos) => {
            println!(
                "Agendador Silver votação por seção: reconciliando a cada {segundos}s. Ctrl+C para sair."
            );
            loop {
                if let Err(erro) = passada_tse_votacao_secao(
                    &definicao, &raw_s3, &bronze, &silver, true, &anos, &ufs,
                )
                .await
                {
                    eprintln!("Atenção: falha na reconciliação da Silver TSE: {erro:#}");
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

fn filtros_tse_votacao_secao(filtros: &[String]) -> Result<(Vec<u16>, Vec<String>)> {
    let mut anos = Vec::new();
    let mut ufs = Vec::new();
    for filtro in filtros {
        let (nome, valores) = filtro
            .split_once('=')
            .context("filtro deve usar NOME=VALORES")?;
        if nome != "ano" && nome != "uf" {
            bail!("filtro desconhecido para votação por seção: {nome}; use ano e uf");
        }
        for valor in valores.split(',') {
            if valor.is_empty() {
                bail!("valor vazio no filtro {nome}");
            }
            match nome {
                "ano" => anos.push(
                    valor
                        .parse()
                        .with_context(|| format!("ano inválido: {valor}"))?,
                ),
                "uf" if valor.len() == 2 && valor.bytes().all(|b| b.is_ascii_uppercase()) => {
                    ufs.push(valor.to_owned())
                }
                "uf" => bail!("UF inválida: {valor}; use sigla maiúscula de dois caracteres"),
                _ => unreachable!(),
            }
        }
    }
    anos.sort_unstable();
    anos.dedup();
    ufs.sort();
    ufs.dedup();
    Ok((anos, ufs))
}

async fn passada_tse_votacao_secao(
    fonte: &todeolho_refino::FonteDeRefino,
    raw_s3: &ArmazenamentoS3,
    bronze: &Arc<dyn ArmazenamentoRaw>,
    silver: &Arc<dyn ArmazenamentoRaw>,
    somente_pendentes: bool,
    anos: &[u16],
    ufs: &[String],
) -> Result<bool> {
    let mut tarefas = Vec::new();
    for recurso in recursos_no_raw(raw_s3, fonte).await? {
        let Some((ano, uf)) =
            todeolho_refino::silver::tarefa_tse_votacao_secao::ano_uf_do_recurso(&recurso)
        else {
            continue;
        };
        if (!anos.is_empty() && !anos.contains(&ano)) || (!ufs.is_empty() && !ufs.contains(&uf)) {
            continue;
        }
        let Some(tarefa) = TarefaDeSilverTseVotacaoSecao::nova(
            Arc::clone(bronze),
            Arc::new(
                match SnapshotBronze::vigente(bronze.as_ref(), "tse", &recurso)
                    .await
                    .context("não foi possível ler snapshot Bronze")?
                {
                    Some(snapshot) => snapshot,
                    None => continue,
                },
            ),
            &recurso,
        ) else {
            continue;
        };
        if somente_pendentes && !tarefa.pendente(silver.as_ref()).await? {
            continue;
        }
        tarefas.push(TarefaPlanejada::nova(tarefa));
    }
    if tarefas.is_empty() {
        println!(
            "Nada a refinar na Silver de votação por seção (recursos filtrados ausentes ou em dia)."
        );
        return Ok(true);
    }
    let dag = DagDeIngestao::construir("tse-votacao-secao", tarefas)?;
    let iniciada_em = Utc::now();
    let contexto = ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4().simple()
        ),
        iniciada_em,
        armazenamento: Arc::clone(silver),
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    };
    let dags = vec![dag];
    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(&contexto.id_execucao, Camada::Silver, iniciada_em, &dags)?,
        historico,
    ));
    registrador.publicar().await;
    tokio::spawn({
        let registrador = Arc::clone(&registrador);
        async move { registrador.manter_batimento().await }
    });
    let relatorio = ExecutorDeDag::novo(concorrencia()?)
        .executar(&dags[0], &contexto, registrador.as_ref())
        .await?;
    imprimir(&relatorio);
    registrador.concluir().await;
    println!("\nexecução Silver: {}", contexto.id_execucao);
    Ok(relatorio.sucesso())
}

/// Reconcilia todas as fontes Silver atualmente suportadas no mesmo agendador.
async fn refinar_todas_silver(intervalo: Option<u64>) -> Result<bool> {
    match intervalo.filter(|segundos| *segundos > 0) {
        None => {
            let bcb = refinar_bcb_silver(None, true, None).await?;
            let ibge = refinar_ibge_silver(None, true, None).await?;
            let senado =
                refinar_senado_silver(None, true, None, TipoSilverSenado::Parlamentares).await?;
            let senado_votacoes =
                refinar_senado_silver(None, true, None, TipoSilverSenado::Votacoes).await?;
            let senado_processos_detalhes =
                refinar_senado_silver(None, true, None, TipoSilverSenado::ProcessosDetalhes)
                    .await?;
            let senado_processos =
                refinar_senado_silver(None, true, None, TipoSilverSenado::Processos).await?;
            let senado_despesas =
                refinar_senado_silver(None, true, None, TipoSilverSenado::Despesas).await?;
            let tse_candidaturas =
                refinar_tse_silver(None, true, None, TipoSilverTse::Candidaturas).await?;
            let tse_bens = refinar_tse_silver(None, true, None, TipoSilverTse::Bens).await?;
            let camara_partidos =
                refinar_camara_silver(None, true, None, TipoSilverCamara::Partidos).await?;
            let camara_legislaturas =
                refinar_camara_silver(None, true, None, TipoSilverCamara::Legislaturas).await?;
            let camara_deputados =
                refinar_camara_silver(None, true, None, TipoSilverCamara::Deputados).await?;
            let camara_votacoes =
                refinar_camara_silver(None, true, None, TipoSilverCamara::Votacoes).await?;
            let camara_despesas =
                refinar_camara_silver(None, true, None, TipoSilverCamara::Despesas).await?;
            let camara_tramitacoes =
                refinar_camara_silver(None, true, None, TipoSilverCamara::Tramitacoes).await?;
            let ibge_regioes = refinar_ibge_localidade_silver(
                Some("ibge-regioes".into()),
                false,
                None,
                TipoLocalidadeIbge::Regioes,
            )
            .await?;
            let ibge_estados = refinar_ibge_localidade_silver(
                Some("ibge-estados".into()),
                false,
                None,
                TipoLocalidadeIbge::Estados,
            )
            .await?;
            let ibge_municipios = refinar_ibge_localidade_silver(
                Some("ibge-municipios".into()),
                false,
                None,
                TipoLocalidadeIbge::Municipios,
            )
            .await?;
            let ipeadata = refinar_ipeadata_silver(Some("ipeadata".into()), false, None).await?;
            Ok(bcb
                && ibge
                && senado
                && senado_votacoes
                && senado_processos_detalhes
                && senado_processos
                && senado_despesas
                && tse_candidaturas
                && tse_bens
                && camara_partidos
                && camara_legislaturas
                && camara_deputados
                && camara_votacoes
                && camara_despesas
                && camara_tramitacoes
                && ibge_regioes
                && ibge_estados
                && ibge_municipios
                && ipeadata)
        }
        Some(segundos) => {
            println!(
                "Agendador Silver: reconciliando todas as fontes a cada {segundos}s. Ctrl+C para sair."
            );
            loop {
                if let Err(erro) = refinar_bcb_silver(None, true, None).await {
                    eprintln!("Atenção: falha na reconciliação Silver BCB: {erro:#}");
                }
                if let Err(erro) = refinar_ibge_silver(None, true, None).await {
                    eprintln!("Atenção: falha na reconciliação Silver IBGE: {erro:#}");
                }
                if let Err(erro) =
                    refinar_senado_silver(None, true, None, TipoSilverSenado::Parlamentares).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Senado: {erro:#}");
                }
                if let Err(erro) =
                    refinar_senado_silver(None, true, None, TipoSilverSenado::Processos).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Senado catálogo: {erro:#}");
                }
                if let Err(erro) =
                    refinar_senado_silver(None, true, None, TipoSilverSenado::Votacoes).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Senado votações: {erro:#}");
                }
                if let Err(erro) =
                    refinar_senado_silver(None, true, None, TipoSilverSenado::ProcessosDetalhes)
                        .await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Senado processos: {erro:#}");
                }
                if let Err(erro) =
                    refinar_senado_silver(None, true, None, TipoSilverSenado::Despesas).await
                {
                    eprintln!(
                        "Atenção: falha na reconciliação Silver despesas do Senado: {erro:#}"
                    );
                }
                if let Err(erro) =
                    refinar_tse_silver(None, true, None, TipoSilverTse::Candidaturas).await
                {
                    eprintln!(
                        "Atenção: falha na reconciliação Silver candidaturas do TSE: {erro:#}"
                    );
                }
                if let Err(erro) = refinar_tse_silver(None, true, None, TipoSilverTse::Bens).await {
                    eprintln!("Atenção: falha na reconciliação Silver bens do TSE: {erro:#}");
                }
                if let Err(erro) =
                    refinar_camara_silver(None, true, None, TipoSilverCamara::Partidos).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Câmara: {erro:#}");
                }
                if let Err(erro) =
                    refinar_camara_silver(None, true, None, TipoSilverCamara::Legislaturas).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Câmara: {erro:#}");
                }
                if let Err(erro) =
                    refinar_camara_silver(None, true, None, TipoSilverCamara::Deputados).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Câmara: {erro:#}");
                }
                if let Err(erro) =
                    refinar_camara_silver(None, true, None, TipoSilverCamara::Votacoes).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Câmara votações: {erro:#}");
                }
                if let Err(erro) =
                    refinar_camara_silver(None, true, None, TipoSilverCamara::Despesas).await
                {
                    eprintln!(
                        "Atenção: falha na reconciliação Silver despesas da Câmara: {erro:#}"
                    );
                }
                if let Err(erro) =
                    refinar_camara_silver(None, true, None, TipoSilverCamara::Tramitacoes).await
                {
                    eprintln!(
                        "Atenção: falha na reconciliação Silver tramitações da Câmara: {erro:#}"
                    );
                }
                if let Err(erro) = refinar_ibge_localidade_silver(
                    Some("ibge-regioes".into()),
                    false,
                    None,
                    TipoLocalidadeIbge::Regioes,
                )
                .await
                {
                    eprintln!("Atenção: falha na reconciliação Silver IBGE regiões: {erro:#}");
                }
                if let Err(erro) = refinar_ibge_localidade_silver(
                    Some("ibge-estados".into()),
                    false,
                    None,
                    TipoLocalidadeIbge::Estados,
                )
                .await
                {
                    eprintln!("Atenção: falha na reconciliação Silver IBGE estados: {erro:#}");
                }
                if let Err(erro) = refinar_ibge_localidade_silver(
                    Some("ibge-municipios".into()),
                    false,
                    None,
                    TipoLocalidadeIbge::Municipios,
                )
                .await
                {
                    eprintln!("Atenção: falha na reconciliação Silver IBGE municípios: {erro:#}");
                }
                if let Err(erro) =
                    refinar_ipeadata_silver(Some("ipeadata".into()), false, None).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Ipeadata: {erro:#}");
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

#[derive(Clone, Copy)]
enum TipoSilverTse {
    Candidaturas,
    Bens,
}

impl TipoSilverTse {
    fn nome(self) -> &'static str {
        match self {
            Self::Candidaturas => "tse-candidaturas",
            Self::Bens => "tse-bens",
        }
    }

    fn criar_tarefa(
        self,
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: &str,
    ) -> TarefaSilverTse {
        match self {
            Self::Candidaturas => TarefaSilverTse::Candidaturas(
                TarefaDeSilverTseCandidaturas::nova(bronze, snapshot, recurso)
                    .expect("recurso de candidaturas deve declarar ano"),
            ),
            Self::Bens => TarefaSilverTse::Bens(
                TarefaDeSilverTseBens::nova(bronze, snapshot, recurso)
                    .expect("recurso de bens deve declarar ano"),
            ),
        }
    }
}

enum TarefaSilverTse {
    Candidaturas(TarefaDeSilverTseCandidaturas),
    Bens(TarefaDeSilverTseBens),
}

#[async_trait::async_trait]
impl Tarefa for TarefaSilverTse {
    fn nome(&self) -> &str {
        match self {
            Self::Candidaturas(tarefa) => tarefa.nome(),
            Self::Bens(tarefa) => tarefa.nome(),
        }
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<todeolho_nucleo::ResumoDaTarefa, todeolho_nucleo::ErroDeIngestao> {
        match self {
            Self::Candidaturas(tarefa) => tarefa.executar(contexto).await,
            Self::Bens(tarefa) => tarefa.executar(contexto).await,
        }
    }
}

impl TarefaSilverTse {
    async fn pendente(
        &self,
        silver: &dyn ArmazenamentoRaw,
    ) -> Result<bool, todeolho_nucleo::ErroDeIngestao> {
        match self {
            Self::Candidaturas(tarefa) => tarefa.pendente(silver).await,
            Self::Bens(tarefa) => tarefa.pendente(silver).await,
        }
    }
}

/// Publica e reconcilia os consolidados anuais TSE suportados na Silver.
async fn refinar_tse_silver(
    fonte: Option<String>,
    todas: bool,
    intervalo: Option<u64>,
    tipo: TipoSilverTse,
) -> Result<bool> {
    if fonte.as_deref().is_some_and(|nome| nome != tipo.nome()) {
        bail!("a camada silver aceita a fonte '{}'", tipo.nome());
    }
    if todas && fonte.is_some() {
        bail!("informe uma fonte ou --todas, não os dois");
    }
    let definicao = catalogo_refino()
        .into_iter()
        .find(|item| item.nome == tipo.nome())
        .with_context(|| format!("fonte {} ausente do catálogo de refino", tipo.nome()))?;
    let raw_s3 = ArmazenamentoS3::novo(configuracao_s3("raw")?)?;
    let bronze: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("bronze")?)?);
    let silver: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("silver")?)?);
    bronze.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    silver.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    match intervalo.filter(|segundos| *segundos > 0) {
        None => passada_tse_silver(&definicao, &raw_s3, &bronze, &silver, false, tipo).await,
        Some(segundos) => {
            println!(
                "Agendador Silver TSE: reconciliando {} a cada {segundos}s. Ctrl+C para sair.",
                tipo.nome()
            );
            loop {
                if let Err(erro) =
                    passada_tse_silver(&definicao, &raw_s3, &bronze, &silver, true, tipo).await
                {
                    eprintln!(
                        "Atenção: falha na reconciliação Silver {}: {erro:#}",
                        tipo.nome()
                    );
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

async fn passada_tse_silver(
    fonte: &FonteDeRefino,
    raw_s3: &ArmazenamentoS3,
    bronze: &Arc<dyn ArmazenamentoRaw>,
    silver: &Arc<dyn ArmazenamentoRaw>,
    somente_pendentes: bool,
    tipo: TipoSilverTse,
) -> Result<bool> {
    let mut tarefas = Vec::new();
    for recurso in recursos_no_raw(raw_s3, fonte).await? {
        let Some(snapshot) = SnapshotBronze::vigente(bronze.as_ref(), "tse", &recurso)
            .await
            .with_context(|| format!("não foi possível ler o snapshot Bronze {}", tipo.nome()))?
        else {
            continue;
        };
        let tarefa = tipo.criar_tarefa(Arc::clone(bronze), Arc::new(snapshot), &recurso);
        if somente_pendentes && !tarefa.pendente(silver.as_ref()).await? {
            continue;
        }
        tarefas.push(TarefaPlanejada::nova(tarefa));
    }
    if tarefas.is_empty() {
        println!(
            "Nada a refinar na Silver {} (recursos em dia ou não ingeridos).",
            tipo.nome()
        );
        return Ok(true);
    }
    let dag = DagDeIngestao::construir(tipo.nome(), tarefas)?;
    let iniciada_em = Utc::now();
    let contexto = ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4().simple()
        ),
        iniciada_em,
        armazenamento: Arc::clone(silver),
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    };
    let dags = vec![dag];
    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(&contexto.id_execucao, Camada::Silver, iniciada_em, &dags)?,
        historico,
    ));
    registrador.publicar().await;
    tokio::spawn({
        let registrador = Arc::clone(&registrador);
        async move { registrador.manter_batimento().await }
    });
    let relatorio = ExecutorDeDag::novo(concorrencia()?)
        .executar(&dags[0], &contexto, registrador.as_ref())
        .await?;
    imprimir(&relatorio);
    registrador.concluir().await;
    println!("\nexecução Silver: {}", contexto.id_execucao);
    Ok(relatorio.sucesso())
}

#[derive(Clone, Copy)]
enum TipoSilverCamara {
    Despesas,
    Tramitacoes,
    Partidos,
    Legislaturas,
    Deputados,
    Votacoes,
}

impl TipoSilverCamara {
    fn nome(self) -> &'static str {
        match self {
            Self::Despesas => "camara-despesas",
            Self::Tramitacoes => "camara-tramitacoes",
            Self::Partidos => "camara-partidos",
            Self::Legislaturas => "camara-legislaturas",
            Self::Deputados => "camara-deputados",
            Self::Votacoes => "camara-votacoes",
        }
    }

    fn criar_tarefa(
        self,
        bronze: Arc<dyn ArmazenamentoRaw>,
        snapshot: Arc<SnapshotBronze>,
        recurso: &str,
    ) -> TarefaSilverCamara {
        match self {
            Self::Despesas => TarefaSilverCamara::Despesas(
                TarefaDeSilverCamaraDespesas::nova(bronze, snapshot, recurso)
                    .expect("recursos da fonte despesas devem declarar ano"),
            ),
            Self::Tramitacoes => TarefaSilverCamara::Tramitacoes(
                TarefaDeSilverCamaraTramitacoes::nova(bronze, snapshot, recurso)
                    .expect("recursos de tramitações devem declarar ano"),
            ),
            Self::Partidos => TarefaSilverCamara::Partidos(TarefaDeSilverCamaraPartidos::nova(
                bronze, snapshot, recurso,
            )),
            Self::Legislaturas => TarefaSilverCamara::Legislaturas(
                TarefaDeSilverCamaraLegislaturas::nova(bronze, snapshot, recurso),
            ),
            Self::Deputados => TarefaSilverCamara::Deputados(TarefaDeSilverCamaraDeputados::nova(
                bronze, snapshot, recurso,
            )),
            Self::Votacoes => TarefaSilverCamara::Votacoes(
                TarefaDeSilverCamaraVotacoes::nova(bronze, snapshot, recurso)
                    .expect("recursos da fonte votacoes devem identificar um dos sete conjuntos"),
            ),
        }
    }
}

enum TarefaSilverCamara {
    Despesas(TarefaDeSilverCamaraDespesas),
    Tramitacoes(TarefaDeSilverCamaraTramitacoes),
    Partidos(TarefaDeSilverCamaraPartidos),
    Legislaturas(TarefaDeSilverCamaraLegislaturas),
    Deputados(TarefaDeSilverCamaraDeputados),
    Votacoes(TarefaDeSilverCamaraVotacoes),
}

#[async_trait::async_trait]
impl Tarefa for TarefaSilverCamara {
    fn nome(&self) -> &str {
        match self {
            Self::Despesas(tarefa) => tarefa.nome(),
            Self::Tramitacoes(tarefa) => tarefa.nome(),
            Self::Partidos(tarefa) => tarefa.nome(),
            Self::Legislaturas(tarefa) => tarefa.nome(),
            Self::Deputados(tarefa) => tarefa.nome(),
            Self::Votacoes(tarefa) => tarefa.nome(),
        }
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<todeolho_nucleo::ResumoDaTarefa, todeolho_nucleo::ErroDeIngestao> {
        match self {
            Self::Despesas(tarefa) => tarefa.executar(contexto).await,
            Self::Tramitacoes(tarefa) => tarefa.executar(contexto).await,
            Self::Partidos(tarefa) => tarefa.executar(contexto).await,
            Self::Legislaturas(tarefa) => tarefa.executar(contexto).await,
            Self::Deputados(tarefa) => tarefa.executar(contexto).await,
            Self::Votacoes(tarefa) => tarefa.executar(contexto).await,
        }
    }
}

impl TarefaSilverCamara {
    async fn pendente(
        &self,
        silver: &dyn ArmazenamentoRaw,
    ) -> Result<bool, todeolho_nucleo::ErroDeIngestao> {
        match self {
            Self::Despesas(tarefa) => tarefa.pendente(silver).await,
            Self::Tramitacoes(tarefa) => tarefa.pendente(silver).await,
            Self::Partidos(tarefa) => tarefa.pendente(silver).await,
            Self::Legislaturas(tarefa) => tarefa.pendente(silver).await,
            Self::Deputados(tarefa) => tarefa.pendente(silver).await,
            Self::Votacoes(tarefa) => tarefa.pendente(silver).await,
        }
    }
}

/// Publica um dos cadastros Câmara atualmente suportados na Silver.
async fn refinar_camara_silver(
    fonte: Option<String>,
    todas: bool,
    intervalo: Option<u64>,
    tipo: TipoSilverCamara,
) -> Result<bool> {
    if fonte.as_deref().is_some_and(|nome| nome != tipo.nome()) {
        bail!("a camada silver aceita a fonte '{}'", tipo.nome());
    }
    if todas && fonte.is_some() {
        bail!("informe uma fonte ou --todas, não os dois");
    }
    let definicao = catalogo_refino()
        .into_iter()
        .find(|item| item.nome == tipo.nome())
        .with_context(|| format!("fonte {} ausente do catálogo de refino", tipo.nome()))?;
    let raw_s3 = ArmazenamentoS3::novo(configuracao_s3("raw")?)?;
    let bronze: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("bronze")?)?);
    let silver: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("silver")?)?);
    bronze.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    silver.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    match intervalo.filter(|segundos| *segundos > 0) {
        None => passada_camara_silver(&definicao, &raw_s3, &bronze, &silver, false, tipo).await,
        Some(segundos) => {
            println!(
                "Agendador Silver Câmara: reconciliando a cada {segundos}s. Ctrl+C para sair."
            );
            loop {
                if let Err(erro) =
                    passada_camara_silver(&definicao, &raw_s3, &bronze, &silver, true, tipo).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Câmara: {erro:#}");
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

async fn passada_camara_silver(
    fonte: &FonteDeRefino,
    raw_s3: &ArmazenamentoS3,
    bronze: &Arc<dyn ArmazenamentoRaw>,
    silver: &Arc<dyn ArmazenamentoRaw>,
    somente_pendentes: bool,
    tipo: TipoSilverCamara,
) -> Result<bool> {
    let mut tarefas = Vec::new();
    for recurso in recursos_no_raw(raw_s3, fonte).await? {
        let Some(snapshot) = SnapshotBronze::vigente(bronze.as_ref(), "camara", &recurso)
            .await
            .context("não foi possível ler o snapshot Bronze Câmara")?
        else {
            continue;
        };
        let tarefa = tipo.criar_tarefa(Arc::clone(bronze), Arc::new(snapshot), &recurso);
        if somente_pendentes && !tarefa.pendente(silver.as_ref()).await? {
            continue;
        }
        tarefas.push(TarefaPlanejada::nova(tarefa));
    }
    if tarefas.is_empty() {
        println!("Nada a refinar na Silver Câmara (recursos em dia ou não ingeridos).");
        return Ok(true);
    }
    let dag = DagDeIngestao::construir(tipo.nome(), tarefas)?;
    let iniciada_em = Utc::now();
    let contexto = ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4().simple()
        ),
        iniciada_em,
        armazenamento: Arc::clone(silver),
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    };
    let dags = vec![dag];
    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(&contexto.id_execucao, Camada::Silver, iniciada_em, &dags)?,
        historico,
    ));
    registrador.publicar().await;
    tokio::spawn({
        let registrador = Arc::clone(&registrador);
        async move { registrador.manter_batimento().await }
    });
    let relatorio = ExecutorDeDag::novo(concorrencia()?)
        .executar(&dags[0], &contexto, registrador.as_ref())
        .await?;
    imprimir(&relatorio);
    registrador.concluir().await;
    println!("\nexecução Silver: {}", contexto.id_execucao);
    Ok(relatorio.sucesso())
}

#[derive(Clone, Copy)]
enum TipoSilverSenado {
    Despesas,
    Parlamentares,
    Votacoes,
    Processos,
    ProcessosDetalhes,
}

impl TipoSilverSenado {
    fn nome(self) -> &'static str {
        match self {
            Self::Despesas => "senado-despesas",
            Self::Parlamentares => "senado-parlamentares",
            Self::Votacoes => "senado-votacoes",
            Self::Processos => "senado-processos",
            Self::ProcessosDetalhes => "senado-processos-detalhes",
        }
    }
}

enum TarefaSilverSenado {
    Despesas(TarefaDeSilverSenadoDespesas),
    Parlamentares(TarefaDeSilverSenadoParlamentares),
    Votacoes(TarefaDeSilverSenadoVotacoes),
    Processos(TarefaDeSilverSenadoProcessos),
    ProcessosDetalhes(TarefaDeSilverSenadoProcessosDetalhes),
}

#[async_trait::async_trait]
impl Tarefa for TarefaSilverSenado {
    fn nome(&self) -> &str {
        match self {
            Self::Despesas(tarefa) => tarefa.nome(),
            Self::Parlamentares(tarefa) => tarefa.nome(),
            Self::Votacoes(tarefa) => tarefa.nome(),
            Self::Processos(tarefa) => tarefa.nome(),
            Self::ProcessosDetalhes(tarefa) => tarefa.nome(),
        }
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<todeolho_nucleo::ResumoDaTarefa, todeolho_nucleo::ErroDeIngestao> {
        match self {
            Self::Despesas(tarefa) => tarefa.executar(contexto).await,
            Self::Parlamentares(tarefa) => tarefa.executar(contexto).await,
            Self::Votacoes(tarefa) => tarefa.executar(contexto).await,
            Self::Processos(tarefa) => tarefa.executar(contexto).await,
            Self::ProcessosDetalhes(tarefa) => tarefa.executar(contexto).await,
        }
    }
}

impl TarefaSilverSenado {
    async fn pendente(
        &self,
        silver: &dyn ArmazenamentoRaw,
    ) -> Result<bool, todeolho_nucleo::ErroDeIngestao> {
        match self {
            Self::Despesas(tarefa) => tarefa.pendente(silver).await,
            Self::Parlamentares(tarefa) => tarefa.pendente(silver).await,
            Self::Votacoes(tarefa) => tarefa.pendente(silver).await,
            Self::Processos(tarefa) => tarefa.pendente(silver).await,
            Self::ProcessosDetalhes(tarefa) => tarefa.pendente(silver).await,
        }
    }
}

/// Publica os contratos Silver anuais ou por legislatura do Senado.
async fn refinar_senado_silver(
    fonte: Option<String>,
    todas: bool,
    intervalo: Option<u64>,
    tipo: TipoSilverSenado,
) -> Result<bool> {
    if fonte.as_deref().is_some_and(|nome| nome != tipo.nome()) {
        bail!("a camada silver aceita a fonte '{}'", tipo.nome());
    }
    if todas && fonte.is_some() {
        bail!("informe uma fonte ou --todas, não os dois");
    }
    let definicao = catalogo_refino()
        .into_iter()
        .find(|item| item.nome == tipo.nome())
        .with_context(|| format!("fonte {} ausente do catálogo de refino", tipo.nome()))?;
    let raw_s3 = ArmazenamentoS3::novo(configuracao_s3("raw")?)?;
    let bronze: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("bronze")?)?);
    let silver: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("silver")?)?);
    bronze.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    silver.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    match intervalo.filter(|segundos| *segundos > 0) {
        None => passada_senado_silver(&definicao, &raw_s3, &bronze, &silver, false, tipo).await,
        Some(segundos) => {
            println!(
                "Agendador Silver Senado: reconciliando a cada {segundos}s. Ctrl+C para sair."
            );
            loop {
                if let Err(erro) =
                    passada_senado_silver(&definicao, &raw_s3, &bronze, &silver, true, tipo).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Senado: {erro:#}");
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

/// Executa uma rodada Silver Senado e omite passadas vazias do painel.
async fn passada_senado_silver(
    fonte: &FonteDeRefino,
    raw_s3: &ArmazenamentoS3,
    bronze: &Arc<dyn ArmazenamentoRaw>,
    silver: &Arc<dyn ArmazenamentoRaw>,
    somente_pendentes: bool,
    tipo: TipoSilverSenado,
) -> Result<bool> {
    let mut tarefas = Vec::new();
    for recurso in recursos_no_raw(raw_s3, fonte).await? {
        let Some(snapshot) = SnapshotBronze::vigente(bronze.as_ref(), "senado", &recurso)
            .await
            .context("não foi possível ler o snapshot Bronze Senado")?
        else {
            continue;
        };
        let snapshot = Arc::new(snapshot);
        let novas = match tipo {
            TipoSilverSenado::Despesas => vec![TarefaSilverSenado::Despesas(
                TarefaDeSilverSenadoDespesas::nova(
                    Arc::clone(bronze),
                    Arc::clone(&snapshot),
                    &recurso,
                )
                .context("recurso de despesas sem ano no caminho")?,
            )],
            TipoSilverSenado::Parlamentares => {
                let legislatura = recurso
                    .strip_prefix("parlamentares/legislatura=")
                    .context("recurso de parlamentares sem legislatura no caminho")?
                    .parse::<u16>()
                    .context("legislatura inválida no caminho do recurso Bronze")?;
                vec![TarefaSilverSenado::Parlamentares(
                    TarefaDeSilverSenadoParlamentares::nova(
                        Arc::clone(bronze),
                        Arc::clone(&snapshot),
                        &recurso,
                        legislatura,
                    ),
                )]
            }
            TipoSilverSenado::Votacoes => {
                let Ok(votos) = TarefaDeSilverSenadoVotacoes::nova_votos(
                    Arc::clone(bronze),
                    Arc::clone(&snapshot),
                    &recurso,
                ) else {
                    eprintln!("Atenção: recurso de votação Senado inválido: {recurso}");
                    continue;
                };
                vec![
                    TarefaSilverSenado::Votacoes(TarefaDeSilverSenadoVotacoes::nova_sessoes(
                        Arc::clone(bronze),
                        Arc::clone(&snapshot),
                        &recurso,
                    )),
                    TarefaSilverSenado::Votacoes(votos),
                ]
            }
            TipoSilverSenado::Processos => {
                vec![TarefaSilverSenado::Processos(
                    TarefaDeSilverSenadoProcessos::nova(
                        Arc::clone(bronze),
                        Arc::clone(&snapshot),
                        &recurso,
                    ),
                )]
            }
            TipoSilverSenado::ProcessosDetalhes => {
                vec![TarefaSilverSenado::ProcessosDetalhes(
                    TarefaDeSilverSenadoProcessosDetalhes::nova(
                        Arc::clone(bronze),
                        Arc::clone(&snapshot),
                        &recurso,
                    ),
                )]
            }
        };
        for tarefa in novas {
            if somente_pendentes && !tarefa.pendente(silver.as_ref()).await? {
                continue;
            }
            tarefas.push(TarefaPlanejada::nova(tarefa));
        }
    }
    if tarefas.is_empty() {
        println!("Nada a refinar na Silver Senado (recursos em dia ou não ingeridos).");
        return Ok(true);
    }
    let dag = DagDeIngestao::construir(tipo.nome(), tarefas)?;
    let iniciada_em = Utc::now();
    let contexto = ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4().simple()
        ),
        iniciada_em,
        armazenamento: Arc::clone(silver),
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    };
    let dags = vec![dag];
    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(&contexto.id_execucao, Camada::Silver, iniciada_em, &dags)?,
        historico,
    ));
    registrador.publicar().await;
    tokio::spawn({
        let registrador = Arc::clone(&registrador);
        async move { registrador.manter_batimento().await }
    });
    let relatorio = ExecutorDeDag::novo(concorrencia()?)
        .executar(&dags[0], &contexto, registrador.as_ref())
        .await?;
    imprimir(&relatorio);
    registrador.concluir().await;
    println!("\nexecução Silver: {}", contexto.id_execucao);
    Ok(relatorio.sucesso())
}

/// Publica as séries BCB/SGS vigentes do bronze para a silver.
async fn refinar_bcb_silver(
    fonte: Option<String>,
    todas: bool,
    intervalo: Option<u64>,
) -> Result<bool> {
    if fonte.as_deref().is_some_and(|nome| nome != "bcb") {
        bail!("a camada silver aceita a fonte 'bcb'");
    }
    if todas && fonte.is_some() {
        bail!("informe uma fonte ou --todas, não os dois");
    }
    let raw_s3 = ArmazenamentoS3::novo(configuracao_s3("raw")?)?;
    let bronze: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("bronze")?)?);
    let silver: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("silver")?)?);
    bronze.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    silver.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    match intervalo.filter(|segundos| *segundos > 0) {
        None => passada_bcb_silver(&raw_s3, &bronze, &silver, false).await,
        Some(segundos) => {
            println!("Agendador Silver BCB: reconciliando a cada {segundos}s. Ctrl+C para sair.");
            loop {
                if let Err(erro) = passada_bcb_silver(&raw_s3, &bronze, &silver, true).await {
                    eprintln!("Atenção: falha na reconciliação Silver: {erro:#}");
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

/// Executa uma rodada BCB pela DAG e só publica no painel quando há trabalho.
async fn passada_bcb_silver(
    raw_s3: &ArmazenamentoS3,
    bronze: &Arc<dyn ArmazenamentoRaw>,
    silver: &Arc<dyn ArmazenamentoRaw>,
    somente_pendentes: bool,
) -> Result<bool> {
    let prefixo = "bcb/series/";
    let chaves = raw_s3
        .listar_chaves(prefixo)
        .await
        .context(ERRO_DE_ARMAZENAMENTO)?;
    let mut recursos: Vec<String> = chaves
        .iter()
        .filter_map(|chave| chave.strip_suffix("/_ultima_versao.json"))
        .filter_map(|recurso| recurso.strip_prefix("bcb/"))
        .map(str::to_owned)
        .collect();
    recursos.sort();
    recursos.dedup();
    let mut tarefas = Vec::new();
    for recurso in recursos {
        let Some(snapshot) = SnapshotBronze::vigente(bronze.as_ref(), "bcb", &recurso)
            .await
            .context("não foi possível ler o snapshot Bronze")?
        else {
            continue;
        };
        let tarefa = TarefaDeSilverBcb::nova(Arc::clone(bronze), Arc::new(snapshot), recurso);
        if somente_pendentes && !tarefa.pendente(silver.as_ref()).await? {
            continue;
        }
        tarefas.push(TarefaPlanejada::nova(tarefa));
    }
    if tarefas.is_empty() {
        println!("Nada a refinar na Silver (séries em dia ou não ingeridas).");
        return Ok(true);
    }
    let dag = DagDeIngestao::construir("bcb", tarefas)?;
    let iniciada_em = Utc::now();
    let contexto = ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4().simple()
        ),
        iniciada_em,
        armazenamento: Arc::clone(silver),
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    };
    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    let dags = vec![dag];
    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(&contexto.id_execucao, Camada::Silver, iniciada_em, &dags)?,
        historico,
    ));
    registrador.publicar().await;
    tokio::spawn({
        let registrador = Arc::clone(&registrador);
        async move { registrador.manter_batimento().await }
    });
    let relatorio = ExecutorDeDag::novo(concorrencia()?)
        .executar(&dags[0], &contexto, registrador.as_ref())
        .await?;
    imprimir(&relatorio);
    registrador.concluir().await;
    println!("\nexecução Silver: {}", contexto.id_execucao);
    Ok(relatorio.sucesso())
}

/// Publica agregados IBGE Silver por fonte ou percorre todas as fontes IBGE suportadas.
async fn refinar_ibge_silver(
    fonte: Option<String>,
    todas: bool,
    intervalo: Option<u64>,
) -> Result<bool> {
    if todas && fonte.is_some() {
        bail!("informe uma fonte IBGE ou --todas, não os dois");
    }
    let fontes = fontes_ibge_silver(fonte.as_deref(), todas);
    if fontes.is_empty() {
        bail!("fonte Silver IBGE desconhecida ou ausente; use --todas ou uma fonte ibge-*");
    }

    let raw_s3 = ArmazenamentoS3::novo(configuracao_s3("raw")?)?;
    let bronze: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("bronze")?)?);
    let silver: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("silver")?)?);
    bronze.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    silver.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    match intervalo.filter(|segundos| *segundos > 0) {
        None => passada_ibge_silver(&fontes, &raw_s3, &bronze, &silver, false).await,
        Some(segundos) => {
            println!(
                "Agendador Silver IBGE: reconciliando {} fonte(s) a cada {segundos}s. Ctrl+C para sair.",
                fontes.len()
            );
            loop {
                if let Err(erro) =
                    passada_ibge_silver(&fontes, &raw_s3, &bronze, &silver, true).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver IBGE: {erro:#}");
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

fn fontes_ibge_silver(fonte: Option<&str>, todas: bool) -> Vec<FonteDeRefino> {
    const FONTES_COM_CONTRATO_DE_AGREGADOS: &[&str] = &[
        "ibge-ipca",
        "ibge-inpc",
        "ibge-pib-municipios",
        "ibge-pib-trimestral",
        "ibge-pnad-desocupacao",
        "ibge-populacao-municipios",
    ];

    catalogo_refino()
        .into_iter()
        .filter(|item| {
            FONTES_COM_CONTRATO_DE_AGREGADOS.contains(&item.nome)
                && (todas || fonte == Some(item.nome))
        })
        .collect()
}

/// Executa uma rodada Silver IBGE e só registra execução se houver recurso pendente.
async fn passada_ibge_silver(
    fontes: &[FonteDeRefino],
    raw_s3: &ArmazenamentoS3,
    bronze: &Arc<dyn ArmazenamentoRaw>,
    silver: &Arc<dyn ArmazenamentoRaw>,
    somente_pendentes: bool,
) -> Result<bool> {
    let mut dags = Vec::new();
    for fonte in fontes {
        let mut tarefas = Vec::new();
        for recurso in recursos_no_raw(raw_s3, fonte).await? {
            let Some(snapshot) = SnapshotBronze::vigente(bronze.as_ref(), "ibge", &recurso)
                .await
                .context("não foi possível ler o snapshot Bronze IBGE")?
            else {
                continue;
            };
            let tarefa = TarefaDeSilverIbge::nova(Arc::clone(bronze), Arc::new(snapshot), &recurso);
            if somente_pendentes && !tarefa.pendente(silver.as_ref()).await? {
                continue;
            }
            tarefas.push(TarefaPlanejada::nova(tarefa));
        }
        if !tarefas.is_empty() {
            dags.push(DagDeIngestao::construir(fonte.nome, tarefas)?);
        }
    }
    if dags.is_empty() {
        println!("Nada a refinar na Silver IBGE (recursos em dia ou não ingeridos).");
        return Ok(true);
    }

    let iniciada_em = Utc::now();
    let contexto = ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4().simple()
        ),
        iniciada_em,
        armazenamento: Arc::clone(silver),
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    };
    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(&contexto.id_execucao, Camada::Silver, iniciada_em, &dags)?,
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
    println!("\nexecução Silver: {}", contexto.id_execucao);
    Ok(sucesso)
}

async fn refinar_ibge_localidade_silver(
    fonte: Option<String>,
    _todas: bool,
    intervalo: Option<u64>,
    tipo: TipoLocalidadeIbge,
) -> Result<bool> {
    let nome_fonte = fonte.as_deref().unwrap_or("ibge-localidade");
    let catalogo = catalogo_refino();
    let item = catalogo
        .iter()
        .find(|f| f.nome == nome_fonte)
        .context("fonte de refino IBGE localidade não encontrada no catálogo")?;
    let raw_s3 = ArmazenamentoS3::novo(configuracao_s3("raw")?)?;
    let bronze: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("bronze")?)?);
    let silver: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("silver")?)?);
    bronze.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    silver.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;

    match intervalo.filter(|s| *s > 0) {
        None => {
            passada_ibge_localidade_silver(nome_fonte, item, &raw_s3, &bronze, &silver, tipo, false)
                .await
        }
        Some(segundos) => {
            println!(
                "Agendador Silver {nome_fonte}: reconciliando a cada {segundos}s. Ctrl+C para sair."
            );
            loop {
                if let Err(erro) = passada_ibge_localidade_silver(
                    nome_fonte, item, &raw_s3, &bronze, &silver, tipo, true,
                )
                .await
                {
                    eprintln!("Atenção: falha na reconciliação Silver {nome_fonte}: {erro:#}");
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

async fn passada_ibge_localidade_silver(
    nome_fonte: &str,
    item: &FonteDeRefino,
    raw_s3: &ArmazenamentoS3,
    bronze: &Arc<dyn ArmazenamentoRaw>,
    silver: &Arc<dyn ArmazenamentoRaw>,
    tipo: TipoLocalidadeIbge,
    somente_pendentes: bool,
) -> Result<bool> {
    let mut tarefas = Vec::new();
    for recurso in recursos_no_raw(raw_s3, item).await? {
        let Some(snapshot) = SnapshotBronze::vigente(bronze.as_ref(), "ibge", &recurso)
            .await
            .context("não foi possível ler o snapshot Bronze IBGE localidade")?
        else {
            continue;
        };
        let tarefa = TarefaDeSilverIbgeLocalidades::nova(
            Arc::clone(bronze),
            Arc::new(snapshot),
            &recurso,
            tipo,
        );
        if somente_pendentes && !tarefa.pendente(silver.as_ref()).await? {
            continue;
        }
        tarefas.push(TarefaPlanejada::nova(tarefa));
    }
    if tarefas.is_empty() {
        println!("Nada a refinar na Silver {nome_fonte} (em dia ou não ingerido).");
        return Ok(true);
    }
    let iniciada_em = Utc::now();
    let contexto = ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4().simple()
        ),
        iniciada_em,
        armazenamento: Arc::clone(silver),
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    };
    let dag = DagDeIngestao::construir(nome_fonte, tarefas)?;
    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(
            &contexto.id_execucao,
            Camada::Silver,
            iniciada_em,
            std::slice::from_ref(&dag),
        )?,
        historico,
    ));
    registrador.publicar().await;
    tokio::spawn({
        let registrador = Arc::clone(&registrador);
        async move { registrador.manter_batimento().await }
    });
    let executor = ExecutorDeDag::novo(concorrencia()?);
    let relatorio = executor
        .executar(&dag, &contexto, registrador.as_ref())
        .await?;
    imprimir(&relatorio);
    registrador.concluir().await;
    println!("\nexecução Silver: {}", contexto.id_execucao);
    Ok(relatorio.sucesso())
}

async fn refinar_ipeadata_silver(
    _fonte: Option<String>,
    _todas: bool,
    intervalo: Option<u64>,
) -> Result<bool> {
    let catalogo = catalogo_refino();
    let item = catalogo
        .iter()
        .find(|f| f.nome == "ipeadata")
        .context("fonte ipeadata não encontrada no catálogo")?;
    let raw_s3 = ArmazenamentoS3::novo(configuracao_s3("raw")?)?;
    let bronze: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("bronze")?)?);
    let silver: Arc<dyn ArmazenamentoRaw> =
        Arc::new(ArmazenamentoS3::novo(configuracao_s3("silver")?)?);
    bronze.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    silver.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;

    match intervalo.filter(|s| *s > 0) {
        None => passada_ipeadata_silver(item, &raw_s3, &bronze, &silver, false).await,
        Some(segundos) => {
            println!(
                "Agendador Silver Ipeadata: reconciliando a cada {segundos}s. Ctrl+C para sair."
            );
            loop {
                if let Err(erro) =
                    passada_ipeadata_silver(item, &raw_s3, &bronze, &silver, true).await
                {
                    eprintln!("Atenção: falha na reconciliação Silver Ipeadata: {erro:#}");
                }
                tokio::time::sleep(Duration::from_secs(segundos)).await;
            }
        }
    }
}

async fn passada_ipeadata_silver(
    item: &FonteDeRefino,
    raw_s3: &ArmazenamentoS3,
    bronze: &Arc<dyn ArmazenamentoRaw>,
    silver: &Arc<dyn ArmazenamentoRaw>,
    somente_pendentes: bool,
) -> Result<bool> {
    let mut tarefas = Vec::new();
    for recurso in recursos_no_raw(raw_s3, item).await? {
        let Some(snapshot) = SnapshotBronze::vigente(bronze.as_ref(), "ipeadata", &recurso)
            .await
            .context("não foi possível ler o snapshot Bronze Ipeadata")?
        else {
            continue;
        };
        let tarefa = TarefaDeSilverIpeadata::nova(Arc::clone(bronze), Arc::new(snapshot), &recurso);
        if somente_pendentes && !tarefa.pendente(silver.as_ref()).await? {
            continue;
        }
        tarefas.push(TarefaPlanejada::nova(tarefa));
    }
    if tarefas.is_empty() {
        println!("Nada a refinar na Silver Ipeadata (em dia ou não ingerido).");
        return Ok(true);
    }
    let iniciada_em = Utc::now();
    let contexto = ContextoDeExecucao {
        id_execucao: format!(
            "{}-{}",
            iniciada_em.format("%Y%m%dT%H%M%SZ"),
            Uuid::new_v4().simple()
        ),
        iniciada_em,
        armazenamento: Arc::clone(silver),
        http: Arc::new(ClienteHttpComRetentativas::novo(
            PoliticaDeRetentativas::default(),
        )?),
        progresso: None,
    };
    let dag = DagDeIngestao::construir("ipeadata", tarefas)?;
    let historico = Arc::new(historico()?);
    historico.preparar().await.context(ERRO_DE_ARMAZENAMENTO)?;
    let registrador = Arc::new(RegistradorDeExecucao::novo(
        RegistroDeExecucao::novo(
            &contexto.id_execucao,
            Camada::Silver,
            iniciada_em,
            std::slice::from_ref(&dag),
        )?,
        historico,
    ));
    registrador.publicar().await;
    tokio::spawn({
        let registrador = Arc::clone(&registrador);
        async move { registrador.manter_batimento().await }
    });
    let executor = ExecutorDeDag::novo(concorrencia()?);
    let relatorio = executor
        .executar(&dag, &contexto, registrador.as_ref())
        .await?;
    imprimir(&relatorio);
    registrador.concluir().await;
    println!("\nexecução Silver: {}", contexto.id_execucao);
    Ok(relatorio.sucesso())
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
            Uuid::new_v4().simple()
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

    #[test]
    fn ibge_silver_seleciona_apenas_fontes_com_contrato_de_agregados() {
        let nomes: Vec<_> = fontes_ibge_silver(None, true)
            .iter()
            .map(|fonte| fonte.nome)
            .collect();

        assert_eq!(
            nomes,
            vec![
                "ibge-ipca",
                "ibge-inpc",
                "ibge-pib-municipios",
                "ibge-pib-trimestral",
                "ibge-pnad-desocupacao",
                "ibge-populacao-municipios",
            ]
        );
        assert!(fontes_ibge_silver(Some("ibge-estados"), false).is_empty());
    }

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
