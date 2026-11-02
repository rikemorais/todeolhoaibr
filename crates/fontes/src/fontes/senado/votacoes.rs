use chrono::{Datelike, Utc};
use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;

const NOME: &str = "senado-votacoes";
const FONTE_RAW: &str = "senado";
const BASE: &str = "https://legis.senado.leg.br/dadosabertos";
/// Primeiro ano com votação registrada na API (1990 e antes retornam vazio).
const PRIMEIRO_ANO: u16 = 1991;

/// Último ano coberto: o **ano corrente**, derivado do relógio, para a fonte não
/// deixar de cobrir o ano novo sem edição manual. Um ano ainda sem votações volta
/// vazio da API, o que é inofensivo.
fn ultimo_ano() -> u16 {
    Utc::now().year() as u16
}

/// Matérias e votações nominais do Senado Federal — "quais projetos foram votados
/// e quem votou em quê".
///
/// Usa o serviço `GET /dadosabertos/votacao?dataInicio=&dataFim=`, que devolve um
/// array de votações **com `votos[]` embutido** (o voto nominal de cada senador) e a
/// matéria votada (`identificacao`, `ementa`, `codigoMateria`). Como o voto já vem
/// embutido, basta **uma requisição por ano** ([`TarefaDeIngestaoHttp::sem_paginacao`]) —
/// sem o fan-out por id que a antiga API por votação exigiria. É o par, no Senado, da
/// fonte `camara-votacoes`. Ver issue #87.
///
/// A resposta é guardada **opaca** na Raw ([ADR 0014]); cruzar matéria ↔ votação ↔ voto
/// ↔ senador e análises ficam para o refino ([ADR 0015]). Fonte separada e `sob_demanda`
/// para não entrar em `--todas` (é um backfill anual). O dado mora em `raw/senado/votacoes/`.
///
/// Documentação: <https://www12.senado.leg.br/dados-abertos/legislativo/plenario/votacoes-nominais>
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteSenadoVotacoes;

impl FonteDeDados for FonteSenadoVotacoes {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Senado Federal - matérias e votações nominais, uma consulta por ano desde 1991"
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    /// São dezenas de anos em lote: não devem entrar em `--todas` por acidente.
    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        (PRIMEIRO_ANO..=ultimo_ano())
            .map(|ano| {
                let url = format!("{BASE}/votacao?dataInicio={ano}-01-01&dataFim={ano}-12-31");
                TarefaPlanejada::nova(
                    TarefaDeIngestaoHttp::sem_paginacao(
                        FONTE_RAW,
                        &format!("votacoes/ano={ano}"),
                        url,
                    )
                    // A API não informa o tamanho de antemão; valor típico de um ano recente.
                    .com_tamanho_tipico(1_500_000),
                )
                .com_etiquetas([("ano", ano.to_string().as_str())])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    /// Anos cobertos pelas tarefas, lidos da etiqueta `ano`.
    fn anos() -> BTreeSet<u16> {
        FonteSenadoVotacoes
            .criar_tarefas()
            .iter()
            .map(|t| {
                t.etiquetas
                    .get("ano")
                    .and_then(|ano| ano.parse().ok())
                    .expect("etiqueta `ano` numérica")
            })
            .collect()
    }

    #[test]
    fn cobre_exatamente_um_ano_por_tarefa_de_1991_ate_o_ultimo() {
        let esperado: BTreeSet<u16> = (PRIMEIRO_ANO..=ultimo_ano()).collect();
        // A contagem bruta pega um ano duplicado que o conjunto esconderia.
        assert_eq!(
            FonteSenadoVotacoes.criar_tarefas().len(),
            esperado.len(),
            "número de tarefas diverge do esperado (duplicata ou lacuna)"
        );
        assert_eq!(anos(), esperado);
    }

    #[test]
    fn comeca_em_1991_quando_a_serie_passa_a_ter_dados() {
        // 1990 e antes retornam vazio na API: o floor não é arbitrário.
        assert_eq!(anos().iter().min().copied(), Some(1991));
    }

    #[test]
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteSenadoVotacoes.sob_demanda());
    }
}
