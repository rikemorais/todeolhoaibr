//! Catálogo de refino: liga cada fonte do bronze ao seu decodificador e ao recurso no raw.
//!
//! Espelha o papel do catálogo da ingestão (`fontes::catalogo`), mas para a decodificação:
//! diz de qual fonte/prefixo do raw cada refino lê e qual [`Decodificador`] usa.

use std::sync::Arc;

use crate::decodificador::Decodificador;
use crate::fontes::{
    DecodificadorCsv, DecodificadorIbgeAgregados, DecodificadorJson, DecodificadorZipCsv,
};

/// Uma fonte do bronze: o nome no catálogo, de onde lê no raw e como decodifica.
pub struct FonteDeRefino {
    /// Nome usado na CLI e no painel (ex.: `camara-despesas`).
    pub nome: &'static str,
    /// Fonte no raw de onde os recursos são lidos (ex.: `camara`).
    pub fonte_raw: &'static str,
    /// Prefixo dos recursos desta fonte dentro do raw (ex.: `despesas/`).
    pub prefixo_recurso: &'static str,
    cria_decodificador: fn() -> Arc<dyn Decodificador>,
}

impl FonteDeRefino {
    /// Cria o decodificador desta fonte.
    pub fn decodificador(&self) -> Arc<dyn Decodificador> {
        (self.cria_decodificador)()
    }
}

