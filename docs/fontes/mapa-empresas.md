# Fonte `mapa_empresas` — estatísticas do registro empresarial

Snapshots mensais do Mapa de Empresas, mantido pelo Ministério do Empreendedorismo, da
Microempresa e da Empresa de Pequeno Porte (MEMP). As planilhas abertas trazem contagens
de empresas abertas, fechadas e ativas por município e por UF. O saldo pode ser derivado
no refino como `empresas_abertas - empresas_fechadas`.

## Arquivos ingeridos

Os arquivos são preservados sem alteração na Raw. A lista abaixo reflete os XLSX que o
portal oficial disponibiliza e que foram verificados em 14/11/2026:

| Período de referência | Arquivo oficial |
| --- | --- |
| 2026-05 | [ArquivoDados maio/2026 v2](https://www.gov.br/memp/pt-br/acesso-a-informacao/dados-abertos/06_arquivodados_mai26-mapa-de-empresas-v2.xlsx) |
| 2026-06 | [ArquivoDados junho/2026](https://www.gov.br/memp/pt-br/acesso-a-informacao/dados-abertos/06_arquivodados_jun26-mapa-de-empresas.xlsx) |
| 2026-07 | [ArquivoDados julho/2026](https://www.gov.br/memp/pt-br/acesso-a-informacao/dados-abertos/07_arquivodados_jul26-mapa-de-empresas.xlsx) |
| 2026-08 | [ArquivoDados agosto/2026](https://www.gov.br/memp/pt-br/acesso-a-informacao/dados-abertos/08_arquivodados_ago26-mapa-de-empresas.xlsx) |

Os XLSX verificados têm as abas `Abertas, Fechadas e Ativas`, `Inova Simples`,
`Empresas Ativas por NJ e Porte` e `Gênero (UF, NJ e Porte)`. Na primeira aba, o MEMP
inclui os blocos geográficos municipal e estadual; os dados estaduais já estão agregados
por UF. Cada snapshot é uma fotografia do mês, não uma observação quadrimestral. O
catálogo precisa ser atualizado quando novos snapshots forem publicados; não são criadas
tarefas para URLs ainda indisponíveis.

## Layout na Raw e filtros

```text
raw/mapa_empresas/movimentacao/periodo=AAAA-MM/_ultima_versao.json
raw/mapa_empresas/movimentacao/periodo=AAAA-MM/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.xlsx
```

Há uma tarefa idempotente por mês, identificada pelo período de referência. Para limitar
a ingestão, use `periodo`:

```bash
docker compose run --rm ingestao executar mapa_empresas --filtro periodo=2026-08
docker compose run --rm ingestao executar mapa_empresas --filtro periodo=2026-06,2026-08
```

Cada planilha contém todas as UFs; por isso não há uma cópia ou manifesto separado por
estado. Para materializar a tabela estadual no Bronze sem modificar o XLSX da Raw, rode:

```bash
docker compose run --rm refino refinar bronze mapa_empresas
```

O decodificador seleciona o bloco estadual da aba `Abertas, Fechadas e Ativas`, valida
que as 27 UFs estão presentes uma vez cada e publica `bronze_mapa_empresas_movimentacao`.
O caminho acrescenta a coluna de partição `periodo`, que pode ser combinada com a UF:

```sql
SELECT
  periodo,
  "UF",
  "Estado",
  CAST("Empresas Abertas" AS BIGINT) AS abertas,
  CAST("Empresas Fechadas" AS BIGINT) AS fechadas,
  CAST("Empresas Abertas" AS BIGINT) - CAST("Empresas Fechadas" AS BIGINT) AS saldo,
  CAST("Empresas Ativas" AS BIGINT) AS ativas
FROM bronze_mapa_empresas_movimentacao
WHERE periodo = '2026-08' AND "UF" = 'MG';
```

Para publicar a camada Silver tipada, rode:

```bash
docker compose run --rm refino refinar silver mapa_empresas
```

A Silver mantém uma linha por mês e UF, converte as datas seriais do Excel para `DATE`,
as contagens para `BIGINT` e calcula `saldo = empresas_abertas - empresas_fechadas`.
Ela valida período, campos obrigatórios, contagens não negativas e a chave (`periodo`,
`uf`) antes de publicar. No DuckDB, a view é `silver_mapa_empresas_movimentacao`:

```sql
SELECT periodo, uf, estado, empresas_abertas, empresas_fechadas, saldo, empresas_ativas
FROM silver_mapa_empresas_movimentacao
WHERE periodo = '2026-08' AND uf = 'MG';
```

Isso preserva o arquivo-fonte e evita duplicá-lo 27 vezes. Os arquivos quadrimestrais do
Mapa continuam sendo PDFs analíticos, não tabelas completas por UF; a pesquisa da origem
e as limitações estão em [pesquisa do Mapa de Empresas](pesquisa-mapa-empresas.md).

## Atualização da fonte

Quando o MEMP publicar uma planilha mensal, acrescente o período e o nome do anexo em
`SNAPSHOTS_PUBLICADOS` em `crates/fontes/src/fontes/mapa_empresas.rs`, atualize a tabela
acima e confirme que o link `/@@download/file` retorna um XLSX. Correções da origem para
um mês já ingerido mantêm o mesmo recurso; o ETag do arquivo permite que a execução
detecte e publique uma versão nova.
