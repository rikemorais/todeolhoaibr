//! PIB per capita dos municípios (IBGE): PIB dos Municípios e população residente
//! estimada, ambos anuais e por município, de onde o refino deriva o PIB per capita.
//!
//! A API de agregados (v3) do IBGE devolve erro 500 ao pedir todos os municípios
//! (`N6`) de vários períodos de uma vez, então cada **ano** é uma tarefa própria, que
//! baixa a série completa daquele ano (todos os municípios) em JSON, gravada opaca na
//! Raw. O PIB per capita (PIB ÷ população) é calculado no refino.

use std::time::Duration;

use todeolho_nucleo::{FonteDeDados, LimiteHttp, PerfilHttp, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

const NOME: &str = "ibge-pib-municipios";

/// A Raw guarda os dados sob a fonte `ibge` (`raw/ibge/...`), embora no catálogo
/// sejam uma fonte separada, para poderem ser `sob_demanda`.
const FONTE_RAW: &str = "ibge";
const BASE: &str = "https://servicodados.ibge.gov.br/api/v3/agregados";

/// PIB dos Municípios (agregado 5938), variável 37 = PIB a preços correntes (Mil Reais).
const AGREGADO_PIB: u32 = 5938;
const VARIAVEL_PIB: u32 = 37;
const PRIMEIRO_ANO_PIB: u16 = 2002;
/// O PIB municipal sai com ~2 anos de defasagem; atualizar quando o IBGE publicar novos anos.
const ULTIMO_ANO_PIB: u16 = 2023;

/// População residente estimada (agregado 6579), variável 9324.
const AGREGADO_POP: u32 = 6579;
const VARIAVEL_POP: u32 = 9324;
const PRIMEIRO_ANO_POP: u16 = 2001;
/// Atualizar conforme o IBGE publica as estimativas anuais de população.
const ULTIMO_ANO_POP: u16 = 2026;

/// Tamanho típico de um ano no nível municipal (`N6`, ~5.570 municípios), medido numa busca real.
const TAMANHO_TIPICO: u64 = 700_000;

/// URL da série de um ano, para todos os municípios (`N6[all]`). Os colchetes vão
/// percent-encoded para o cliente HTTP.
fn url(agregado: u32, variavel: u32, ano: u16) -> String {
    format!("{BASE}/{agregado}/periodos/{ano}/variaveis/{variavel}?localidades=N6%5Ball%5D")
}

fn tarefa(
    indicador: &str,
    recurso_base: &str,
    agregado: u32,
    variavel: u32,
    ano: u16,
) -> TarefaPlanejada {
    let recurso = format!("{recurso_base}/ano={ano}");
    TarefaPlanejada::nova(
        TarefaDeIngestaoHttp::sem_paginacao(FONTE_RAW, &recurso, url(agregado, variavel, ano))
            .com_tamanho_tipico(TAMANHO_TIPICO),
    )
    .com_etiquetas([("ano", ano.to_string().as_str()), ("indicador", indicador)])
}

/// PIB per capita dos municípios (IBGE).
///
/// Captura, por ano e por município (`N6`), o PIB dos Municípios (agregado 5938) e a
/// população residente estimada (agregado 6579). As respostas ficam opacas na Raw
/// ([ADR 0014]); o PIB per capita (PIB ÷ população) e o recorte por UF são do refino
/// ([ADR 0015]), que une tudo no `município + ano`. Fonte separada da `ibge` para ser
/// `sob_demanda` (são dezenas de anos no nível municipal) e não entrar em `--todas`.
/// Ver issue #30.
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteIbgePibMunicipios;

impl FonteDeDados for FonteIbgePibMunicipios {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "IBGE - PIB dos Municípios e população residente estimada, por ano e município (desde 2001)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        // Dezenas de consultas pesadas ao mesmo host; um ritmo modesto evita sufocá-lo.
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "servicodados.ibge.gov.br",
                requisicoes: 4,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let pib = (PRIMEIRO_ANO_PIB..=ULTIMO_ANO_PIB)
            .map(|ano| tarefa("pib", "pib-municipios", AGREGADO_PIB, VARIAVEL_PIB, ano));
        let populacao = (PRIMEIRO_ANO_POP..=ULTIMO_ANO_POP).map(|ano| {
            tarefa(
                "populacao",
                "populacao-municipios",
                AGREGADO_POP,
                VARIAVEL_POP,
                ano,
            )
        });
        pib.chain(populacao).collect()
    }
}

#[cfg(test)]
mod testes {
    use std::collections::BTreeSet;

    use super::*;

    fn anos_do_indicador(indicador: &str) -> BTreeSet<u16> {
        FonteIbgePibMunicipios
            .criar_tarefas()
            .iter()
            .filter(|t| t.etiquetas.get("indicador").map(String::as_str) == Some(indicador))
            .map(|t| t.etiquetas["ano"].parse().expect("ano numérico"))
            .collect()
    }

    #[test]
    fn cobre_os_anos_de_pib_e_de_populacao() {
        assert_eq!(
            anos_do_indicador("pib"),
            (PRIMEIRO_ANO_PIB..=ULTIMO_ANO_PIB).collect()
        );
        assert_eq!(
            anos_do_indicador("populacao"),
            (PRIMEIRO_ANO_POP..=ULTIMO_ANO_POP).collect()
        );
    }

    #[test]
    fn a_url_pede_um_ano_no_nivel_municipal() {
        assert_eq!(
            url(AGREGADO_PIB, VARIAVEL_PIB, 2020),
            "https://servicodados.ibge.gov.br/api/v3/agregados/5938/periodos/2020/variaveis/37?localidades=N6%5Ball%5D"
        );
    }

    #[test]
    fn cada_tarefa_tem_ano_e_indicador_coerentes_com_o_recurso() {
        for tarefa in FonteIbgePibMunicipios.criar_tarefas() {
            let nome = tarefa.tarefa.nome();
            let ano = tarefa.etiquetas.get("ano").expect("etiqueta ano");
            let indicador = tarefa
                .etiquetas
                .get("indicador")
                .expect("etiqueta indicador");
            let base = match indicador.as_str() {
                "pib" => "pib-municipios",
                "populacao" => "populacao-municipios",
                outro => panic!("indicador inesperado: {outro}"),
            };
            assert_eq!(nome, format!("{base}/ano={ano}"), "recurso incoerente");
        }
    }

    #[test]
    fn declara_o_limite_do_host_do_ibge() {
        let limite = FonteIbgePibMunicipios
            .perfil_http()
            .limite
            .expect("limite do IBGE");
        assert_eq!(limite.host, "servicodados.ibge.gov.br");
    }

    #[test]
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteIbgePibMunicipios.sob_demanda());
    }
}
