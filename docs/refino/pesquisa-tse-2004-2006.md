# Contratos candidatos — prestação de contas TSE 2004 e 2006

**Escopo:** partes dos manifestos Bronze vigentes de `tse/prestacao-contas/ano=2004` e `ano=2006`, consultadas em 2026-11-05. As consultas retornaram apenas schemas e contagens agregadas; nenhum registro, nome, documento ou valor individual foi exibido. Ambos os manifestos consultados têm `execucao=20261104T220445Z-f211faa3`.

## Separação dos conjuntos e mudanças de layout

Cada ano tem quatro partes: despesas e receitas de candidatos e de comitês. Em 2004, os nomes das partes de comitê são `DespesaComitê` e `ReceitaComitê`; em 2006 são iguais. Os conjuntos de candidatos mantêm `DespesaCandidato` e `ReceitaCandidato`, mas as **colunas mudam**. A tabela enumera os campos que permitem comparar datas e valores, não um mapeamento semântico completo.

| Ano | Parte | Nome no manifesto | Campo de data | Campo de valor | Contexto disponível no schema |
| --- | ---: | --- | --- | --- | --- |
| 2004 | 1 | `DespesaCandidato` | `DT_DOC_DESP` | `VR_DESPESA` | `NO_CAND`, `NR_CAND`, `SG_UE`, `SG_PART`, `NR_DOC_DESP` |
| 2004 | 2 | `ReceitaCandidato` | `DT_RECEITA` | `VR_RECEITA` | `NO_CAND`, `NR_CAND`, `SG_UE`, `SG_PART`, `NO_DOADOR` |
| 2004 | 3 | `DespesaComitê` | `DT_DOC_DESP` | `VR_DESPESA` | `DS_ORGAO`, `SG_UE`, `SG_PART`, `NR_DOC_DESP` |
| 2004 | 4 | `ReceitaComitê` | `DT_RECEITA` | `VR_RECEITA` | `DS_ORGAO`, `SG_UE`, `SG_PART`, `NO_DOADOR` |
| 2006 | 1 | `DespesaCandidato` | `DATA_DESPESA` | `VALOR_DESPESA` | `SEQUENCIAL_CANDIDATO`, `NUMERO_CANDIDATO`, `UNIDADE_ELEITORAL_CANDIDATO`, `NUMERO_DOCUMENTO` |
| 2006 | 2 | `ReceitaCandidato` | `DATA_RECEITA` | `VALOR_RECEITA` | `SEQUENCIAL_CANDIDATO`, `NUMERO_CANDIDATO`, `UNIDADE_ELEITORAL_CANDIDATO`, `NOME_DOADOR` |
| 2006 | 3 | `DespesaComitê` | `DT_DOC_DESP` | `VR_DESPESA` | `DS_ORGAO`, `SG_UE`, `SG_PART`, `NR_DOC_DESP` |
| 2006 | 4 | `ReceitaComitê` | `DT_RECEITA` | `VR_RECEITA` | `DS_ORGAO`, `SG_UE`, `SG_PART`, `NO_DOADOR` |

O `SEQUENCIAL_CANDIDATO` presente nos conjuntos de candidatos de [2002](pesquisa-tse-2002.md) **não existe nos de 2004**, mas retorna em 2006. Em 2004, há cabeçalhos literais que parecem expressões de SQL da exportação, como `RTRIM(LTRIM(DR.DS_TITULO))`; não se deve renomeá-los com base só na aparência. Em 2006, as colunas dos candidatos usam nomes por extenso (`VALOR_DESPESA`, `DATA_DESPESA`), enquanto os comitês ainda usam `VR_*` e `DT_*`. O campo de situação cadastral também aparece com grafias diferentes nos dois grupos. Nenhum desses fatos prova que colunas parecidas tenham semântica idêntica entre anos.

## Volumes, duplicatas e conversão

As linhas distintas são `SELECT DISTINCT *` sobre as colunas **físicas** de cada Parquet, com `hive_partitioning=false`. Cópias exatas excedentes são a diferença entre linhas e linhas distintas. Não foi testada uma chave de negócio para 2004 ou 2006.

| Ano | Tabela | Linhas | Linhas distintas | Cópias exatas excedentes | Data vazia | Valor vazio |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 2004 | `DespesaCandidato` | 1.799.942 | 1.777.229 | 22.713 | 174 | 0 |
| 2004 | `ReceitaCandidato` | 736.836 | 729.121 | 7.715 | 130 | 0 |
| 2004 | `DespesaComitê` | 347.516 | 342.937 | 4.579 | 135 | 0 |
| 2004 | `ReceitaComitê` | 95.986 | 94.354 | 1.632 | 7 | 0 |
| 2006 | `DespesaCandidato` | 1.096.926 | 1.046.887 | 50.039 | 0 | 0 |
| 2006 | `ReceitaCandidato` | 265.579 | 259.531 | 6.048 | 26 | 556 |
| 2006 | `DespesaComitê` | 82.507 | 81.901 | 606 | 0 | 0 |
| 2006 | `ReceitaComitê` | 11.141 | 10.658 | 483 | 0 | 18 |

Todas as datas **preenchidas** dos oito conjuntos converteram por `DD/MM/AAAA` com `TRY_STRPTIME`; todos os valores **não vazios** converteram para `DECIMAL(18,2)` após trocar vírgula por ponto. A conversão de teste não prova que cada valor tenha no máximo duas casas decimais, pois um cast pode arredondar. A implementação deverá conferir a escala antes de gravar um decimal.

## Política candidata para a Silver

1. Criar contrato separado por **ano e conjunto**. O grão preservado é uma linha publicada; a referência técnica (`manifesto`, `parte`, `ordinal`) distingue todas as linhas dentro da versão Bronze. Ela não substitui uma chave de negócio entre versões.
2. Preservar as cópias exatas até confirmar com a documentação de cada eleição se são repetições semânticas ou registros intencionais. Contá-las por tabela e versão. Não unir candidato e comitê nem reaproveitar a chave candidata de 2024.
3. Tratar identificadores e documentos como texto. Datas vazias viram `DATE` nulo com contagem; valores vazios de 2006 viram `DECIMAL` nulo com contagem. Campo não vazio que falhe no parse, ou valor que exigiria arredondamento, deve bloquear a primeira publicação até revisão. Município não deve ser inferido de `SG_UE` ou nomes.
4. Mapear cabeçalhos de 2004 que contêm expressões e diferenças de vocabulário de 2006 somente com leiaute específico ou validação equivalente da origem. Até lá, esses campos ficam textuais e o contrato Silver tipado permanece candidato, sem publicação canônica.

## Método

Foram selecionadas as quatro chaves de cada manifesto vigente e lidas com DuckDB em `read_parquet(caminho, hive_partitioning=false)`. As contagens usam `COUNT(*)`, `SELECT DISTINCT *`, testes de `NULL`/texto vazio, `TRY_STRPTIME(data, '%d/%m/%Y')` e `TRY_CAST(REPLACE(valor, ',', '.') AS DECIMAL(18,2))`. As consultas de resultado foram limitadas a uma linha agregada por parte. Os números valem para essas versões locais e precisam ser revalidados após nova publicação do Bronze.
