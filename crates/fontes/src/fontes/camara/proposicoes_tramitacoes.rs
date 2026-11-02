use chrono::{Datelike, Utc};
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

use super::api;

const NOME: &str = "camara-tramitacoes";

/// A Raw guarda as tramitações sob a mesma fonte `camara` (`raw/camara/tramitacoes/...`),
/// embora no catálogo sejam uma fonte separada, para poderem ser `sob_demanda`.
const FONTE_RAW: &str = "camara";
/// Arquivos anuais em lote dos Dados Abertos da Câmara.
const BASE_ARQUIVOS: &str = "https://dadosabertos.camara.leg.br/arquivos";
/// Primeiro ano com arquivo anual de tramitações publicado.
const PRIMEIRO_ANO: u16 = 2001;

fn ultimo_ano() -> u16 {
    Utc::now().year() as u16
}

fn tarefas_ate(ano_final: u16) -> Vec<TarefaPlanejada> {
    (PRIMEIRO_ANO..=ano_final)
        .map(|ano| {
            let mut tarefa = TarefaDeArquivo::nova(
                format!("tramitacoes_{ano}"),
                FONTE_RAW,
                format!("tramitacoes/ano={ano}"),
                format!(
                    "{BASE_ARQUIVOS}/proposicoesTramitacoes/json/proposicoesTramitacoes-{ano}.json"
                ),
            );
            if ano == ano_final {
                tarefa = tarefa.aceitar_ausencia();
            }
            TarefaPlanejada::nova(tarefa).com_etiquetas([("ano", ano.to_string().as_str())])
        })
        .collect()
}

/// Histórico oficial das tramitações das proposições da Câmara dos Deputados.
///
/// Em vez de percorrer a API REST proposição a proposição (`/proposicoes/{id}/tramitacoes`,
/// um fan-out de centenas de milhares de requisições), usa os **arquivos anuais em lote**
/// dos Dados Abertos (`/arquivos/proposicoesTramitacoes/json/proposicoesTramitacoes-{ano}.json`):
/// um download por ano, via [`TarefaDeArquivo`], guardado **opaco** na Raw ([ADR 0014]).
///
/// Os arquivos trazem `ETag`/`Last-Modified` e são regerados diariamente, então a rodada é
/// **incremental de graça**: a pré-checagem condicional `If-None-Match` ([ADR 0012]) faz o
/// ano sem mudança responder `304` e ser pulado. O desdobramento de cada passo de tramitação
/// (órgão, data, situação, despacho) e o cruzamento com a proposição são trabalho do refino
/// ([ADR 0015]).
///
/// Fonte separada da `camara` (API leve) para ser `sob_demanda` e não entrar em `--todas`.
/// O dado mora em `raw/camara/tramitacoes/`. Ver issues #113 e #127.
///
/// Documentação dos campos: <https://dadosabertos.camara.leg.br/howtouse/2023-12-26-dados-tramitacoes.html>
///
/// [ADR 0012]: ../../../docs/adr/0012-pre-checagem-idempotente.md
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteCamaraTramitacoes;

impl FonteDeDados for FonteCamaraTramitacoes {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Câmara dos Deputados - histórico de tramitações das proposições, um arquivo por ano desde 2001"
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

    /// Anos cobertos pelas tarefas, lidos do nome `tramitacoes_<ano>`.
    fn anos() -> BTreeSet<u16> {
        FonteCamaraTramitacoes
            .criar_tarefas()
            .iter()
            .map(|t| {
                let nome = t.tarefa.nome();
                nome.strip_prefix("tramitacoes_")
                    .expect("nome no formato tramitacoes_<ano>")
                    .parse()
                    .expect("ano numérico")
            })
            .collect()
    }

    #[test]
    fn cobre_cada_ano_de_2001_ate_o_atual() {
        let esperado: BTreeSet<u16> = (PRIMEIRO_ANO..=ultimo_ano()).collect();
        assert_eq!(anos(), esperado);
    }

    #[test]
    fn inclui_o_ano_novo_e_nao_passa_dele() {
        let tarefas = tarefas_ate(2027);
        assert!(
            tarefas
                .iter()
                .any(|tarefa| tarefa.etiquetas["ano"] == "2027")
        );
        assert!(
            !tarefas
                .iter()
                .any(|tarefa| tarefa.etiquetas["ano"] == "2028")
        );
    }

    #[test]
    fn a_etiqueta_ano_bate_com_o_nome() {
        for tarefa in FonteCamaraTramitacoes.criar_tarefas() {
            let nome = tarefa.tarefa.nome();
            let ano = nome
                .strip_prefix("tramitacoes_")
                .expect("nome no formato tramitacoes_<ano>");
            // A etiqueta guia `--filtro`: se divergir do nome, o filtro
            // selecionaria a tarefa errada.
            assert_eq!(tarefa.etiquetas.get("ano").map(String::as_str), Some(ano));
        }
    }

    #[test]
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteCamaraTramitacoes.sob_demanda());
    }
}
