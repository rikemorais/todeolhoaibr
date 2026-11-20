use std::time::Duration;

use async_trait::async_trait;
use todeolho_nucleo::{
    ContextoDeExecucao, ErroDeIngestao, FonteDeDados, LimiteHttp, PerfilHttp, RedirecionamentoHttp,
    ResumoDaTarefa, Tarefa, TarefaPlanejada,
};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "car";
const BASE_WFS: &str = "https://geoserver.car.gov.br/geoserver/sicar/wfs";
const REGISTROS_POR_PAGINA: usize = 10_000;
const LIMITE_DE_PAGINAS: usize = 1_000;

const UFS: [&str; 27] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "DF", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE",
    "PI", "PR", "RJ", "RN", "RO", "RR", "RS", "SC", "SE", "SP", "TO",
];

fn url_wfs(uf: &str, inicio: usize) -> String {
    format!(
        "{BASE_WFS}?service=WFS&version=2.0.0&request=GetFeature\
         &typeNames=sicar:sicar_imoveis_{uf}&outputFormat=SHAPE-ZIP\
         &count={REGISTROS_POR_PAGINA}&startIndex={inicio}",
        uf = uf.to_lowercase()
    )
}

fn url_contagem(uf: &str) -> String {
    format!(
        "{BASE_WFS}?service=WFS&version=2.0.0&request=GetFeature\
         &typeNames=sicar:sicar_imoveis_{uf}&resultType=hits",
        uf = uf.to_lowercase()
    )
}

fn ler_total(corpo: &[u8], url: &str) -> Result<usize, ErroDeIngestao> {
    let texto = std::str::from_utf8(corpo).map_err(|erro| ErroDeIngestao::Paginacao {
        recurso: url.to_owned(),
        detalhe: format!("resposta de contagem não é UTF-8: {erro}"),
    })?;
    let valor = texto
        .split_once("numberMatched=\"")
        .and_then(|(_, resto)| resto.split_once('"'))
        .map(|(valor, _)| valor)
        .ok_or_else(|| ErroDeIngestao::Paginacao {
            recurso: url.to_owned(),
            detalhe: "resposta WFS sem numberMatched".into(),
        })?;
    valor.parse().map_err(|erro| ErroDeIngestao::Paginacao {
        recurso: url.to_owned(),
        detalhe: format!("numberMatched inválido ({valor}): {erro}"),
    })
}

struct TarefaCar {
    nome: String,
    uf: &'static str,
}

#[async_trait]
impl Tarefa for TarefaCar {
    fn nome(&self) -> &str {
        &self.nome
    }

    async fn executar(
        &self,
        contexto: &ContextoDeExecucao,
    ) -> Result<ResumoDaTarefa, ErroDeIngestao> {
        let url = url_contagem(self.uf);
        let resposta = contexto.http.obter(&url, "application/xml").await?;
        let total = ler_total(&resposta.corpo, &url)?;
        if total == 0 {
            return Err(ErroDeIngestao::Paginacao {
                recurso: self.nome.clone(),
                detalhe: "o WFS informou zero imóveis para a UF".into(),
            });
        }
        let paginas = total.div_ceil(REGISTROS_POR_PAGINA);
        if paginas > LIMITE_DE_PAGINAS {
            return Err(ErroDeIngestao::Paginacao {
                recurso: self.nome.clone(),
                detalhe: format!("{paginas} páginas excedem o limite de {LIMITE_DE_PAGINAS}"),
            });
        }

        let mut resumo = ResumoDaTarefa {
            reaproveitada: true,
            ..ResumoDaTarefa::default()
        };
        for pagina in 0..paginas {
            let tarefa = TarefaDeArquivo::nova(
                format!("{}/pagina={pagina:04}", self.nome),
                NOME,
                format!("imoveis/{}/pagina={pagina:04}", self.uf),
                url_wfs(self.uf, pagina * REGISTROS_POR_PAGINA),
            );
            let parcial = tarefa.executar(contexto).await?;
            resumo.partes += parcial.partes;
            resumo.bytes += parcial.bytes;
            resumo.reaproveitada &= parcial.reaproveitada;
            resumo.motivo = resumo.motivo.or(parcial.motivo);
        }
        Ok(resumo)
    }
}

/// Cadastro Ambiental Rural (CAR) — perímetros de imóveis rurais por UF.
///
/// O GeoServer limita a resposta WFS a 10 mil feições. Cada tarefa consulta
/// `numberMatched` e publica uma página por recurso Raw para permitir retomada.
pub struct FonteCar;

impl FonteDeDados for FonteCar {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "Cadastro Ambiental Rural — perímetros de imóveis rurais por UF (SICAR/WFS)"
    }

    fn sob_demanda(&self) -> bool {
        true
    }

    fn usar_tls_nativo(&self) -> bool {
        true
    }

    fn perfil_http(&self) -> PerfilHttp {
        PerfilHttp {
            credencial: None,
            limite: Some(LimiteHttp {
                host: "geoserver.car.gov.br",
                requisicoes: 1,
                intervalo: Duration::from_secs(1),
            }),
            agente: None,
            redirecionamento: RedirecionamentoHttp::Seguir,
            aceitar_certificado_invalido: false,
        }
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        UFS.iter()
            .map(|&uf| {
                TarefaPlanejada::nova(TarefaCar {
                    nome: format!("imoveis/{uf}"),
                    uf,
                })
                .com_etiquetas([("uf", uf), ("formato", "shapefile")])
            })
            .collect()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::collections::HashSet;
    use todeolho_nucleo::FiltroDeTarefas;

    #[test]
    fn total_27_tarefas() {
        assert_eq!(FonteCar.criar_tarefas().len(), 27);
    }

    #[test]
    fn nomes_sao_unicos() {
        let tarefas = FonteCar.criar_tarefas();
        let nomes: HashSet<_> = tarefas.iter().map(|t| t.tarefa.nome()).collect();
        assert_eq!(nomes.len(), tarefas.len());
    }

    #[test]
    fn filtra_por_uf() {
        let filtradas = FiltroDeTarefas::a_partir_de(&["uf=DF".to_owned()])
            .unwrap()
            .aplicar(FonteCar.criar_tarefas())
            .unwrap();
        assert_eq!(filtradas.len(), 1);
        assert_eq!(filtradas[0].tarefa.nome(), "imoveis/DF");
    }

    #[test]
    fn paginas_tem_indices_distintos() {
        assert!(url_wfs("BA", 0).contains("count=10000&startIndex=0"));
        assert!(url_wfs("BA", 10_000).contains("count=10000&startIndex=10000"));
    }

    #[test]
    fn le_contagem_wfs_e_rejeita_resposta_incompleta() {
        let xml = br#"<wfs:FeatureCollection numberMatched="1333886" numberReturned="0"/>"#;
        assert_eq!(ler_total(xml, "url").unwrap(), 1_333_886);
        assert!(ler_total(b"<wfs:FeatureCollection/>", "url").is_err());
    }

    #[test]
    fn perfil_da_fonte() {
        assert!(FonteCar.sob_demanda());
        assert!(FonteCar.usar_tls_nativo());
        assert!(FonteCar.perfil_http().credencial.is_none());
        let limite = FonteCar.perfil_http().limite.unwrap();
        assert_eq!(limite.host, "geoserver.car.gov.br");
        assert_eq!(limite.requisicoes, 1);
    }
}
