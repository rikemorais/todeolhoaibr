//! Menu de terminal para escolher o que ingerir. Só conhece texto entrando e saindo, então
//! funciona em qualquer terminal e pode ser testado sem um.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead, Write};

use todeolho_nucleo::{FiltroDeTarefas, formatar_bytes};

/// O que o menu precisa saber de uma tarefa.
#[derive(Debug, Clone)]
pub struct OpcaoDeTarefa {
    pub etiquetas: BTreeMap<String, String>,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct OpcaoDeFonte {
    pub nome: String,
    pub descricao: String,
    pub tarefas: Vec<OpcaoDeTarefa>,
}

/// O que foi escolhido: o índice da fonte e o filtro a aplicar nela.
pub type Escolha = Vec<(usize, FiltroDeTarefas)>;

/// Conduz a escolha. `None` significa que a pessoa desistiu.
pub fn escolher(
    fontes: &[OpcaoDeFonte],
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> io::Result<Option<Escolha>> {
    writeln!(saida, "\nFontes disponíveis:\n")?;
    for (indice, fonte) in fontes.iter().enumerate() {
        writeln!(
            saida,
            "  {}) {:<8} {:>9}  {}",
            indice + 1,
            fonte.nome,
            tamanho(&fonte.tarefas),
            fonte.descricao
        )?;
    }

    let Some(numeros) = perguntar_numeros(
        "\nDigite os números das fontes, separados por vírgula (Enter para sair): ",
        fontes.len(),
        entrada,
        saida,
    )?
    else {
        return Ok(None);
    };

    let mut escolha = Vec::new();
    for indice in numeros {
        let fonte = &fontes[indice];
        let mut filtro = FiltroDeTarefas::default();
        let mut restantes: Vec<&OpcaoDeTarefa> = fonte.tarefas.iter().collect();

        for dimensao in dimensoes(&fonte.tarefas) {
            let valores = valores_com_tamanho(&restantes, &dimensao);
            if valores.len() < 2 {
                continue;
            }

            writeln!(saida, "\n{}: escolha {dimensao}\n", fonte.nome)?;
            for (posicao, (valor, bytes)) in valores.iter().enumerate() {
                writeln!(saida, "  {:>2}) {valor:<6} {bytes:>9}", posicao + 1)?;
            }

            let Some(numeros) = perguntar_numeros(
                "\nDigite os números, separados por vírgula (Enter para todos): ",
                valores.len(),
                entrada,
                saida,
            )?
            else {
                continue;
            };

            let escolhidos: BTreeSet<&str> =
                numeros.iter().map(|&n| valores[n].0.as_str()).collect();
            filtro.restringir(&dimensao, escolhidos.iter().copied());
            restantes.retain(|tarefa| {
                tarefa
                    .etiquetas
                    .get(&dimensao)
                    .is_some_and(|valor| escolhidos.contains(valor.as_str()))
            });
        }

        escolha.push((indice, filtro));
    }
    Ok(Some(escolha))
}

/// Pergunta até receber uma resposta válida. Enter ou fim da entrada devolvem `None`.
fn perguntar_numeros(
    pergunta: &str,
    limite: usize,
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> io::Result<Option<Vec<usize>>> {
    loop {
        write!(saida, "{pergunta}")?;
        saida.flush()?;

        let mut linha = String::new();
        if entrada.read_line(&mut linha)? == 0 || linha.trim().is_empty() {
            return Ok(None);
        }

        match interpretar_numeros(&linha, limite) {
            Some(numeros) => return Ok(Some(numeros)),
            None => writeln!(saida, "Resposta inválida. Use números de 1 a {limite}.")?,
        }
    }
}

fn interpretar_numeros(linha: &str, limite: usize) -> Option<Vec<usize>> {
    let mut numeros = BTreeSet::new();
    for parte in linha.split(',') {
        let numero: usize = parte.trim().parse().ok()?;
        if numero == 0 || numero > limite {
            return None;
        }
        numeros.insert(numero - 1);
    }
    Some(numeros.into_iter().collect())
}

fn dimensoes(tarefas: &[OpcaoDeTarefa]) -> BTreeSet<String> {
    tarefas
        .iter()
        .flat_map(|tarefa| tarefa.etiquetas.keys().cloned())
        .collect()
}

fn valores_com_tamanho(tarefas: &[&OpcaoDeTarefa], dimensao: &str) -> Vec<(String, String)> {
    let mut grupos: BTreeMap<&str, Vec<OpcaoDeTarefa>> = BTreeMap::new();
    for tarefa in tarefas {
        if let Some(valor) = tarefa.etiquetas.get(dimensao) {
            grupos.entry(valor).or_default().push((*tarefa).clone());
        }
    }
    grupos
        .into_iter()
        .map(|(valor, tarefas)| (valor.to_owned(), tamanho(&tarefas)))
        .collect()
}

fn tamanho(tarefas: &[OpcaoDeTarefa]) -> String {
    let soma: u64 = tarefas.iter().filter_map(|tarefa| tarefa.bytes).sum();
    if soma == 0 && tarefas.iter().any(|tarefa| tarefa.bytes.is_none()) {
        return "?".to_owned();
    }
    formatar_bytes(soma)
}

#[cfg(test)]
mod testes {
    use std::io::Cursor;

    use super::*;

    fn tarefa(ano: &str, uf: &str, bytes: u64) -> OpcaoDeTarefa {
        OpcaoDeTarefa {
            etiquetas: BTreeMap::from([
                ("ano".to_owned(), ano.to_owned()),
                ("uf".to_owned(), uf.to_owned()),
            ]),
            bytes: Some(bytes),
        }
    }

    fn fontes() -> Vec<OpcaoDeFonte> {
        vec![
            OpcaoDeFonte {
                nome: "ibge".into(),
                descricao: "IBGE".into(),
                tarefas: vec![OpcaoDeTarefa {
                    etiquetas: BTreeMap::new(),
                    bytes: Some(2_000_000),
                }],
            },
            OpcaoDeFonte {
                nome: "tse".into(),
                descricao: "TSE".into(),
                tarefas: vec![
                    tarefa("2022", "AC", 1024),
                    tarefa("2022", "SP", 1_048_576),
                    tarefa("2024", "AC", 2048),
                ],
            },
        ]
    }

    fn responder(respostas: &str) -> (Option<Escolha>, String) {
        let mut saida = Vec::new();
        let escolha = escolher(
            &fontes(),
            &mut Cursor::new(respostas.to_owned()),
            &mut saida,
        )
        .unwrap();
        (escolha, String::from_utf8(saida).unwrap())
    }

    fn filtro(textos: &[&str]) -> FiltroDeTarefas {
        let textos: Vec<String> = textos.iter().map(|t| (*t).to_owned()).collect();
        FiltroDeTarefas::a_partir_de(&textos).unwrap()
    }

    #[test]
    fn enter_na_primeira_pergunta_e_desistencia() {
        assert_eq!(responder("\n").0, None);
    }

    #[test]
    fn fim_da_entrada_e_desistencia() {
        assert_eq!(responder("").0, None);
    }

    #[test]
    fn fonte_sem_etiquetas_nao_faz_perguntas_extras() {
        let (escolha, saida) = responder("1\n");

        assert_eq!(escolha, Some(vec![(0, FiltroDeTarefas::default())]));
        assert!(!saida.contains("escolha ano"));
    }

    #[test]
    fn mostra_o_tamanho_de_cada_opcao() {
        let (_, saida) = responder("2\n\n\n");

        assert!(saida.contains("1,0 MB"), "{saida}");
        assert!(saida.contains("2024"), "{saida}");
        assert!(saida.contains("2,0 KB"), "{saida}");
    }

    #[test]
    fn filtra_por_ano_e_uf() {
        let (escolha, _) = responder("2\n1\n2\n");

        assert_eq!(escolha, Some(vec![(1, filtro(&["ano=2022", "uf=SP"]))]));
    }

    #[test]
    fn enter_em_um_filtro_significa_todos() {
        let (escolha, _) = responder("2\n\n1\n");

        assert_eq!(escolha, Some(vec![(1, filtro(&["uf=AC"]))]));
    }

    #[test]
    fn nao_oferece_opcao_que_nao_existe_depois_do_filtro_anterior() {
        // Em 2024 só existe AC: não há o que perguntar sobre a UF.
        let (escolha, saida) = responder("2\n2\n");

        assert_eq!(escolha, Some(vec![(1, filtro(&["ano=2024"]))]));
        assert!(!saida.contains("escolha uf"), "{saida}");
    }

    #[test]
    fn repete_a_pergunta_quando_a_resposta_e_invalida() {
        for invalida in ["9", "0", "abc", "1,x"] {
            let (escolha, saida) = responder(&format!("{invalida}\n1\n"));

            assert!(saida.contains("Resposta inválida"), "{invalida}");
            assert_eq!(escolha, Some(vec![(0, FiltroDeTarefas::default())]));
        }
    }

    #[test]
    fn varias_fontes_sem_repeticao() {
        let (escolha, _) = responder("1, 2,1\n\n\n");

        let indices: Vec<usize> = escolha.unwrap().iter().map(|(i, _)| *i).collect();
        assert_eq!(indices, [0, 1]);
    }

    #[test]
    fn tamanho_desconhecido_nao_aparece_como_zero() {
        let tarefas = [OpcaoDeTarefa {
            etiquetas: BTreeMap::new(),
            bytes: None,
        }];
        assert_eq!(tamanho(&tarefas), "?");
    }
}
