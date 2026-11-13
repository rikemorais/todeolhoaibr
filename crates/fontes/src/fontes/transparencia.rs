//! Benefícios sociais do Portal da Transparência, preservados sem transformação na Raw.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{Datelike, NaiveDate, Utc};
use serde::Deserialize;
use todeolho_nucleo::{
    CredencialHttp, ErroDeIngestao, FonteDeDados, LimiteHttp, PerfilHttp, PrecisaoDaEstimativa,
    ResumoDaTarefa, Tarefa, TarefaPlanejada, estado_na_raw, manifesto_vigente,
};

use crate::ingestao_http::TarefaDeIngestaoHttp;
use crate::paginacao::EstrategiaDePaginacao;

const NOME: &str = "transparencia-beneficios";
const FONTE_RAW: &str = "transparencia";
const HOST_API: &str = "api.portaldatransparencia.gov.br";
const BASE_API: &str = "https://api.portaldatransparencia.gov.br/api-de-dados";
const PRIMEIRA_COMPETENCIA_BOLSA: (i32, u32) = (2013, 1);
const ULTIMA_COMPETENCIA_BOLSA: (i32, u32) = (2021, 10);
const PRIMEIRA_COMPETENCIA_AUXILIO: (i32, u32) = (2021, 11);
const ULTIMA_COMPETENCIA_AUXILIO: (i32, u32) = (2023, 2);
const PRIMEIRA_COMPETENCIA_NOVO_BOLSA: (i32, u32) = (2023, 3);
const PRIMEIRA_COMPETENCIA_BPC: (i32, u32) = (2019, 1);
const TAMANHO_TIPICO: u64 = 64 * 1024;
const LIMITE_PAGINAS: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Programa {
    BolsaFamilia,
    AuxilioBrasil,
    NovoBolsaFamilia,
    Bpc,
}

impl Programa {
    fn slug(self) -> &'static str {
        match self {
            Self::BolsaFamilia => "bolsa-familia",
            Self::AuxilioBrasil => "auxilio-brasil",
            Self::NovoBolsaFamilia => "novo-bolsa-familia",
            Self::Bpc => "bpc",
        }
    }

    fn endpoint(self) -> &'static str {
        match self {
            Self::BolsaFamilia => "bolsa-familia-sacado-beneficiario-por-municipio",
            Self::AuxilioBrasil => "auxilio-brasil-sacado-beneficiario-por-municipio",
            Self::NovoBolsaFamilia => "novo-bolsa-familia-sacado-beneficiario-por-municipio",
            Self::Bpc => "bpc-beneficiario-por-municipio",
        }
    }

    fn competencia_inicial(self) -> (i32, u32) {
        match self {
            Self::BolsaFamilia => PRIMEIRA_COMPETENCIA_BOLSA,
            Self::AuxilioBrasil => PRIMEIRA_COMPETENCIA_AUXILIO,
            Self::NovoBolsaFamilia => PRIMEIRA_COMPETENCIA_NOVO_BOLSA,
            Self::Bpc => PRIMEIRA_COMPETENCIA_BPC,
        }
    }

    fn competencia_final(self) -> (i32, u32) {
        match self {
            Self::BolsaFamilia => ULTIMA_COMPETENCIA_BOLSA,
            Self::AuxilioBrasil => ULTIMA_COMPETENCIA_AUXILIO,
            Self::NovoBolsaFamilia | Self::Bpc => ultima_competencia_publicada(),
        }
    }

    fn url(self, competencia: &str, municipio: &str, pagina: usize) -> String {
        format!(
            "{BASE_API}/{}?mesAno={}&codigoIbge={municipio}&pagina={pagina}",
            self.endpoint(),
            competencia.replace('-', "")
        )
    }
}

/// Mês de referência conservador: evita solicitar o mês corrente e o imediatamente
/// anterior, que ainda pode estar em publicação no Portal.
fn ultima_competencia_publicada() -> (i32, u32) {
    let mes = Utc::now().date_naive().with_day(1).expect("dia válido");
    let mes = mes
        .checked_sub_months(chrono::Months::new(2))
        .unwrap_or(mes);
    (mes.year(), mes.month())
}

fn competencias(inicio: (i32, u32), fim: (i32, u32)) -> Vec<String> {
    let mut atual = NaiveDate::from_ymd_opt(inicio.0, inicio.1, 1).expect("competência válida");
    let limite = NaiveDate::from_ymd_opt(fim.0, fim.1, 1).expect("competência válida");
    let mut resultado = Vec::new();
    while atual <= limite {
        resultado.push(format!("{}-{:02}", atual.year(), atual.month()));
        let Some(proximo) = atual.checked_add_months(chrono::Months::new(1)) else {
            break;
        };
        atual = proximo;
    }
    resultado
}

fn recurso(programa: Programa, municipio: u32, competencia: &str) -> String {
    format!(
        "beneficios/programa={}/municipio={municipio}/competencia={competencia}",
        programa.slug()
    )
}

#[derive(Deserialize)]
struct MunicipioIbge {
    id: u32,
}

