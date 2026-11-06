# Prestação de contas do TSE em 2010 e 2012 — recortes para a Silver

**Escopo:** inventário das versões Bronze vigentes e validação inicial dos formatos de data e valor em uma parte por família. As contagens por família vêm dos metadados Parquet de **todas** as partes. As conversões foram testadas apenas nas partes identificadas abaixo; não definem ainda chave de lançamento nem autorizam deduplicação. Consulta local em 2026-11-05, vinculada à [fase 0 (#185)](https://github.com/rikemorais/todeolhoaibr/issues/185).

## Seleção reproduzível

Use o ponteiro `_ultima_versao.json` de cada recurso e o manifesto apontado. Nesta consulta, as execuções vigentes eram `20261105T041222Z-8947609b` para `ano=2010` e `20261105T185603Z-f00f20d4` para `ano=2012`. O manifesto de 2012 traz a regra `receitas-partidos-sem-cabecalho-v1`. Os números abaixo não combinam execuções históricas.

### 2010

O ZIP da Raw organiza as seis famílias em `candidato/<UF>/`, `comite/<UF>/` e `partido/<UF>/`, com 27 UFs e `BR`. O manifesto Bronze registra apenas o nome da família, repetido 28 vezes; o caminho de origem com a UF **não está no campo `url`**. As partes consultadas de PE e BR têm uma só UF na coluna `UF`, coerente com o diretório original. Antes da publicação Silver, a rotina precisa verificar essa correspondência em todas as partes e preservar parte e ordinal como procedência.

| Família do manifesto | Partes | Linhas em todas as partes | Parte PE consultada | Colunas |
| --- | ---: | ---: | ---: | ---: |
| `ReceitasCandidatos` | 28 | 427.017 | `00001` | 19 |
| `DespesasCandidatos` | 28 | 1.907.800 | `00002` | 19 |
| `DespesasComites` | 28 | 67.084 | `00057` | 14 |
| `ReceitasComites` | 28 | 15.045 | `00058` | 14 |
| `ReceitasPartidos` | 28 | 16.027 | `00113` | 14 |
| `DespesasPartidos` | 28 | 16.635 | `00114` | 14 |

Nas seis partes PE, `Data da receita` ou `Data da despesa` preenchida converteu com `%d/%m/%Y` ou `%Y-%m-%d`, e `Valor receita` ou `Valor despesa` com vírgula decimal converteu para `DECIMAL(18,2)`. Não houve campos vazios nem falhas nessas partes. A coluna `Data e hora` da parte `00001` tem 6.716 valores preenchidos, todos iguais, no formato `%d/%m/%Y%H:%M:%S` (sem espaço); ela não identifica um lançamento e não deve substituir a data da receita. Esses testes não cobrem as outras UFs ou a parte BR.

### 2012

As famílias têm contagens e schemas diferentes. `despesas_partidos` e `receitas_partidos` incluem tanto `_brasil` quanto `_BR`; as outras quatro têm `_brasil` e partes por UF. As contagens da coluna “demais partes” incluem `_BR` quando presente. O consolidado **não deve ser somado às partes estaduais** para formar uma tabela canônica sem regra de cobertura: em todas as famílias, seu volume difere da soma das demais partes.

| Família | Partes | Linhas `_brasil` | Linhas nas demais partes | Parte `_brasil` |
| --- | ---: | ---: | ---: | ---: |
| `despesas_candidatos` | 27 | 5.123.841 | 2.704.246 | `00006` |
| `despesas_comites` | 27 | 422.955 | 182.841 | `00033` |
| `despesas_partidos` | 28 | 74.788 | 38.837 | `00060` |
| `receitas_candidatos` | 27 | 2.782.157 | 1.997.288 | `00088` |
| `receitas_comites` | 27 | 35.383 | 35.211 | `00115` |
| `receitas_partidos` | 28 | 38.406 | 10.528 | `00142` |

Os seis `_brasil` têm, respectivamente, 23, 20, 20, 28, 21 e 25 colunas. Seus nomes variam em acentuação e grafia (`Número UE`/`Numero UE`, `Município`/`Municipio`, `Espécie recurso`/`Especie recurso`). Cada família exige mapeamento explícito. `receitas_partidos_2012_BR` e as 26 partes por UF têm 19 colunas posicionais `coluna_001`…`coluna_019`, enquanto `_brasil` tem 25 colunas nomeadas. O [levantamento do leiaute oficial](pesquisa-tse-2012.md) mantém pendente o significado das 19 posições. Não atribuir a elas os nomes do consolidado.

Nas partes `_brasil` consultadas, todas as datas e valores de lançamento estão preenchidos. Datas das três famílias de despesas e de `receitas_partidos` converteram com `%d/%m/%Y` ou `%Y-%m-%d`. `receitas_candidatos` usa `%d/%m/%Y%H:%M:%S` e `receitas_comites` usa `%d-%b-%y`; todas as 2.782.157 e 35.383 datas, respectivamente, converteram nesses formatos específicos. Todos os valores consultados converteram para `DECIMAL(18,2)` após a normalização de vírgula decimal. Uma regra de data global para todo 2012 rejeitaria linhas válidas.

## Reconciliação de UFs e comitês (2026-11-05)

A [issue #208](https://github.com/rikemorais/todeolhoaibr/issues/208) aprofunda dois recortes da fase 0. As versões Bronze são as mesmas identificadas acima.

Em 2010, os **168 membros TXT** do ZIP Raw e as **168 partes** do manifesto estão na mesma ordem. O nome da família de cada parte corresponde ao nome do membro, e a única `UF` preenchida nas suas linhas corresponde ao diretório daquele membro. Cada uma das seis famílias tem 28 UFs distintas, exatamente uma parte `BR`, e nenhuma parte com UF vazia ou misturada. Isso valida a reconstrução da UF a partir da coluna para esta versão, mas o contrato ainda deve guardar a referência à parte porque o manifesto omite o caminho do membro.

Em 2012, comparei todas as 26 partes por UF com `_brasil` nas duas famílias de comitês. As projeções usaram somente colunas comuns, descartaram `Data e hora` e as colunas exclusivas do consolidado, converteram as datas para `DATE` e os valores para `DECIMAL(18,2)`. `INTERSECT ALL` e `EXCEPT ALL` preservaram a multiplicidade de linhas idênticas. Nenhuma data ou valor falhou na conversão. As datas estaduais usam `%d/%m/%y`; `_brasil` usa `%d-%b-%y` para receitas e `%d/%m/%Y` para despesas.

| Família de 2012 | Linhas UF | Linhas `_brasil` | Linhas comuns | Só nas UFs | Só em `_brasil` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `receitas_comites` | 35.211 | 35.383 | 34.806 | 405 | 577 |
| `despesas_comites` | 182.841 | 422.955 | 181.577 | 1.264 | 241.378 |

Portanto `_brasil` **não é cópia exata nem superconjunto** das partes por UF nessa projeção. Publicar somente um lado perderia linhas observadas no outro; unir os dois sem conciliação duplicaria milhares de linhas. A diferença pode refletir cobertura, correções ou atributos, mas estes testes não estabelecem sua causa nem a precedência correta.

Testei a combinação candidata (`UF`, `Sequencial Comite`, `Número do documento`, data, valor) após as mesmas conversões. Ela não é única, e o documento está vazio em parte relevante das linhas:

| Família e lado | Linhas | Combinações distintas | Documento vazio |
| --- | ---: | ---: | ---: |
| Receitas, UFs | 35.211 | 34.213 | 14.182 |
| Receitas, `_brasil` | 35.383 | 34.469 | 14.219 |
| Despesas, UFs | 182.841 | 158.701 | 52.872 |
| Despesas, `_brasil` | 422.955 | 336.245 | 124.219 |

`UF` e `Sequencial Comite` não estavam vazios nesses quatro recortes. A combinação continua exploratória; não usar `DISTINCT` ou o documento vazio como regra de deduplicação. Preservar lado, parte, ordinal e versão Bronze para investigar as diferenças.

## Contrato candidato e pendências

Tratar cada família e ano como contrato separado. No caso de 2012, separar ainda o leiaute posicional de receitas partidárias. O identificador técnico (`versão Bronze`, `parte`, `ordinal`) preserva todas as linhas, inclusive variantes, até que grão, chave de negócio, política de consolidação e deduplicação sejam comprovados. `UF`, sequência de candidato/comitê/diretório, documento, data e valor são campos a testar; nenhum deles foi validado aqui como chave de lançamento.

Antes de materializar: verificar schemas e conversões em todas as partes, comparar `_brasil` com UFs e `_BR` por conteúdo nas famílias ainda não reconciliadas, investigar as diferenças dos comitês, medir nulos/sentinelas e colisões de chaves por família, definir precedência dos consolidados e obter o leiaute oficial das 19 posições de 2012. Não publicar uma tabela Silver canônica dessas famílias apenas com este inventário.

## Método

O inventário usa os manifestos vigentes e `parquet_metadata` do DuckDB para somar linhas por arquivo sem ler registros individuais. `DESCRIBE` identificou os schemas das partes listadas. `COUNT`, `TRY_STRPTIME` e `TRY_CAST` mediram as conversões agregadas. Os nomes dos diretórios de 2010 foram conferidos no ZIP Raw local. Nenhum identificador pessoal ou linha individual foi copiado para esta nota.
