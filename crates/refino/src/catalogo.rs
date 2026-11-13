//! Catálogo de refino: liga cada fonte do bronze ao seu decodificador e ao recurso no raw.
//!
//! Espelha o papel do catálogo da ingestão (`fontes::catalogo`), mas para a decodificação:
//! diz de qual fonte/prefixo do raw cada refino lê e qual [`Decodificador`] usa.

use std::sync::Arc;

use crate::decodificador::Decodificador;
use crate::fontes::{
    DecodificadorCotahist, DecodificadorCsv, DecodificadorIbgeAgregados, DecodificadorJson,
    DecodificadorSenadoProcessosDetalhes, DecodificadorXmlDataset, DecodificadorZipCsv,
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
        // Pacotes do TSE: ZIP com vários CSVs/TXT por UF (`;`, Latin-1), decodificados em
        // fluxo (o part é baixado para disco e o ZIP é lido de lá — ZIPs de >1 GB não cabem
        // na memória; ver #171).
        FonteDeRefino {
            nome: "tse-prestacao-contas",
            fonte_raw: "tse",
            prefixo_recurso: "prestacao-contas/",
            cria_decodificador: || Arc::new(DecodificadorZipCsv::em_fluxo(b';')),
        },
        FonteDeRefino {
            nome: "tse-votacao-secao",
            fonte_raw: "tse",
            prefixo_recurso: "votacao_secao/",
            cria_decodificador: || Arc::new(DecodificadorZipCsv::em_fluxo(b';')),
        },
        FonteDeRefino {
            nome: "tse-candidaturas",
            fonte_raw: "tse",
            prefixo_recurso: "candidaturas/",
            cria_decodificador: || Arc::new(DecodificadorZipCsv::em_fluxo(b';')),
        },
        FonteDeRefino {
            nome: "tse-bens",
            fonte_raw: "tse",
            prefixo_recurso: "bens/",
            cria_decodificador: || Arc::new(DecodificadorZipCsv::em_fluxo(b';')),
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
            cria_decodificador: || Arc::new(DecodificadorJson::com_caminho("serie", &["value"])),
        },
        // APIs de dados abertos da Câmara: registros sob a chave `dados`.
        FonteDeRefino {
            nome: "camara-tramitacoes",
            fonte_raw: "camara",
            prefixo_recurso: "tramitacoes/",
            cria_decodificador: || {
                Arc::new(DecodificadorJson::com_caminho("tramitacoes", &["dados"]))
            },
        },
        FonteDeRefino {
            nome: "camara-deputados",
            fonte_raw: "camara",
            prefixo_recurso: "deputados/",
            cria_decodificador: || {
                Arc::new(DecodificadorJson::com_caminho("deputados", &["dados"]))
            },
        },
        FonteDeRefino {
            nome: "camara-partidos",
            fonte_raw: "camara",
            prefixo_recurso: "partidos/",
            cria_decodificador: || Arc::new(DecodificadorJson::com_caminho("partidos", &["dados"])),
        },
        FonteDeRefino {
            nome: "camara-legislaturas",
            fonte_raw: "camara",
            prefixo_recurso: "legislaturas/",
            cria_decodificador: || {
                Arc::new(DecodificadorJson::com_caminho("legislaturas", &["dados"]))
            },
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
        // IBGE SIDRA (Censos): array flat de objetos com colunas codificadas.
        FonteDeRefino {
            nome: "ibge-renda-per-capita",
            fonte_raw: "ibge",
            prefixo_recurso: "renda-per-capita/",
            cria_decodificador: || Arc::new(DecodificadorJson::novo("renda-per-capita")),
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
        // Detalhes de processos do Senado: um JSON por processo, agregados por ano.
        FonteDeRefino {
            nome: "senado-processos-detalhes",
            fonte_raw: "senado",
            prefixo_recurso: "processos-detalhes/",
            cria_decodificador: || Arc::new(DecodificadorSenadoProcessosDetalhes),
        },
        // Senadores por legislatura: array aninhado fundo no JSON do Senado.
        FonteDeRefino {
            nome: "senado-parlamentares",
            fonte_raw: "senado",
            prefixo_recurso: "parlamentares/",
            cria_decodificador: || {
                Arc::new(DecodificadorJson::com_caminho(
                    "parlamentares",
                    &[
                        "ListaParlamentarLegislatura",
                        "Parlamentares",
                        "Parlamentar",
                    ],
                ))
            },
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
        // --- Fontes que gravam CSV solto no raw ---
        // UNDP Human Development Report: CSV com vírgula, sem preâmbulo.
        FonteDeRefino {
            nome: "undp",
            fonte_raw: "undp",
            prefixo_recurso: "indices-compostos",
            cria_decodificador: || Arc::new(DecodificadorCsv::novo("indices-compostos", b',', 0)),
        },
        // Remuneração estadual de MG: CSV com ponto-e-vírgula do CKAN.
        FonteDeRefino {
            nome: "remuneracao-estadual-mg",
            fonte_raw: "remuneracao-estadual",
            prefixo_recurso: "uf=MG/",
            cria_decodificador: || Arc::new(DecodificadorCsv::novo("remuneracao-mg", b';', 0)),
        },
        // Remuneração estadual de GO: CSV com ponto-e-vírgula do CKAN.
        FonteDeRefino {
            nome: "remuneracao-estadual-go",
            fonte_raw: "remuneracao-estadual",
            prefixo_recurso: "uf=GO/",
            cria_decodificador: || Arc::new(DecodificadorCsv::novo("remuneracao-go", b';', 0)),
        },
        // --- Fontes JSON ---
        // World Bank Open Data: resposta `[meta, [registros]]`, array sob índice 1.
        FonteDeRefino {
            nome: "world-bank",
            fonte_raw: "world-bank",
            prefixo_recurso: "indicadores/",
            cria_decodificador: || Arc::new(DecodificadorJson::com_caminho("indicador", &["1"])),
        },
        // TCE-CE (SIM): registros sob a chave `elements`.
        FonteDeRefino {
            nome: "remuneracao-municipal-tce-ce",
            fonte_raw: "remuneracao-municipal-tce-ce",
            prefixo_recurso: "folhas-pagamentos/",
            cria_decodificador: || {
                Arc::new(DecodificadorJson::com_caminho(
                    "folhas-pagamentos",
                    &["elements"],
                ))
            },
        },
        // Portal da Transparência - benefícios: array de registros no topo por página.
        FonteDeRefino {
            nome: "transparencia-beneficios",
            fonte_raw: "transparencia",
            prefixo_recurso: "beneficios/",
            cria_decodificador: || Arc::new(DecodificadorJson::novo("beneficios")),
        },
        // --- ZIP com CSV ---
        // Portal da Transparência - servidores ativos: ZIP com CSV (`;`).
        FonteDeRefino {
            nome: "transparencia-servidores-ativos",
            fonte_raw: "transparencia",
            prefixo_recurso: "servidores-ativos/",
            cria_decodificador: || Arc::new(DecodificadorZipCsv::novo(b';')),
        },
        // --- XML DataSet ---
        // ANA/SNIRH: estações hidrológicas em XML DataSet (dois recursos: fluvio e pluvio).
        FonteDeRefino {
            nome: "ana",
            fonte_raw: "ana",
            prefixo_recurso: "estacoes-",
            cria_decodificador: || Arc::new(DecodificadorXmlDataset::novo("estacoes")),
        },
        // --- Fixed-width (ZIP) ---
        // B3 COTAHIST: cotações históricas em ZIP com TXT de 245 bytes por registro.
        FonteDeRefino {
            nome: "b3",
            fonte_raw: "b3",
            prefixo_recurso: "cotahist/",
            cria_decodificador: || Arc::new(DecodificadorCotahist),
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
        let _ = fonte.decodificador();
    }

    #[test]
    fn nomes_do_catalogo_sao_unicos() {
        let catalogo = catalogo_refino();
        let mut nomes: Vec<&str> = catalogo.iter().map(|f| f.nome).collect();
        nomes.sort();
        let antes = nomes.len();
        nomes.dedup();
        assert_eq!(antes, nomes.len(), "nomes duplicados no catálogo de refino");
    }

    #[test]
    fn novas_fontes_apontam_para_o_raw_correto() {
        let catalogo = catalogo_refino();
        let casos = [
            ("undp", "undp", "indices-compostos"),
            ("remuneracao-estadual-mg", "remuneracao-estadual", "uf=MG/"),
            ("remuneracao-estadual-go", "remuneracao-estadual", "uf=GO/"),
            ("world-bank", "world-bank", "indicadores/"),
            (
                "remuneracao-municipal-tce-ce",
                "remuneracao-municipal-tce-ce",
                "folhas-pagamentos/",
            ),
            ("transparencia-beneficios", "transparencia", "beneficios/"),
            (
                "transparencia-servidores-ativos",
                "transparencia",
                "servidores-ativos/",
            ),
            ("ana", "ana", "estacoes-"),
            ("b3", "b3", "cotahist/"),
        ];
        for (nome, raw, prefixo) in casos {
            let fonte = catalogo
                .iter()
                .find(|f| f.nome == nome)
                .unwrap_or_else(|| panic!("{nome} ausente no catálogo"));
            assert_eq!(fonte.fonte_raw, raw, "raw errado para {nome}");
            assert_eq!(fonte.prefixo_recurso, prefixo, "prefixo errado para {nome}");
            let _ = fonte.decodificador();
        }
    }
}