/// A API retorna um array por página, sem total ou link de próxima página. Uma página
/// vazia encerra a sequência; o limite superior evita laços causados por erro da origem.
struct PaginaPortal {
    url_base: String,
    proxima: AtomicUsize,
}

impl PaginaPortal {
    fn nova(url_base: String) -> Self {
        Self {
            url_base,
            proxima: AtomicUsize::new(2),
        }
    }
}

impl EstrategiaDePaginacao for PaginaPortal {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        if corpo.is_empty() {
            return Ok(None);
        }
        let itens: Vec<serde_json::Value> = serde_json::from_slice(corpo)
            .map_err(|erro| format!("resposta do Portal não é um array JSON: {erro}"))?;
        if itens.is_empty() {
            return Ok(None);
        }
        let pagina = self.proxima.fetch_add(1, Ordering::Relaxed);
        Ok(Some(
            self.url_base
                .replace("pagina=1", &format!("pagina={pagina}")),
        ))
    }
}

struct TarefaProgramaCompetencia {
    nome: String,
    programa: Programa,
    competencia: String,
}

impl TarefaProgramaCompetencia {
    fn nova(programa: Programa, competencia: &str) -> Self {
        Self {
            nome: format!("{}-{competencia}", programa.slug()),
            programa,
            competencia: competencia.to_owned(),
        }
    }

    async fn municipios(
        &self,
        contexto: &todeolho_nucleo::ContextoDeExecucao,
    ) -> Result<Vec<MunicipioIbge>, ErroDeIngestao> {
        let estado = estado_na_raw(contexto.armazenamento.as_ref(), "ibge", "municipios").await?;
        if !matches!(estado, Some(ref estado) if estado.completa && estado.integra) {
            return Err(ErroDeIngestao::Manifesto {
                recurso: "ibge/municipios".to_owned(),
                detalhe: "cadastro IBGE ausente, incompleto ou corrompido na Raw; execute a fonte `ibge` antes desta fonte".to_owned(),
            });
        }
        let manifesto = manifesto_vigente(contexto.armazenamento.as_ref(), "ibge", "municipios")
            .await?
            .ok_or_else(|| ErroDeIngestao::Manifesto {
                recurso: "ibge/municipios".to_owned(),
                detalhe: "cadastro IBGE ausente na Raw; execute a fonte `ibge` antes desta fonte"
                    .to_owned(),
            })?;
        let mut municipios = Vec::new();
        for parte in manifesto.partes {
            let bytes = contexto
                .armazenamento
                .ler(&parte.chave)
                .await?
                .ok_or_else(|| ErroDeIngestao::Manifesto {
                    recurso: "ibge/municipios".to_owned(),
                    detalhe: format!("parte do cadastro IBGE ausente: {}", parte.chave),
                })?;
            let mut pagina: Vec<MunicipioIbge> =
                serde_json::from_slice(&bytes).map_err(|erro| ErroDeIngestao::Manifesto {
                    recurso: "ibge/municipios".to_owned(),
                    detalhe: format!("cadastro IBGE inválido: {erro}"),
                })?;
            municipios.append(&mut pagina);
        }
        if municipios.is_empty() {
            return Err(ErroDeIngestao::Manifesto {
                recurso: "ibge/municipios".to_owned(),
                detalhe: "cadastro IBGE não contém municípios".to_owned(),
            });
        }
        Ok(municipios)
    }
}

#[async_trait]
impl Tarefa for TarefaProgramaCompetencia {
    fn nome(&self) -> &str {
        &self.nome
    }

    async fn executar(
        &self,
        contexto: &todeolho_nucleo::ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let municipios = self.municipios(contexto).await?;
        let mut resumo = ResumoDaTarefa {
            reaproveitada: true,
            ..ResumoDaTarefa::default()
        };

        for municipio in municipios {
            let codigo = municipio.id.to_string();
            let recurso = recurso(self.programa, municipio.id, &self.competencia);
            let url = self.programa.url(&self.competencia, &codigo, 1);
            let tarefa = TarefaDeIngestaoHttp::paginada(
                FONTE_RAW,
                &recurso,
                url.clone(),
                PaginaPortal::nova(url),
            )
            .com_tamanho_tipico(TAMANHO_TIPICO)
            .com_limite_de_paginas(LIMITE_PAGINAS);
            let parcial = tarefa.executar(contexto).await?;
            resumo.partes += parcial.partes;
            resumo.bytes += parcial.bytes;
            resumo.reaproveitada &= parcial.reaproveitada;
            if resumo.motivo.is_none() {
                resumo.motivo = parcial.motivo;
            }
        }
        Ok(resumo)
    }

    async fn bytes_a_baixar(
        &self,
        _contexto: &todeolho_nucleo::ContextoDeExecucao,
        _precisao: PrecisaoDaEstimativa,
    ) -> Result<Option<u64>, ErroDeIngestao> {
        Ok(None)
    }
}

/// Benefícios sociais em partições mensais e municipais, na Raw do Portal da Transparência.
pub struct FonteTransparenciaBeneficios;