/// Fontes do bronze disponíveis. Cresce conforme novos decodificadores entram.
pub fn catalogo_refino() -> Vec<FonteDeRefino> {
    vec![
        // CEAP da Câmara: ZIP com um CSV anual (`;`).
        FonteDeRefino {
            nome: "camara-despesas",
            fonte_raw: "camara",
            prefixo_recurso: "despesas/",
            cria_decodificador: || Arc::new(DecodificadorZipCsv::novo(b';')),
        },
        // Pacotes do TSE: ZIP com vários CSVs por UF (`;`, Latin-1).
        // `tse-prestacao-contas` fica de fora por ora: tem CSVs consolidados de vários GB
        // (precisam de decodificação em fluxo) e anos antigos em `.txt` aninhado (ver #171).
        FonteDeRefino {
            nome: "tse-candidaturas",
            fonte_raw: "tse",
            prefixo_recurso: "candidaturas/",
            cria_decodificador: || Arc::new(DecodificadorZipCsv::novo(b';')),
        },
        FonteDeRefino {
            nome: "tse-bens",
            fonte_raw: "tse",
            prefixo_recurso: "bens/",
            cria_decodificador: || Arc::new(DecodificadorZipCsv::novo(b';')),
        },
        // CEAPS do Senado: CSV solto em Latin-1 com uma linha de preâmbulo antes do cabeçalho.
        FonteDeRefino {
            nome: "senado-despesas",
            fonte_raw: "senado",
            prefixo_recurso: "despesas/",
            cria_decodificador: || Arc::new(DecodificadorCsv::novo("despesas", b';', 1)),
        },
        // Orientações de votação da Câmara: CSV solto em UTF-8 com BOM.
        FonteDeRefino {
            nome: "camara-votacoes",
            fonte_raw: "camara",
            prefixo_recurso: "votacoes/",
            cria_decodificador: || Arc::new(DecodificadorCsv::novo("votacoes", b';', 0)),
        },
        // --- JSON de API: array de registros (no topo, ou sob uma chave) ---
        // BCB (SGS): array no topo [{data, valor}].
        FonteDeRefino {
            nome: "bcb",
            fonte_raw: "bcb",
            prefixo_recurso: "series/",
            cria_decodificador: || Arc::new(DecodificadorJson::novo("serie")),
        },
        // Ipeadata (OData): registros sob a chave `value`.
        FonteDeRefino {
            nome: "ipeadata",
            fonte_raw: "ipeadata",
            prefixo_recurso: "series/",
            cria_decodificador: || Arc::new(DecodificadorJson::com_raiz("serie", "value")),
        },
        // APIs de dados abertos da Câmara: registros sob a chave `dados`.
        FonteDeRefino {
            nome: "camara-tramitacoes",
            fonte_raw: "camara",
            prefixo_recurso: "tramitacoes/",
            cria_decodificador: || Arc::new(DecodificadorJson::com_raiz("tramitacoes", "dados")),
        },
        FonteDeRefino {
            nome: "camara-deputados",
            fonte_raw: "camara",
            prefixo_recurso: "deputados/",
            cria_decodificador: || Arc::new(DecodificadorJson::com_raiz("deputados", "dados")),
        },
        FonteDeRefino {
            nome: "camara-partidos",
            fonte_raw: "camara",
            prefixo_recurso: "partidos/",
            cria_decodificador: || Arc::new(DecodificadorJson::com_raiz("partidos", "dados")),
        },
        FonteDeRefino {
            nome: "camara-legislaturas",
            fonte_raw: "camara",
            prefixo_recurso: "legislaturas/",
            cria_decodificador: || Arc::new(DecodificadorJson::com_raiz("legislaturas", "dados")),
        },
        // Listas do IBGE (localidades): array de objetos no topo.
        FonteDeRefino {
            nome: "ibge-estados",
            fonte_raw: "ibge",
            prefixo_recurso: "estados/",
            cria_decodificador: || Arc::new(DecodificadorJson::novo("estados")),
        },
        FonteDeRefino {
            nome: "ibge-regioes",
            fonte_raw: "ibge",
            prefixo_recurso: "regioes/",
            cria_decodificador: || Arc::new(DecodificadorJson::novo("regioes")),
        },
        FonteDeRefino {
            nome: "ibge-municipios",
            fonte_raw: "ibge",
            prefixo_recurso: "municipios/",
            cria_decodificador: || Arc::new(DecodificadorJson::novo("municipios")),
        },
        // Senado (dados abertos): array de objetos no topo.
        FonteDeRefino {
            nome: "senado-processos",
            fonte_raw: "senado",
            prefixo_recurso: "processos/",
            cria_decodificador: || Arc::new(DecodificadorJson::novo("processos")),
        },
        FonteDeRefino {
            nome: "senado-votacoes",
            fonte_raw: "senado",
            prefixo_recurso: "votacoes/",
            cria_decodificador: || Arc::new(DecodificadorJson::novo("votacoes")),
        },
        // --- IBGE agregados (API v3): série aninhada explodida em linhas ---
        FonteDeRefino {
            nome: "ibge-ipca",
            fonte_raw: "ibge",
            prefixo_recurso: "ipca/",
            cria_decodificador: || Arc::new(DecodificadorIbgeAgregados::novo("ipca")),
        },
        FonteDeRefino {
            nome: "ibge-inpc",
            fonte_raw: "ibge",
            prefixo_recurso: "inpc/",
            cria_decodificador: || Arc::new(DecodificadorIbgeAgregados::novo("inpc")),
        },
        FonteDeRefino {
            nome: "ibge-pib-municipios",
            fonte_raw: "ibge",
            prefixo_recurso: "pib-municipios/",
            cria_decodificador: || Arc::new(DecodificadorIbgeAgregados::novo("pib-municipios")),
        },
        FonteDeRefino {
            nome: "ibge-pib-trimestral",
            fonte_raw: "ibge",
            prefixo_recurso: "pib-trimestral/",
            cria_decodificador: || Arc::new(DecodificadorIbgeAgregados::novo("pib-trimestral")),
        },
        FonteDeRefino {
            nome: "ibge-pnad-desocupacao",
            fonte_raw: "ibge",
            prefixo_recurso: "pnad-desocupacao/",
            cria_decodificador: || Arc::new(DecodificadorIbgeAgregados::novo("pnad-desocupacao")),
        },
        FonteDeRefino {
            nome: "ibge-populacao-municipios",
            fonte_raw: "ibge",
            prefixo_recurso: "populacao-municipios/",
            cria_decodificador: || {
                Arc::new(DecodificadorIbgeAgregados::novo("populacao-municipios"))
            },
        },
    ]
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_catalogo_tem_a_camara_despesas_apontando_para_o_raw_camara() {
        let fonte = catalogo_refino()
            .into_iter()
            .find(|f| f.nome == "camara-despesas")
            .expect("camara-despesas no catálogo");
        assert_eq!(fonte.fonte_raw, "camara");
        assert_eq!(fonte.prefixo_recurso, "despesas/");
        // O decodificador é construível.
        let _ = fonte.decodificador();
    }
}
