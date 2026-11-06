# Contrato candidato — prestação de contas TSE 2002

**Escopo:** quatro partes do manifesto bronze vigente de `tse/prestacao-contas/ano=2002`, consultadas em 2026-11-05. Esta nota usa somente schemas e contagens agregadas; não exibe registros, nomes, documentos ou valores individuais. A versão consultada é `execucao=20261104T220445Z-f211faa3`.

## Quatro tabelas internas

O manifesto contém uma parte por conjunto. Os campos abaixo são os campos **físicos** do Parquet; `ano`, `data_ingestao` e `execucao` aparecem na leitura padrão do DuckDB por inferência do caminho, mas não são colunas armazenadas. O ano da Silver deve vir do recurso `ano=2002` e ser validado contra o contexto de execução.

| Tabela do manifesto | Linhas | Colunas físicas de interesse | Grão candidato |
| --- | ---: | --- | --- |
| `DespesaCandidato` | 572.219 | `SEQUENCIAL_CANDIDATO`, `SG_UF`, `SG_PART`, `DS_CARGO`, `NO_CAND`, `NR_CAND`, `DT_DOC_DESP`, `CD_CPF_CGC`, `SG_UF_FORNECEDOR`, `NO_FOR`, `VR_DESPESA`, `DS_TITULO` | uma linha de despesa de candidato publicada |
| `ReceitaCandidato` | 134.509 | `SEQUENCIAL_CANDIDATO`, `SG_UF`, `SG_PART`, `DS_CARGO`, `NO_CAND`, `NR_CAND`, `DT_RECEITA`, `CD_CPF_CGC`, `SG_UF_DOADOR`, `NO_DOADOR`, `VR_RECEITA`, `TP_RECURSO` | uma linha de receita de candidato publicada |
| `DespesaComite` | 38.037 | `SG_UF`, `SG_PART`, `DS_ORGAO`, `DT_DOC_DESP`, `CD_CPF_CGC`, `SG_UF_FORNECEDOR`, `NO_FOR`, `VR_DESPESA`, `DS_TITULO` | uma linha de despesa de comitê publicada |
| `ReceitaComite` | 6.188 | `SG_UF`, `SG_PART`, `DS_ORGAO`, `DT_RECEITA`, `CD_CPF_CGC_DOA`, `SG_UF_DOADOR`, `NO_DOADOR`, `VR_RECEITA`, `TP_RECURSO` | uma linha de receita de comitê publicada |

Os conjuntos de comitê não têm `SEQUENCIAL_CANDIDATO`, e nenhum dos quatro tem identificador oficial de lançamento explícito. `CD_CPF_CGC` e `CD_CPF_CGC_DOA` são identificadores textuais, jamais números. `SG_UF`, `SG_PART`, `DS_ORGAO`, cargos, nomes, títulos e tipos permanecem texto até haver regra específica de normalização. Não inferir município a partir de UF ou nome.

## Chave e duplicatas

A chave técnica para preservar o grão publicado é (`manifesto bronze`, `parte bronze`, `ordinal da linha`), fornecida pelo [leitor de linhagem](../../crates/refino/src/linhagem.rs). Ela não é uma chave de negócio entre versões. Foram testadas combinações exploratórias de contexto, data, documento e valor; nenhuma identificou todas as linhas. `SELECT DISTINCT` foi aplicado às **colunas físicas completas**, sem hash e sem a procedência inferida pelo caminho.

| Tabela | Linhas distintas completas | Cópias exatas excedentes | Combinação exploratória distinta |
| --- | ---: | ---: | ---: |
| `DespesaCandidato` | 555.387 | 16.832 | 539.947 |
| `ReceitaCandidato` | 132.268 | 2.241 | 131.738 |
| `DespesaComite` | 35.223 | 2.814 | 34.411 |
| `ReceitaComite` | 6.111 | 77 | 6.078 |

As combinações exploratórias foram: para candidato, (`SEQUENCIAL_CANDIDATO`, `SG_UF`, `SG_PART`, data, `CD_CPF_CGC`, valor); para comitê, (`SG_UF`, `SG_PART`, `DS_ORGAO`, data, documento, valor). Elas colidem até entre linhas com atributos diferentes. `SEQUENCIAL_CANDIDATO` sozinho tem 3.893 valores distintos nas 572.219 despesas e 3.915 nas 134.509 receitas: identifica o candidato, não o lançamento. **Política candidata:** preservar todas as linhas, inclusive cópias exatas, e contar duplicatas como indicador de qualidade. Não deduplicar por documento, valor ou combinação exploratória sem evidência semântica adicional da origem.

## Tipos, nulos e qualidade

- `DT_DOC_DESP` e `DT_RECEITA` convertem por `DD/MM/AAAA` quando preenchidas. Há 75 datas vazias em `DespesaCandidato`, 22 em `ReceitaCandidato`, duas em `DespesaComite` e nenhuma em `ReceitaComite`; nenhuma data preenchida falhou nessa conversão. O contrato deve permitir `DATE` nulo, contar essas ausências e rejeitar formato não vazio inválido numa versão futura.
- `VR_DESPESA` e `VR_RECEITA` não estão vazios e todos converteram para `DECIMAL(18,2)` após trocar vírgula decimal por ponto. A regra deve rejeitar valor não vazio que não converta e não arredondar silenciosamente além de duas casas.
- `SEQUENCIAL_CANDIDATO` (nas duas tabelas de candidato), `SG_UF`, `SG_PART` e `DS_ORGAO` (nos comitês) não estão vazios no recorte. Ausência futura de um desses campos contextuais deve bloquear a publicação dessa tabela até revisão do contrato.
- Cada tabela deve ter contagens de entrada, saída, datas vazias, rejeições por motivo e duplicatas exatas. O limiar candidato é zero para erro de parse de data/valor e campo contextual obrigatório ausente. As datas realmente vazias são aceitas e medidas; nenhuma linha é removida apenas por ser duplicata.

Este contrato é **candidato para 2002**. A semântica de possíveis correções, os identificadores de lançamento e a relação com outras eleições não foram comprovados. Uma Silver canônica entre anos depende de validação de layouts adicionais; 2024 usa conjuntos e colunas diferentes.

## Consulta reproduzível

As consultas usaram DuckDB sobre as quatro chaves do manifesto vigente, com `read_parquet(..., hive_partitioning=false)`. Para cada parte foram calculados `COUNT(*)`, `COUNT(*)` sobre `SELECT DISTINCT *`, `COUNT(*)` sobre `SELECT DISTINCT` da combinação exploratória e contagens de `NULL`/texto vazio. Datas foram verificadas com `TRY_STRPTIME(campo, '%d/%m/%Y')`; valores, com `TRY_CAST(REPLACE(campo, ',', '.') AS DECIMAL(18,2))`. Nenhuma consulta retornou linhas individuais.
