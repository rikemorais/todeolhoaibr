use chrono::{Datelike, Utc};
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

use super::api;

const NOME: &str = "camara-votacoes";

/// A Raw guarda as votações sob a mesma fonte `camara` (`raw/camara/votacoes/...`),
/// embora no catálogo sejam uma fonte separada, para poderem ser `sob_demanda`.
const FONTE_RAW: &str = "camara";
/// Arquivos anuais em lote dos Dados Abertos da Câmara.
const BASE_ARQUIVOS: &str = "https://dadosabertos.camara.leg.br/arquivos";

fn ultimo_ano() -> u16 {
    Utc::now().year() as u16
}

fn tarefas_ate(ano_final: u16) -> Vec<TarefaPlanejada> {
    CONJUNTOS
        .iter()
        .flat_map(|&(conjunto, primeiro_ano)| {
            (primeiro_ano..=ano_final).map(move |ano| {
                let mut tarefa = TarefaDeArquivo::nova(
                    format!("{conjunto}_{ano}"),
                    FONTE_RAW,
                    format!("votacoes/{conjunto}/ano={ano}"),
                    format!("{BASE_ARQUIVOS}/{conjunto}/csv/{conjunto}-{ano}.csv"),
                );
                if ano == ano_final {
                    tarefa = tarefa.aceitar_ausencia();
                }
                TarefaPlanejada::nova(tarefa)
                    .com_etiquetas([("ano", ano.to_string().as_str()), ("conjunto", conjunto)])
            })
        })
        .collect()
}

/// Conjuntos (datasets) anuais ingeridos e o primeiro ano publicado de cada um.
///
/// O escopo começa em **2001**, o primeiro ano com voto nominal (`votacoesVotos`):
/// é a janela onde "quem votou em quê" existe. `proposicoes`/`votacoes` têm arquivos
/// mais antigos (desde ~1990), mas sem os votos não servem ao objetivo desta fonte.
/// `votacoesOrientacoes` (orientação de bancada) só começa em 2003.
const CONJUNTOS: &[(&str, u16)] = &[
    ("proposicoes", 2001),
    ("proposicoesAutores", 2001),
    ("votacoes", 2001),
    ("votacoesVotos", 2001),
    ("votacoesObjetos", 2001),
    ("votacoesProposicoes", 2001),
    ("votacoesOrientacoes", 2003),
];

/// Proposições e votações nominais da Câmara dos Deputados — "quais projetos foram
/// votados e quem votou em quê".
///
/// Em vez da API REST (`/votacoes/{id}/votos`, que exigiria um fan-out por id de
/// votação), usa os **arquivos anuais em lote** dos Dados Abertos
/// (`/arquivos/{conjunto}/csv/{conjunto}-{ano}.csv`): um download por (conjunto, ano),
/// via [`TarefaDeArquivo`], guardado **opaco** na Raw ([ADR 0014]). O voto nominal de
/// cada deputado vem no conjunto `votacoesVotos`; o cruzamento entre proposição,
/// votação, voto e deputado é trabalho do refino ([ADR 0015]).
///
/// Fonte separada da `camara` (API leve) para ser `sob_demanda` e não entrar em
/// `--todas`. O dado mora em `raw/camara/votacoes/`. Ver issue #86.
///
/// Documentação dos conjuntos: <https://dadosabertos.camara.leg.br/howtouse/2020-02-28-dados-votacoes.html>
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteCamaraVotacoes;

impl FonteDeDados for FonteCamaraVotacoes {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Câmara dos Deputados - proposições e votações nominais, um arquivo por ano desde 2001"
    }

    /// São dezenas de arquivos anuais em lote: não devem entrar em `--todas` por acidente.
    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        tarefas_ate(ultimo_ano())
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    /// Pares (conjunto, ano) cobertos pelas tarefas, lidos do nome `<conjunto>_<ano>`.
    fn pares() -> BTreeSet<(String, u16)> {
        FonteCamaraVotacoes
            .criar_tarefas()
            .iter()
            .map(|t| {
                let nome = t.tarefa.nome();
                let (conjunto, ano) = nome
                    .rsplit_once('_')
                    .expect("nome no formato <conjunto>_<ano>");
                (conjunto.to_owned(), ano.parse().expect("ano numérico"))
            })
            .collect()
    }

    #[test]
    fn cobre_exatamente_cada_conjunto_do_seu_primeiro_ano_ate_o_ultimo() {
        let esperado: BTreeSet<(String, u16)> = CONJUNTOS
            .iter()
            .flat_map(|&(conjunto, primeiro)| {
                (primeiro..=ultimo_ano()).map(move |ano| (conjunto.to_owned(), ano))
            })
            .collect();
        // A contagem bruta (antes de virar `BTreeSet`) pega uma duplicata de
        // `(conjunto, ano)` que o conjunto esconderia.
        assert_eq!(
            FonteCamaraVotacoes.criar_tarefas().len(),
            esperado.len(),
            "número de tarefas diverge do esperado (duplicata ou lacuna)"
        );
        assert_eq!(pares(), esperado);
    }

    #[test]
    fn cobre_os_sete_conjuntos_esperados() {
        // Lista explícita e independente de `CONJUNTOS`: se um dataset exigido
        // for removido de `CONJUNTOS`, este teste falha (incluindo `votacoesVotos`,
        // o voto nominal, sem o qual a fonte perde o sentido).
        let esperados: BTreeSet<String> = [
            "proposicoes",
            "proposicoesAutores",
            "votacoes",
            "votacoesVotos",
            "votacoesObjetos",
            "votacoesProposicoes",
            "votacoesOrientacoes",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let gerados: BTreeSet<String> = pares().into_iter().map(|(conjunto, _)| conjunto).collect();
        assert_eq!(gerados, esperados);
    }

    #[test]
    fn inclui_o_ano_novo_em_todos_os_conjuntos() {
        let tarefas = tarefas_ate(2027);
        let atuais: BTreeSet<_> = tarefas
            .iter()
            .filter(|tarefa| tarefa.etiquetas["ano"] == "2027")
            .map(|tarefa| tarefa.etiquetas["conjunto"].as_str())
            .collect();
        assert_eq!(atuais.len(), CONJUNTOS.len());
        assert!(
            !tarefas
                .iter()
                .any(|tarefa| tarefa.etiquetas["ano"] == "2028")
        );
    }

    #[test]
    fn as_etiquetas_batem_com_o_nome() {
        for tarefa in FonteCamaraVotacoes.criar_tarefas() {
            let nome = tarefa.tarefa.nome();
            let (conjunto, ano) = nome
                .rsplit_once('_')
                .expect("nome no formato <conjunto>_<ano>");
            // As etiquetas guiam `--filtro`: se divergirem do nome, o filtro
            // selecionaria a tarefa errada.
            assert_eq!(tarefa.etiquetas.get("ano").map(String::as_str), Some(ano));
            assert_eq!(
                tarefa.etiquetas.get("conjunto").map(String::as_str),
                Some(conjunto)
            );
        }
    }

    #[test]
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteCamaraVotacoes.sob_demanda());
    }
}
