# Contrato candidato — prestação de contas TSE 2008

**Escopo:** manifesto Bronze vigente de `tse/prestacao-contas/ano=2008`, versão `execucao=20261105T041222Z-8947609b`, consultado em 2026-11-05. A análise retornou schemas e contagens agregadas, sem nomes, documentos ou valores individuais.

## Seleção das partes

O [ZIP publicado pelo TSE](https://cdn.tse.jus.br/estatistica/sead/odsele/prestacao_contas/prestacao_contas_2008.zip), preservado na Raw local, contém quatro CSVs de contas, `LEIAME.pdf` e `leiaute-2008.txt`. O TXT descreve campos, delimitador e finalidade dos quatro conjuntos. O decodificador Bronze também o representou como `leiaute-2008`, um Parquet de **uma linha e uma coluna** chamada `Arquivo Eleições 2008`. Esse objeto é documentação da origem: o catálogo Silver deve selecionar **somente** as quatro partes de dados abaixo, por nome de tabela do manifesto, e ignorar `leiaute-2008` como entrada tabular.

| Parte Bronze | Tabela do manifesto | Linhas | Campo de data | Campo de valor | Contexto relevante |
| --- | --- | ---: | --- | --- | --- |
| `00001` | `despesas_candidatos_2008_brasil` | 4.347.657 | `DT_DESPESA` | `VR_DESPESA` | `SEQUENCIAL_CANDIDATO`, `SG_UE`, `NR_CANDIDATO`, `DS_NR_DOCUMENTO` |
| `00002` | `despesas_comites_2008_brasil` | 404.088 | `DT_DESPESA` | `VR_DESPESA` | `DS_ORGAO`, `SG_UE`, `NR_CNPJ`, `DS_NR_DOCUMENTO` |
| `00004` | `receitas_candidatos_2008_brasil` | 2.203.360 | `DT_RECEITA` | `VR_RECEITA` | `SEQUENCIAL_CANDIDATO`, `SG_UE`, `NR_CANDIDATO`, `CD_CPF_CNPJ_DOADOR` |
| `00005` | `receitas_comites_2008_brasil` | 168.940 | `DT_RECEITA` | `VR_RECEITA` | `DS_ORGAO`, `SG_UE`, `NR_CNPJ`, `CD_CPF_CNPJ_DOADOR` |

O grão candidato de cada tabela é uma linha de despesa ou receita publicada, de candidato ou comitê. Nenhum campo do leiaute é declarado identificador único do lançamento. `SEQUENCIAL_CANDIDATO` tem 66.607 valores distintos em 4.347.657 despesas e 66.634 em 2.203.360 receitas; identifica o candidato no leiaute, não a linha de conta. O contrato deve preservar cada linha pela referência técnica (`manifesto`, `parte`, `ordinal`) até validar chave e política de duplicatas.

## Tipos e qualidade observados

As datas não vazias das quatro partes converteram com `DD/MM/AAAA`; os valores não vazios converteram para `DECIMAL(18,2)` após trocar vírgula por ponto. A conversão de teste não verifica se algum valor precisaria de arredondamento, que deve ser recusado explicitamente na implementação. Os números de ausência são:

| Tabela | Data vazia | Valor vazio | Parse inválido entre preenchidos |
| --- | ---: | ---: | ---: |
| Despesas de candidatos | 0 | 0 | 0 |
| Despesas de comitês | 0 | 0 | 0 |
| Receitas de candidatos | 1 | 1 | 0 |
| Receitas de comitês | 0 | 0 | 0 |

Datas e valores vazios devem virar nulos tipados com contagem, sem inventar zero ou data. Valores não vazios com parse inválido ou mais de duas casas significativas devem bloquear a primeira publicação até revisão. Identificadores de candidato, CPF/CNPJ, documento, partido e localidade permanecem texto. `SG_UE` é descrito no leiaute como código de município; não inferir outro nível geográfico apenas pelo nome da coluna nem converter o código em número.

## Divergências do leiaute e do cabeçalho físico

O TXT do ZIP é documentação da própria origem, mas seus nomes não coincidem integralmente com os cabeçalhos dos CSVs publicados. Exemplos observados no schema Bronze:

- O leiaute chama o nome do candidato de `NO_CAND` nas despesas, enquanto o CSV usa `NM_CANDIDATO`; o número aparece como `NR_CAND` no TXT e `NR_CANDIDATO` no CSV.
- Na despesa de candidato, o TXT descreve `NR_CNPJ` de campanha e o CSV traz `CD_NUM_CNPJ`.
- Campos repetidos de UF/localidade são desambiguados no Parquet com sufixo `_1` (`SG_UE_1`, `NM_UE_1`), enquanto o TXT usa sufixo `1` sem separador.
- Os dois CSVs de comitê têm como cabeçalho literal `RTRIM(LTRIM(DR.DS_TITULO))` em uma coluna que o leiaute chama `DS_TITULO`. A equivalência precisa ser validada por posição e por conteúdo agregado antes de renomear a coluna Silver.

O mapeamento de nomes deve ser versionado para 2008 e por tabela. O leiaute não autoriza reaproveitar automaticamente os contratos de [2002](pesquisa-tse-2002.md) ou [2004/2006](pesquisa-tse-2004-2006.md). A política de cópias exatas e chave de negócio ainda requer validação; nenhuma deduplicação foi aprovada neste recorte.

## Método reproduzível

O manifesto Bronze selecionou as cinco partes da versão indicada. O arquivo de origem foi listado no ZIP Raw; somente o TXT de documentação foi lido como texto. Para as quatro tabelas de contas, as consultas DuckDB usaram `read_parquet(caminho, hive_partitioning=false)` e retornaram `COUNT(*)`, contagens de `NULL`/texto vazio, `COUNT(DISTINCT SEQUENCIAL_CANDIDATO)` onde existe, `TRY_STRPTIME(data, '%d/%m/%Y')` e `TRY_CAST(REPLACE(valor, ',', '.') AS DECIMAL(18,2))`. Nenhuma linha de conta foi retornada. A validação precisa ser repetida se o ponteiro Bronze mudar.
