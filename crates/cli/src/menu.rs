//! Menu de terminal para escolher o que ingerir. Só conhece texto entrando e saindo, então
//! funciona em qualquer terminal e pode ser testado sem um.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, BufRead, Write};

use todeolho_nucleo::{EspacoNecessario, FiltroDeTarefas, formatar_bytes};

/// O que o menu precisa saber de uma tarefa.
#[derive(Debug, Clone)]
pub struct OpcaoDeTarefa {
    pub etiquetas: BTreeMap<String, String>,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct OpcaoDeFonte {
    pub nome: String,
    pub tarefas: Vec<OpcaoDeTarefa>,
}

/// O que foi escolhido: o índice da fonte e o filtro a aplicar nela.
pub type Escolha = Vec<(usize, FiltroDeTarefas)>;

enum EscolhaInicial {
    Todas,
    Numeros(Vec<usize>),
}

/// Conduz a escolha. `None` significa que a pessoa desistiu.
pub fn escolher(
    fontes: &[OpcaoDeFonte],
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> io::Result<Option<Escolha>> {
    writeln!(saida, "\nFontes disponíveis:")?;
    // A coluna "Falta baixar" começa uma coluna depois do nome mais longo, então calculamos a
    // largura da coluna de nomes em tempo de execução em vez de fixá-la (nomes maiores que o
    // palpite antigo transbordavam e empurravam os valores para a direita).
    let largura_nome = fontes
        .iter()
        .map(|fonte| fonte.nome.chars().count())
        .chain(["Fonte".chars().count()])
        .max()
        .unwrap_or(0);
    writeln!(
        saida,
        "      {:<largura_nome$} {:>12}",
        "Fonte", "Falta baixar"
    )?;
    for (indice, fonte) in fontes.iter().enumerate() {
        writeln!(
            saida,
            "  {:>2}) {:<largura_nome$} {:>12}",
            indice + 1,
            fonte.nome,
            tamanho(&fonte.tarefas)
        )?;
    }
    let espaco = EspacoNecessario::somar(
        fontes
            .iter()
            .flat_map(|fonte| fonte.tarefas.iter().map(|tarefa| &tarefa.bytes)),
    );
    writeln!(saida, "\nTotal conhecido: {}", formatar_bytes(espaco.bytes))?;
    if espaco.sem_estimativa > 0 {
        writeln!(
            saida,
            "Tarefas sem estimativa: {}. + indica valor parcial.",
            espaco.sem_estimativa
        )?;
    }

    let Some(resposta) = perguntar_fontes(fontes.len(), entrada, saida)? else {
        return Ok(None);
    };
    let numeros = match resposta {
        EscolhaInicial::Todas => {
            return Ok(Some(
                (0..fontes.len())
                    .map(|indice| (indice, FiltroDeTarefas::default()))
                    .collect(),
            ));
        }
        EscolhaInicial::Numeros(numeros) => numeros,
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

/// Na primeira pergunta, Enter inclui todas as fontes; `q` ou fim da entrada sai.
fn perguntar_fontes(
    limite: usize,
    entrada: &mut impl BufRead,
    saida: &mut impl Write,
) -> io::Result<Option<EscolhaInicial>> {
    loop {
        write!(
            saida,
            "\nDigite os números das fontes, separados por vírgula (Enter para todas; q para sair): "
        )?;
        saida.flush()?;

        let mut linha = String::new();
        if entrada.read_line(&mut linha)? == 0 {
            return Ok(None);
        }
        let resposta = linha.trim();
        if resposta.eq_ignore_ascii_case("q") || resposta.eq_ignore_ascii_case("sair") {
            return Ok(None);
        }
        if resposta.is_empty() {
            return Ok(Some(EscolhaInicial::Todas));
        }
        match interpretar_numeros(resposta, limite) {
            Some(numeros) => return Ok(Some(EscolhaInicial::Numeros(numeros))),
            None => writeln!(
                saida,
                "Resposta inválida. Use números de 1 a {limite} ou q para sair."
            )?,
        }
    }
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
    let espaco = EspacoNecessario::somar(tarefas.iter().map(|tarefa| &tarefa.bytes));
    if espaco.sem_estimativa > 0 {
        if espaco.bytes == 0 {
            return "desconhecido".to_owned();
        }
        return format!("{}+", formatar_bytes(espaco.bytes));
    }
    formatar_bytes(espaco.bytes)
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
                tarefas: vec![OpcaoDeTarefa {
                    etiquetas: BTreeMap::new(),
                    bytes: Some(2_000_000),
                }],
            },
            OpcaoDeFonte {
                nome: "tse".into(),
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
    fn enter_na_primeira_pergunta_seleciona_todas_as_fontes() {
        let (escolha, saida) = responder("\n");
        assert_eq!(
            escolha,
            Some(vec![
                (0, FiltroDeTarefas::default()),
                (1, FiltroDeTarefas::default()),
            ])
        );
        assert!(!saida.contains("escolha ano"), "{saida}");
    }

    #[test]
    fn q_na_primeira_pergunta_e_desistencia() {
        assert_eq!(responder("q\n").0, None);
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
        assert_eq!(tamanho(&tarefas), "desconhecido");
    }

    #[test]
    fn tarefa_sem_estimativa_aparece_como_desconhecido() {
        let mut fontes = fontes();
        for tarefa in &mut fontes[1].tarefas {
            tarefa.bytes = None;
        }
        let mut saida = Vec::new();
        escolher(&fontes, &mut Cursor::new("\n"), &mut saida).unwrap();
        let texto = String::from_utf8(saida).unwrap();
        assert!(
            texto.contains("tse") && texto.contains("desconhecido"),
            "{texto}"
        );
        assert!(!texto.contains("a consultar"), "{texto}");
    }

    #[test]
    fn a_coluna_falta_baixar_alinha_uma_coluna_depois_do_nome_mais_longo() {
        // O nome mais longo passa da largura antiga fixa (16/19) e empurrava os valores para a
        // direita. A coluna de valores deve começar sempre na mesma posição, logo após o nome.
        let fontes = vec![
            OpcaoDeFonte {
                nome: "ibge".into(),
                tarefas: vec![OpcaoDeTarefa {
                    etiquetas: BTreeMap::new(),
                    bytes: Some(2_000_000),
                }],
            },
            OpcaoDeFonte {
                nome: "senado-processos-detalhes".into(),
                tarefas: vec![OpcaoDeTarefa {
                    etiquetas: BTreeMap::new(),
                    bytes: Some(0),
                }],
            },
        ];
        let mut saida = Vec::new();
        escolher(&fontes, &mut Cursor::new("\n"), &mut saida).unwrap();
        let texto = String::from_utf8(saida).unwrap();

        let coluna_valor = |linha: &str, marca: &str| linha.rfind(marca);
        let linha = |marca: &str| {
            texto
                .lines()
                .find(|l| l.contains(marca))
                .unwrap_or_else(|| panic!("sem a linha de {marca}:\n{texto}"))
                .to_owned()
        };

        let cabecalho = linha("Falta baixar");
        let linha_curta = linha(") ibge");
        let linha_longa = linha("senado-processos-detalhes");

        let col_cabecalho = coluna_valor(&cabecalho, "Falta baixar").unwrap();
        let col_curta = coluna_valor(&linha_curta, "1,9 MB").unwrap();
        let col_longa = coluna_valor(&linha_longa, "0 B").unwrap();

        // O fim das colunas de valor (borda direita do campo) coincide em todas as linhas.
        assert_eq!(
            col_cabecalho + "Falta baixar".len(),
            col_curta + "1,9 MB".len(),
            "cabeçalho x linha curta desalinhados:\n{texto}"
        );
        assert_eq!(
            col_curta + "1,9 MB".len(),
            col_longa + "0 B".len(),
            "linha curta x linha longa desalinhadas:\n{texto}"
        );
    }

    #[test]
    fn menu_mostra_total_conhecido_e_distingue_estimativas_parciais() {
        let mut fontes = fontes();
        fontes[0].tarefas.push(OpcaoDeTarefa {
            etiquetas: BTreeMap::new(),
            bytes: None,
        });
        let mut saida = Vec::new();
        escolher(&fontes, &mut Cursor::new("\n"), &mut saida).unwrap();
        let texto = String::from_utf8(saida).unwrap();

        assert!(texto.contains("Falta baixar"), "{texto}");
        // Linha do ibge: 2 MB conhecidos + 1 tarefa sem estimativa -> parcial.
        assert!(texto.contains("1,9 MB+"), "{texto}");
        // tse agora é estimado no menu, em vez de "a consultar".
        assert!(!texto.contains("a consultar"), "{texto}");
        // Total conhecido inclui todas as fontes (ibge 2 MB + tse ~1 MB).
        assert!(texto.contains("Total conhecido: 2,9 MB"), "{texto}");
        assert!(texto.contains("Tarefas sem estimativa: 1"), "{texto}");
    }
}
