# Camada bronze

O **bronze** é o primeiro estágio do refino ([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)): pega os bytes fiéis da camada `raw` e só os **decodifica e estrutura** em **Parquet**, mantendo correspondência **1:1 com a origem**, **sem regra de negócio** (isso é o silver). O conhecimento de formato que o [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md) mantém fora da ingestão mora aqui, no crate `refino`.

## O que faz (e o que não faz)

- **Faz:** descompacta (ZIP), reencoda para **UTF-8** (tenta UTF-8, cai para Windows-1252/Latin-1), parseia (CSV/JSON) e grava **Parquet** — todas as colunas como **texto**, nomeadas como na origem.
- **Não faz:** tipagem, deduplicação, chaves canônicas (`município + ano`), padronização entre fontes, cruzamentos — tudo isso é do **silver** em diante. Nunca reescreve o `raw`.

## Como funciona

- O comando `todeolho refinar bronze <fonte>` descobre no `raw` os recursos já ingeridos daquela fonte (os que têm `_ultima_versao.json`), monta uma **DAG de tarefas** — uma por recurso — e roda no **mesmo executor e painel** da ingestão.
- Cada tarefa lê a procedência do `raw` (manifesto vigente), decodifica cada parte e grava **um Parquet por tabela** no bucket `bronze`, com manifesto e ponteiro próprios.
- **Idempotência:** o ponteiro do bronze guarda o validador da **versão do raw**; reexecutar só reprocessa quando o `raw` mudou.

## Layout no bucket `bronze`

Espelha o `raw`, no bucket `bronze`:

```text
bronze/<fonte>/<recurso>/_ultima_versao.json
bronze/<fonte>/<recurso>/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.parquet
bronze/<fonte>/<recurso>/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

Um ZIP com N CSVs vira N Parquets, nomeados pelo CSV interno (registrado no manifesto).

## Como rodar

```bash
# Decodifica para bronze os recursos já ingeridos de uma fonte
docker compose run --rm refino refinar bronze camara-despesas
```

O serviço `refino` tem um `mem_limit` maior que o da ingestão porque decodificar CSV/JSON para colunar consome mais memória do que a ingestão, que vai em fluxo.

## Fontes cobertas

O catálogo de refino cresce por fonte. Hoje:

| Fonte (bronze) | Lê do raw | Formato de entrada |
| --- | --- | --- |
| `camara-despesas` | `camara/despesas/*` | ZIP + CSV (`;`) |

## Consultar o bronze

As tabelas Parquet são consultadas com **DuckDB** ([ADR 0018](../adr/0018-motor-de-consulta-duckdb.md)) — uma "Athena local": SQL sobre o Parquet **no lugar**, sem mover os dados. O serviço `consulta` sobe o DuckDB **somente-leitura** sobre os buckets e já cria uma **view por tabela**, nomeada `bronze_<fonte>_<recurso>` (ex.: `bronze_camara_despesas`).

As views são montadas a partir do **ponteiro de versão vigente** (`_ultima_versao.json`) de cada partição, então consultam só a versão atual — o histórico não entra em dobro. As colunas de partição (ex.: `ano`) e de procedência (`data_ingestao`, `execucao`) vêm do caminho.

```bash
# Interface gráfica (explorador + editor SQL) em http://localhost:4213
docker compose up -d --build consulta

# SQL no terminal, totalmente offline, com as views já carregadas
docker compose run --rm --build consulta sql
```

```sql
-- Quantas linhas por ano das despesas da Câmara
SELECT ano, count(*) AS linhas
FROM bronze_camara_despesas
GROUP BY ano ORDER BY ano;

-- Ler direto do versitygw (sem a view), via httpfs
SELECT count(*) FROM read_parquet('s3://bronze/camara/despesas/ano=2024/**/*.parquet');
```

!!! note "UI precisa de internet; o CLI não"
    A **UI** (`localhost:4213`) baixa o frontend de `ui.duckdb.org`, então precisa de internet no navegador; as **queries rodam 100% locais**. Sem internet, use `consulta sql`, que é offline. A UI faz *bind* em loopback dentro do container, e o serviço expõe a porta por uma ponte (`socat`).

## Limitações conhecidas

- O decodificador carrega o CSV inteiro em memória. Para arquivos anuais (dezenas de MB) é aceitável; arquivos gigantes (ex.: CNPJ) pedirão escrita por *row-group* em fluxo — ver [ADR 0007](../adr/0007-docker-para-maquinas-modestas.md).