impl FonteDeDados for FonteTransparenciaBeneficios {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Portal da Transparência - Bolsa Família, Auxílio Brasil e BPC por município e competência"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: Some(CredencialHttp {
                host: HOST_API,
                variavel: "PORTAL_TRANSPARENCIA_TOKEN",
                cabecalho: "chave-api-dados",
                prefixo: "",
            }),
            limite: Some(LimiteHttp {
                host: HOST_API,
                requisicoes: 90,
                intervalo: Duration::from_secs(60),
            }),
            agente: None,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let programas = [
            Programa::BolsaFamilia,
            Programa::AuxilioBrasil,
            Programa::NovoBolsaFamilia,
            Programa::Bpc,
        ];
        programas
            .into_iter()
            .flat_map(|programa| {
                competencias(programa.competencia_inicial(), programa.competencia_final())
                    .into_iter()
                    .map(move |competencia| {
                        TarefaPlanejada::nova(TarefaProgramaCompetencia::nova(
                            programa,
                            &competencia,
                        ))
                        .com_etiquetas([
                            ("programa", programa.slug()),
                            ("competencia", competencia.as_str()),
                        ])
                    })
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn lista_programas_e_competencias_historicas() {
        assert_eq!(
            Programa::BolsaFamilia.endpoint(),
            "bolsa-familia-sacado-beneficiario-por-municipio"
        );
        assert_eq!(
            competencias(PRIMEIRA_COMPETENCIA_AUXILIO, ULTIMA_COMPETENCIA_AUXILIO),
            [
                "2021-11", "2021-12", "2022-01", "2022-02", "2022-03", "2022-04", "2022-05",
                "2022-06", "2022-07", "2022-08", "2022-09", "2022-10", "2022-11", "2022-12",
                "2023-01", "2023-02"
            ]
        );
        assert_eq!(
            Programa::Bpc.url("2026-09", "3550308", 2),
            "https://api.portaldatransparencia.gov.br/api-de-dados/bpc-beneficiario-por-municipio?mesAno=202609&codigoIbge=3550308&pagina=2"
        );
    }

    #[test]
    fn pagina_vazia_finaliza_e_array_com_dados_avanca() {
        let url = Programa::Bpc.url("2026-09", "3550308", 1);
        let pagina = PaginaPortal::nova(url);
        assert_eq!(pagina.proxima_url(b"").unwrap(), None);
        assert_eq!(pagina.proxima_url(b"[]").unwrap(), None);
        assert_eq!(
            pagina.proxima_url(br#"[{"id":1}]"#).unwrap().as_deref(),
            Some(
                "https://api.portaldatransparencia.gov.br/api-de-dados/bpc-beneficiario-por-municipio?mesAno=202609&codigoIbge=3550308&pagina=2"
            )
        );
        assert_eq!(
            pagina.proxima_url(br#"[{"id":2}]"#).unwrap().as_deref(),
            Some(
                "https://api.portaldatransparencia.gov.br/api-de-dados/bpc-beneficiario-por-municipio?mesAno=202609&codigoIbge=3550308&pagina=3"
            )
        );
        assert!(pagina.proxima_url(b"<html>").is_err());
    }

    #[test]
    fn chave_raw_particiona_por_programa_municipio_e_competencia() {
        assert_eq!(
            recurso(Programa::NovoBolsaFamilia, 3550308, "2026-09"),
            "beneficios/programa=novo-bolsa-familia/municipio=3550308/competencia=2026-09"
        );
        assert_ne!(
            recurso(Programa::Bpc, 3550308, "2026-09"),
            recurso(Programa::NovoBolsaFamilia, 3550308, "2026-09")
        );
        assert_ne!(
            recurso(Programa::Bpc, 3550308, "2026-09"),
            recurso(Programa::Bpc, 3509502, "2026-09")
        );
        assert_ne!(
            recurso(Programa::Bpc, 3550308, "2026-09"),
            recurso(Programa::Bpc, 3550308, "2026-08")
        );
    }

    #[test]
    fn fonte_exige_credencial_e_fica_fora_de_todas() {
        let fonte = FonteTransparenciaBeneficios;
        assert!(fonte.sob_demanda());
        assert_eq!(fonte.nome(), NOME);
        let perfil = fonte.perfil_http();
        let credencial = perfil.credencial.unwrap();
        assert_eq!(credencial.variavel, "PORTAL_TRANSPARENCIA_TOKEN");
        assert_eq!(credencial.host, HOST_API);
        assert_eq!(perfil.limite.unwrap().requisicoes, 90);
    }

    #[test]
    fn tarefas_tem_etiquetas_para_filtros_e_recursos_por_programa() {
        let tarefas = FonteTransparenciaBeneficios.criar_tarefas();
        assert!(tarefas.len() >= 250);
        assert!(tarefas.iter().all(|t| {
            t.etiquetas.contains_key("programa") && t.etiquetas.contains_key("competencia")
        }));
        assert!(tarefas.iter().any(|t| {
            t.etiquetas["programa"] == "bpc" && t.etiquetas["competencia"] == "2019-01"
        }));
    }
}
