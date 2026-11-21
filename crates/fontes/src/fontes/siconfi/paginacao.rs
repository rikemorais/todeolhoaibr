use std::collections::VecDeque;
use std::sync::Mutex;

use serde::Deserialize;

use crate::paginacao::EstrategiaDePaginacao;

use super::api::BASE;

#[derive(Deserialize)]
struct EnvelopeDeEntes {
    items: Vec<Ente>,
}

#[derive(Deserialize)]
struct Ente {
    cod_ibge: u64,
    uf: String,
}

fn parsear_entes(corpo: &[u8], uf: &str) -> Result<Vec<u64>, String> {
    let envelope: EnvelopeDeEntes = serde_json::from_slice(corpo)
        .map_err(|erro| format!("listagem de entes do SICONFI inválida: {erro}"))?;
    Ok(envelope
        .items
        .into_iter()
        .filter(|ente| ente.uf == uf)
        .map(|ente| ente.cod_ibge)
        .collect())
}

struct EstadoDaPaginacao {
    entes_processados: bool,
    urls_pendentes: VecDeque<String>,
}

// --- DCA ---

pub(super) struct PaginacaoDca {
    uf: String,
    ano: u16,
    estado: Mutex<EstadoDaPaginacao>,
}

impl PaginacaoDca {
    pub(super) fn new(uf: &str, ano: u16) -> Self {
        Self {
            uf: uf.to_owned(),
            ano,
            estado: Mutex::new(EstadoDaPaginacao {
                entes_processados: false,
                urls_pendentes: VecDeque::new(),
            }),
        }
    }
}

impl EstrategiaDePaginacao for PaginacaoDca {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        let mut estado = self
            .estado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if !estado.entes_processados {
            let codigos = parsear_entes(corpo, &self.uf)?;
            estado.urls_pendentes.extend(codigos.into_iter().map(|cod| {
                format!(
                    "{BASE}dca?an_exercicio={}&id_ente={cod}&limit=10000",
                    self.ano
                )
            }));
            estado.entes_processados = true;
        }

        Ok(estado.urls_pendentes.pop_front())
    }
}

// --- RREO ---

pub(super) struct PaginacaoRreo {
    uf: String,
    ano: u16,
    bimestre: u8,
    estado: Mutex<EstadoDaPaginacao>,
}

impl PaginacaoRreo {
    pub(super) fn new(uf: &str, ano: u16, bimestre: u8) -> Self {
        Self {
            uf: uf.to_owned(),
            ano,
            bimestre,
            estado: Mutex::new(EstadoDaPaginacao {
                entes_processados: false,
                urls_pendentes: VecDeque::new(),
            }),
        }
    }
}

impl EstrategiaDePaginacao for PaginacaoRreo {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        let mut estado = self
            .estado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if !estado.entes_processados {
            let codigos = parsear_entes(corpo, &self.uf)?;
            estado.urls_pendentes.extend(codigos.into_iter().map(|cod| {
                format!(
                    "{BASE}rreo?an_exercicio={}&nr_periodo={}&co_tipo_demonstrativo=RREO&id_ente={cod}&limit=10000",
                    self.ano, self.bimestre
                )
            }));
            estado.entes_processados = true;
        }

        Ok(estado.urls_pendentes.pop_front())
    }
}

// --- RGF ---

const PODERES: [&str; 5] = ["E", "L", "J", "M", "D"];

pub(super) struct PaginacaoRgf {
    uf: String,
    ano: u16,
    quadrimestre: u8,
    estado: Mutex<EstadoDaPaginacao>,
}

impl PaginacaoRgf {
    pub(super) fn new(uf: &str, ano: u16, quadrimestre: u8) -> Self {
        Self {
            uf: uf.to_owned(),
            ano,
            quadrimestre,
            estado: Mutex::new(EstadoDaPaginacao {
                entes_processados: false,
                urls_pendentes: VecDeque::new(),
            }),
        }
    }
}

impl EstrategiaDePaginacao for PaginacaoRgf {
    fn proxima_url(&self, corpo: &[u8]) -> Result<Option<String>, String> {
        let mut estado = self
            .estado
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if !estado.entes_processados {
            let codigos = parsear_entes(corpo, &self.uf)?;
            for cod in codigos {
                for poder in &PODERES {
                    estado.urls_pendentes.push_back(format!(
                        "{BASE}rgf?an_exercicio={}&in_periodicidade=Q&nr_periodo={}&co_tipo_demonstrativo=RGF&co_poder={poder}&id_ente={cod}&limit=10000",
                        self.ano, self.quadrimestre
                    ));
                }
            }
            estado.entes_processados = true;
        }

        Ok(estado.urls_pendentes.pop_front())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const CORPO_ENTES: &[u8] = br#"{"items":[
        {"cod_ibge":1700301,"uf":"TO","regiao":"Norte","cd_uf":17,"instituicao":"Prefeitura Municipal","descricao":"Almas","capital":0,"cod_ibge_6":170030},
        {"cod_ibge":3550308,"uf":"SP","regiao":"Sudeste","cd_uf":35,"instituicao":"Prefeitura Municipal","descricao":"Sao Paulo","capital":1,"cod_ibge_6":355030}
    ],"hasMore":false,"limit":100,"offset":0,"count":2}"#;

    #[test]
    fn dca_filtra_por_uf_e_gera_urls() {
        let paginacao = PaginacaoDca::new("TO", 2024);
        let url = paginacao.proxima_url(CORPO_ENTES).unwrap();
        assert!(url.unwrap().contains("id_ente=1700301"));

        let url = paginacao.proxima_url(b"").unwrap();
        assert!(url.is_none());
    }

    #[test]
    fn rreo_inclui_bimestre_na_url() {
        let paginacao = PaginacaoRreo::new("SP", 2024, 3);
        let url = paginacao.proxima_url(CORPO_ENTES).unwrap().unwrap();
        assert!(url.contains("nr_periodo=3"));
        assert!(url.contains("id_ente=3550308"));
    }

    #[test]
    fn rgf_gera_cinco_urls_por_entidade() {
        let paginacao = PaginacaoRgf::new("TO", 2024, 1);
        let _ = paginacao.proxima_url(CORPO_ENTES).unwrap();
        let estado = paginacao.estado.lock().unwrap();
        // 1 ente TO × 5 poderes − 1 já consumido
        assert_eq!(estado.urls_pendentes.len(), 4);
    }
}
