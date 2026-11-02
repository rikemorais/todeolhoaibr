use todeolho_nucleo::{FonteDeDados, TarefaPlanejada};

use crate::ingestao_arquivo::TarefaDeArquivo;

const NOME: &str = "tse";
const BASE: &str = "https://cdn.tse.jus.br/estatistica/sead/odsele/votacao_secao";

/// Primeira eleição com votação por seção publicada pelo TSE.
const PRIMEIRA_ELEICAO: u16 = 1998;
/// Atualizar a cada nova eleição.
const ULTIMA_ELEICAO: u16 = 2026;

const ESTADOS: [&str; 26] = [
    "AC", "AL", "AM", "AP", "BA", "CE", "ES", "GO", "MA", "MG", "MS", "MT", "PA", "PB", "PE", "PI",
    "PR", "RJ", "RN", "RO", "RR", "RS", "SC", "SE", "SP", "TO",
];

fn eleicoes() -> impl Iterator<Item = u16> {
    (PRIMEIRA_ELEICAO..=ULTIMA_ELEICAO).step_by(2)
}

/// Unidades publicadas em cada eleição. Eleições gerais incluem o DF, que não tem eleição
/// municipal, e `BR`. Em 2010 o TSE publicou também `VT` e `ZZ`.
fn unidades(ano: u16) -> Vec<&'static str> {
    let mut unidades = ESTADOS.to_vec();
    if (ano - PRIMEIRA_ELEICAO).is_multiple_of(4) {
        unidades.extend(["DF", "BR"]);
    }
    if ano == 2010 {
        unidades.extend(["VT", "ZZ"]);
    }
    unidades
}

/// Votação por seção eleitoral: <https://dadosabertos.tse.jus.br/dataset/?tags=Resultados>
///
/// Cada linha traz UF, município, zona, seção, cargo, candidato e votos, então um único
/// conjunto atende às consultas por seção, cidade e estado.
pub struct FonteTse;

impl FonteDeDados for FonteTse {
    fn nome(&self) -> &str {
        NOME
    }

    fn descricao(&self) -> &str {
        "TSE - Votos por seção eleitoral, de 1998 a 2026"
    }

    /// São cerca de 16 GB: não devem entrar em `--todas` por acidente.
    fn sob_demanda(&self) -> bool {
        true
    }

    fn criar_tarefas(&self) -> Vec<TarefaPlanejada> {
        let mut tarefas = Vec::new();
        for ano in eleicoes() {
            for uf in unidades(ano) {
                tarefas.push(
                    TarefaPlanejada::nova(TarefaDeArquivo::nova(
                        format!("{ano}_{uf}"),
                        NOME,
                        format!("votacao_secao/ano={ano}/uf={uf}"),
                        format!("{BASE}/votacao_secao_{ano}_{uf}.zip"),
                    ))
                    .com_etiquetas([("ano", ano.to_string().as_str()), ("uf", uf)]),
                );
            }
        }
        tarefas
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn nomes_de(ano: &str) -> Vec<String> {
        FonteTse
            .criar_tarefas()
            .iter()
            .filter(|t| t.etiquetas.get("ano").is_some_and(|valor| valor == ano))
            .map(|t| t.tarefa.nome().to_owned())
            .collect()
    }

    #[test]
    fn cobre_os_408_arquivos_publicados() {
        assert_eq!(FonteTse.criar_tarefas().len(), 408);
    }

    #[test]
    fn eleicoes_municipais_nao_tem_df_e_gerais_tem() {
        assert_eq!(nomes_de("2024").len(), 26);
        assert!(!nomes_de("2024").contains(&"2024_DF".to_owned()));
        assert_eq!(nomes_de("2022").len(), 28);
        assert!(nomes_de("2022").contains(&"2022_DF".to_owned()));
    }

    #[test]
    fn toda_tarefa_pode_ser_filtrada_por_ano_e_uf() {
        for tarefa in FonteTse.criar_tarefas() {
            assert!(tarefa.etiquetas.contains_key("ano"));
            assert!(tarefa.etiquetas.contains_key("uf"));
        }
    }

    #[test]
    fn nao_entra_em_todas_por_acidente() {
        assert!(FonteTse.sob_demanda());
    }
}
