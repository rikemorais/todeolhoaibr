//! IDH global e índices compostos do Human Development Report (UNDP).
//!
//! O CSV completo traz HDI, GDI, GII, PHDI, MPI e seus componentes por país
//! e ano (1990-2023), publicado pelo UNDP/HDRO. Gravado opaco na Raw; o recorte
//! por país e a interpretação ficam no refino.

use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_http::TarefaDeIngestaoHttp;

use super::api;

const NOME: &str = "undp";
const URL_INDICES: &str =
    "https://hdr.undp.org/sites/default/files/2025_HDR/HDR25_Composite_indices_complete_time_series.csv";

/// IDH global e índices compostos de desenvolvimento humano (UNDP/HDRO).
///
/// O CSV completo traz todos os índices compostos (HDI, GDI, GII, PHDI, MPI)
/// e seus componentes por país e ano (1990-2023), gravado opaco na Raw
/// ([ADR 0014]). O recorte por país e a normalização são do refino ([ADR 0015]).
/// Ver issue #64.
///
/// Catálogo: <https://hdr.undp.org/data-center/documentation-and-downloads>
///
/// [ADR 0014]: ../../../docs/adr/0014-raw-opaca-e-refino-em-estagio-separado.md
/// [ADR 0015]: ../../../docs/adr/0015-arquitetura-de-dados-em-medalhao.md
pub struct FonteUndp;

impl FonteDeDados for FonteUndp {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "UNDP Human Development Report - IDH global e índices compostos por país"
    }

    fn perfil_http(&self) -> todeolho_nucleo::PerfilHttp {
        api::perfil_http()
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        vec![TarefaPlanejada::nova(
            TarefaDeIngestaoHttp::sem_paginacao(NOME, "indices-compostos", URL_INDICES.to_owned())
                .com_tamanho_tipico(2_100_000),
        )
        .com_etiquetas([("recurso", "indices-compostos")])]
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cria_uma_unica_tarefa_com_recurso_e_etiqueta() {
        let tarefas = FonteUndp.criar_tarefas();
        assert_eq!(tarefas.len(), 1);
        let tarefa = &tarefas[0];
        assert_eq!(tarefa.tarefa.nome(), "indices-compostos");
        assert_eq!(
            tarefa.etiquetas.get("recurso").map(String::as_str),
            Some("indices-compostos")
        );
    }

    #[test]
    fn a_url_aponta_para_o_csv_do_hdr_2025() {
        assert!(URL_INDICES.starts_with("https://hdr.undp.org/"));
        assert!(URL_INDICES.ends_with(".csv"));
    }

    #[test]
    fn nao_exige_credencial() {
        let perfil = FonteUndp.perfil_http();
        assert!(perfil.credencial.is_none());
    }

    #[test]
    fn declara_o_limite_do_host_do_undp() {
        let limite = FonteUndp.perfil_http().limite.expect("limite do UNDP");
        assert_eq!(limite.host, "hdr.undp.org");
    }
}
