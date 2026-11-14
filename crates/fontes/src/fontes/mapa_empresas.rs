//! Snapshots mensais do Mapa de Empresas publicados pelo MEMP.

use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "mapa_empresas";
const BASE: &str = "https://www.gov.br/memp/pt-br/acesso-a-informacao/dados-abertos";

/// Arquivos XLSX atualmente publicados no catálogo oficial de dados abertos.
/// Acrescente aqui cada novo snapshot mensal quando o MEMP o disponibilizar.
const SNAPSHOTS_PUBLICADOS: [(&str, &str); 4] = [
    ("2026-05", "06_arquivodados_mai26-mapa-de-empresas-v2.xlsx"),
    ("2026-06", "06_arquivodados_jun26-mapa-de-empresas.xlsx"),
    ("2026-07", "07_arquivodados_jul26-mapa-de-empresas.xlsx"),
    ("2026-08", "08_arquivodados_ago26-mapa-de-empresas.xlsx"),
];

fn tarefas() -> Vec<TarefaPlanejada> {
    SNAPSHOTS_PUBLICADOS
        .iter()
        .map(|(periodo, arquivo)| {
            let recurso = format!("movimentacao/periodo={periodo}");
            TarefaPlanejada::nova(TarefaDeArquivo::nova(
                format!("mapa-empresas-{periodo}"),
                NOME,
                recurso,
                url(arquivo),
            ))
            .com_etiquetas([("periodo", *periodo)])
        })
        .collect()
}

fn url(arquivo: &str) -> String {
    format!("{BASE}/{arquivo}/@@download/file")
}

/// Abertura, fechamento e empresas ativas por município e UF, em snapshots mensais.
///
/// Os XLSX são preservados sem alteração na Raw. Cada planilha contém todos os estados;
/// o período filtra a tarefa de ingestão e o recorte por UF é feito sobre as linhas do
/// arquivo no refino. Consulte `docs/fontes/mapa-empresas.md`.
pub struct FonteMapaEmpresas;

impl FonteDeDados for FonteMapaEmpresas {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "MEMP - empresas abertas, fechadas e ativas por município e UF, em snapshots mensais"
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        tarefas()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use todeolho_nucleo::FiltroDeTarefas;

    #[test]
    fn disponibiliza_apenas_snapshots_mensais_publicados() {
        let tarefas = FonteMapaEmpresas.criar_tarefas();
        let periodos: Vec<_> = tarefas
            .iter()
            .map(|tarefa| tarefa.etiquetas["periodo"].as_str())
            .collect();

        assert_eq!(periodos, ["2026-05", "2026-06", "2026-07", "2026-08"]);
    }

    #[test]
    fn permite_filtrar_por_periodo() {
        let filtro = FiltroDeTarefas::a_partir_de(&["periodo=2026-08".to_owned()]).unwrap();
        let selecionadas = filtro.aplicar(FonteMapaEmpresas.criar_tarefas()).unwrap();

        assert_eq!(selecionadas.len(), 1);
        assert_eq!(selecionadas[0].etiquetas["periodo"], "2026-08");
    }

    #[test]
    fn cada_snapshot_tem_recurso_e_url_estaveis() {
        let tarefas = FonteMapaEmpresas.criar_tarefas();

        for tarefa in tarefas {
            let periodo = &tarefa.etiquetas["periodo"];
            assert_eq!(tarefa.tarefa.nome(), format!("mapa-empresas-{periodo}"));
        }
    }

    #[test]
    fn url_usa_o_download_do_anexo_oficial() {
        assert_eq!(
            url("08_arquivodados_ago26-mapa-de-empresas.xlsx"),
            "https://www.gov.br/memp/pt-br/acesso-a-informacao/dados-abertos/08_arquivodados_ago26-mapa-de-empresas.xlsx/@@download/file"
        );
    }
}
