# 0018. Motor de consulta analítica: DuckDB sobre Parquet no versitygw

- **Status:** Aceito
- **Data:** 2026-11-03

## Contexto

O [ADR 0015](0015-arquitetura-de-dados-em-medalhao.md) definiu o medalhão (`raw` → `bronze` → `silver` → `gold` → `diamond`), e o `bronze` será gravado em **Parquet** (colunar) no versitygw ([ADR 0005](0005-armazenamento-s3-com-versitygw.md)). Precisamos de uma forma de **consultar essas tabelas com SQL de dentro do ambiente Docker** — uma espécie de "Athena local": SQL sobre arquivos Parquet no object storage, sem montar um data warehouse nem mover os dados para outra base.

Restrições do projeto: rodar em **máquinas modestas** ([ADR 0007](0007-docker-para-maquinas-modestas.md)), com `mem_limit` por contêiner, e consumir o Parquet **no lugar** (sem ETL para uma base separada, mantendo o Parquet como fonte).

## Decisão

Adotar o **DuckDB** como motor de consulta analítica sobre as camadas em Parquet.

- O DuckDB **lê o Parquet diretamente** — do sistema de arquivos (`/dados/bronze/...`) ou do próprio versitygw via a extensão `httpfs` (`SELECT * FROM 's3://bronze/<fonte>/.../*.parquet'`). É o modelo do Athena: SQL sobre arquivos no object storage, sem carregar numa base intermediária.
- É **in-process, binário único e de pegada mínima**, compatível com o [ADR 0007](0007-docker-para-maquinas-modestas.md) e com a cultura de `mem_limit`.
- A **exploração/depuração** das camadas (começando pelo `bronze`) é feita com o DuckDB — CLI, o web UI nativo (`duckdb -ui`) ou um contêiner enxuto apontando **somente-leitura** para os buckets.
- Uma **UI de BI** (ex.: Superset/Metabase) é tratada como **camada separada e opcional, por cima do DuckDB** (via `duckdb-engine`), e só quando houver dashboards para outras pessoas — idealmente consumindo `gold`/`diamond`, não o `bronze` cru. Não entra agora.

## Consequências

- Ganhamos "Athena local": SQL sobre o Parquet do medalhão sem infraestrutura de warehouse e sem duplicar dados.
- O `bronze` fica **consultável assim que existe**, mesmo antes de `silver`/`gold` — útil para validar a decodificação e explorar o dado.
- O DuckDB é ótimo para leitura analítica concorrente; não é um banco multiusuário transacional — o que é adequado, já que as camadas são **somente-leitura** para o consumo.
- A escolha da **UI de consumo** fica adiada e desacoplada do motor: qualquer front (Superset, Metabase, um painel próprio) fala com o DuckDB quando a hora chegar.
- Mantém a fidelidade do medalhão: o motor **não reescreve** as camadas, só as lê ([ADR 0015](0015-arquitetura-de-dados-em-medalhao.md)).

## Alternativas consideradas

- **Trino/Presto** (o OSS por trás do Athena): é o "Athena de verdade", mas exige coordinator + workers em JVM — peso de infraestrutura grande, contra o [ADR 0007](0007-docker-para-maquinas-modestas.md). Rejeitada por ora.
- **ClickHouse**: motor colunar excelente, mas o modelo natural é **carregar** os dados nele (e não consultar o Parquet no lugar), somando operação e storage duplicado. Overkill para explorar o `bronze`. Rejeitada por ora.
- **Superset/Metabase como "o SGBD":** são **UI de BI**, não motor de consulta — precisam de um engine por baixo. Entram, se entrarem, por cima do DuckDB. Não resolvem o problema sozinhos.
- **PostgreSQL + extensões (parquet_fdw/DuckDB FDW):** adiciona um servidor relacional para ler Parquet, mais pesado e com suporte a Parquet menos direto que o DuckDB nativo. Rejeitada.
