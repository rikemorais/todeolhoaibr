# Fontes oficiais do TSE: prestação de contas 2016–2022

Pesquisa realizada em 9 de novembro de 2026 para apoiar o contrato de publicação Silver da issue #185. O foco é o que as fontes oficiais descrevem sobre pacotes, leiautes e recortes territoriais, distinguindo a existência de arquivos da semântica de seus registros.

## Pergunta

O que o TSE documenta sobre os conjuntos e leiautes de prestação de contas de 2016, 2018, 2020 e 2022? As fontes oficiais explicam se arquivos por UF, nacionais (`BR`/`BRASIL`) e suplementares são partes equivalentes, complementares ou substitutas?

## Fontes e datas

1. [Conjunto oficial de prestação de contas 2016](https://dadosabertos.tse.jus.br/dataset/prestacao-de-contas-eleitorais-2016), publicado pelo TSE/ASEPA; página indica criação em 24/08/2021. A página distingue os recursos “Prestação de contas final suplementar” e “Prestações de contas finais”, ambos com escopo exibido como “Todas as UFs”.
2. [Conjunto oficial de prestação de contas 2018](https://dadosabertos.tse.jus.br/dataset/prestacao-de-contas-eleitorais-2018), TSE/ASEPA; criação indicada em 17/08/2021. A página lista recursos de órgãos partidários e candidatos, ambos “Todas as UFs”.
3. [Conjunto oficial de prestação de contas 2020](https://dadosabertos.tse.jus.br/dataset/prestacao-de-contas-eleitorais-2020), TSE/ASEPA; criação indicada em 24/08/2021. Lista recursos de órgãos partidários e candidatos, ambos “Todas as UFs”.
4. [Conjunto oficial de prestação de contas 2022](https://dadosabertos.tse.jus.br/dataset/dadosabertos-tse-jus-br-dataset-prestacao-de-contas-eleitorais-2022), TSE/ASEPA; criação indicada em 23/08/2022. Lista recursos de órgãos partidários e candidatos, ambos “Todas as UFs”.
5. [Grupo oficial “Prestação de Contas Eleitorais”](https://dadosabertos.tse.jus.br/group/about/prestacao-de-contas-eleitorais). O TSE avisa que regras e estruturas podem variar conforme a eleição e recomenda consultar o `leia-me.pdf` de cada pacote.

Os pacotes examinados são distribuídos pelo CDN oficial `cdn.tse.jus.br`. Os membros PDF foram lidos diretamente dos arquivos oficiais; a lista de membros/nomes de arquivos abaixo é uma observação do conteúdo dos pacotes, não uma declaração adicional do TSE sobre relação de dados.

## Achados

### 2016

- O portal lista explicitamente um recurso separado de “prestação de contas final suplementar” e outro de “prestações de contas finais”. Isso comprova que o TSE distribui categorias de recurso distintas, não o significado linha a linha de `_sup` nem sua relação com a prestação regular.
- O pacote oficial de prestação final suplementar contém tabelas de receitas e despesas de candidatos e partidos, além do PDF `Leiaute_RDE_2016_1.0.1.pdf` (versão 1.0.1, datado de agosto de 2016). O leiaute descreve campos e sentinelas, mas não define `_sup`, `_BR`/`_brasil`, nem uma regra de união, precedência ou deduplicação.
- Na inventariação local do Bronze, as quatro famílias `*_prestacao_contas_final_2016` aparecem com 28 partes cada: 26 UFs, `_brasil` e `_sup`. Isso é evidência de nomes e presença no snapshot importado; o portal, por si, não especifica essa composição nem o contrato semântico dos sufixos.

### 2018, 2020 e 2022

- As páginas oficiais listam dois pacotes principais: órgãos partidários e candidatos. Os pacotes de cada ano incluem leia-mes para despesas contratadas, despesas pagas, receitas e receitas de doador originário, para os dois grupos. Os leia-mes PDF correspondentes nos pacotes 2018, 2020 e 2022 são byte a byte idênticos nas cópias oficiais consultadas.
- Os leia-mes documentam padrões de nomes como `..._<ANO>_<UF>` e `..._<ANO>_BRASIL`. Também definem o campo `UF`: `BR` indica informação em nível nacional; `VT`, voto em trânsito; e `ZZ`, exterior. Portanto, `BR` no campo é um valor de abrangência nacional, não uma UF.
- Para candidatos, o leiaute define a abrangência de candidatura: `BR` para Presidente/Vice-Presidente; a sigla da UF para cargos estaduais; e código municipal para cargos municipais. O nome da unidade eleitoral para Presidente é “Brasil”. Para órgãos partidários, o leiaute separa esfera partidária (federal, estadual/distrital, municipal/zonal) de UF/município do prestador. Essas descrições caracterizam campos/entidades; não afirmam que um arquivo terminado em `_BRASIL` seja soma ou cópia das partes territoriais.
- Os leia-mes definem `TP_PRESTACAO_CONTAS`: `0` final, `1` relatório financeiro e `2` parcial. Também explicam que cálculos divulgados no DivulgaCandContas usam a última entrega do tipo Final recepcionada com sucesso pelo SPCE-WEB, para os cálculos especificados. Isso não estabelece que cada tabela exportada seja filtrada pelo mesmo critério, nem que o final exclua retificações ou o conjunto de outra categoria.
- Nos leia-mes, “eleições suplementares” (e extraordinárias, no caso do vocabulário a partir de 2022) são eleições sem periodicidade fixa. Os eventos suplementares ocorridos entre eleições ordinárias são incluídos no pacote/arquivo do ano da eleição ordinária correspondente. Esta definição trata de tipo/ano de eleição, não de um arquivo denominado “prestação de contas final suplementar” em separado. Não se deve usar essa passagem como explicação do `_sup` de 2016.
- Na contagem do Bronze local, 2018 e 2022 têm 29 partes por família (27 UFs, `_BR`, `_BRASIL`); 2020 tem 27 (26 UFs, `_BRASIL`). São inventários do snapshot do projeto, não semântica declarada pelo TSE. Em 2020, a ausência de `_BR` no nome importado tampouco prova que o pacote não tenha informação nacional: o leia-me identifica `BR` como valor de campo e `_BRASIL` como forma do nome da tabela.

## Limites da evidência

- As páginas do portal indicam escopo amplo (“Todas as UFs”), fonte SPCE e pacotes disponíveis; isso não equivale a afirmar que os arquivos internos sejam particionados de um modo específico ou contenham linhas não sobrepostas.
- A definição de `BR` como valor do campo `UF` e a convenção `_BRASIL` para nome de tabela não demonstram equivalência entre as duas formas, nem que o nacional seja união de UFs. Não encontrei nos leia-mes regra oficial sobre precedência, composição ou deduplicação de `_BR`, `_BRASIL` e arquivos por UF.
- O portal diferencia a prestação final suplementar de 2016 da final regular, mas não explica a relação de cobertura entre elas (aditivo, retificação, segundo turno ou outra hipótese). Não há base oficial suficiente para escolher uma interpretação pelo nome.
- A frase sobre “última entrega [...] do tipo Final” aparece em observações de cálculos específicos do DivulgaCandContas. Generalizá-la para o conteúdo integral das tabelas excederia a fonte.
- Inventário, igualdade binária dos leia-mes entre anos e contagens de partes são observações locais dos artefatos consultados. Não constituem contrato oficial nem verificam a igualdade de conteúdo das tabelas nacionais e estaduais.

## Conclusão para o contrato Silver

Há evidência oficial explícita de que 2016 publica recursos distintos de prestação final regular e final suplementar; em 2018–2022, os leia-mes definem campos e convenções territoriais e registram eleições suplementares dentro do pacote do ano ordinário correspondente. O TSE documenta `BR` como valor nacional de campo e `_BRASIL` como padrão de nome de tabela. **Não há evidência oficial suficiente para afirmar que recortes por UF, `_BR`, `_BRASIL` sejam equivalentes, complementares ou substitutos, nem para definir a semântica ou precedência do `_sup` de 2016.** O contrato deve preservar essas origens como conjuntos distintos até que uma validação de conteúdo e uma regra de domínio sustentada por evidência sejam concluídas.

O resultado da validação de conteúdo do snapshot local (schemas, contagens, SHA-256 de linha e multiplicidades) está registrado em [pesquisa-tse-2014-2022.md](pesquisa-tse-2014-2022.md). Esses resultados locais não ampliam o que o TSE declara oficialmente.
