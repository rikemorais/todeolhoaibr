# Smoke e benchmark do executor SQL

O exemplo `crates/refino/examples/benchmark_sql_executor.rs` gera Parquet sintético em batches, executa SQL pelo DataFusion e transmite a saída para outro Parquet. Ele registra linhas e bytes de entrada/saída, tempo de consulta e escrita, RSS amostrado a cada 20 ms durante a consulta e a escrita, e pico de spill. A amostragem paralela reduz a chance de perder picos que ocorram entre batches de saída; ainda é uma observação amostrada, não o pico exato do kernel. Os dados são sintéticos; os números não representam uma medição de uma fonte TSE, BCB ou IBGE.

Reproduzir com 10 milhões de linhas e pool de memória DataFusion limitado a 64 MiB:

```bash
cargo run -p todeolho-refino --example benchmark_sql_executor --locked --offline -- 10000000 64
```

## Resultado registrado

Ambiente: Linux x86_64, Rust 1.99.1, 12 CPUs visíveis e 31 GiB de RAM no host. Execução no perfil `dev`, sem Docker nem limite de memória no processo. O pool interno do DataFusion foi limitado a 64 MiB; RSS é amostrado em `/proc/self/status` a cada 20 ms durante a consulta e a escrita, representando o maior valor observado, não um pico capturado pelo kernel.

A primeira compilação do exemplo em perfil `dev` levou 2 min 26 s com metadados de dependências já compilados pelo `cargo check` anterior. Não é uma medida de build totalmente frio nem do perfil `release`.

| Medida | Resultado |
|---|---:|
| Linhas Bronze de entrada | 10.000.000 |
| Parquet Bronze sintético | 41.876.889 bytes |
| Linhas Silver de saída | 10.000.000 |
| Parquet Silver de saída | 13.537.355 bytes |
| Pool de memória DataFusion | 67.108.864 bytes (64 MiB) |
| Maior RSS observado | 215.646.208 bytes (205,6 MiB) |
| Maior spill observado | 160.236.672 bytes (152,8 MiB) |
| SQL + escrita Parquet | 28.524 ms |

O RSS supera o pool porque ele mede o processo inteiro, incluindo runtime, buffers, Parquet e execução. O executor não substitui o limite de memória do contêiner; a validação operacional deve considerar ambos.

O exemplo usa `GROUP BY` sem `ORDER BY`, pois a ordem de saída não faz parte do contrato. No mesmo fixture, adicionar `ORDER BY id` com pool de 64 MiB falhou por falta de memória em `ExternalSorterMerge`, um operador que não aceita spill. SQLs que exigem ordenação global precisam de orçamento medido e continuam sujeitos a falhar sem publicar o ponteiro.

Este benchmark valida cardinalidade, streaming e spill em volume sintético de 10 milhões de linhas. A comparação de semântica, distribuição real, paridade e desempenho por fonte pertence aos issues de migração, começando pela #239.

## Smoke TSE com RSS observado

Em 2026-11-07, foram executados os contratos SQL da branch `feat/refino-sql-tse-243` sobre o Bronze local, publicando apenas em buckets de smoke. Os RSS locais da tabela abaixo foram amostrados em `/proc/<pid>/status` a cada 50 ms e usaram concorrência 1. As medições cgroup posteriores usaram containers independentes com limite de 2 GiB e buckets Silver descartáveis; o histórico das execuções foi separado do bucket consumido pela UI.

| Família | Recursos | Linhas publicadas | Tempo total aproximado | Pico RSS observado |
|---|---:|---:|---:|---:|
| Candidaturas | 17 (1994–2026) | 3.703.307 | 11min40s | 905.420 KiB (884 MiB) |
| Bens declarados | 11 (2006–2026) | 5.101.128 | 7min13s | 527.448 KiB (515 MiB) |
| Despesas pagas | 2024/BRASIL | 3.719.650 | 8min22s | 1.723.348 KiB (1,64 GiB) local; 2.097.152 KiB (2 GiB) cgroup |
| Votação por seção | 2024/AC + RO (amostra inicial) | 506.130 | 7,4s | 1.277.188 KiB (1,22 GiB) cgroup |

### Votação por seção, amostras 2024 por UF

Depois de trocar a janela global por uma agregação escalar compacta da chave e configurar pool de 256 MiB com spill de 8 GiB por recurso, os recursos de 2024 foram executados em containers release independentes com limite cgroup de 2 GiB. Foram processadas todas as 26 UFs com snapshot Bronze disponível; DF não gerou tarefa. Os buckets Silver eram descartáveis; nenhum ponteiro vigente foi alterado.

| UF | Bytes Silver publicados | Pico cgroup |
|---|---:|---:|
| AC | 7.466.288 | 185,8 MiB |
| AL | 25.428.387 | 214,0 MiB |
| AM | 46.979.533 | 288,6 MiB |
| AP | 6.803.135 | 185,9 MiB |
| BA | 180.520.001 | 700,4 MiB |
| CE | 103.169.274 | 441,1 MiB |
| ES | 47.210.626 | 274,7 MiB |
| GO | 94.940.153 | 455,9 MiB |
| MA | 76.666.108 | 409,9 MiB |
| MG | 337.143.798 | 1.040,5 MiB |
| MS | 32.196.026 | 278,7 MiB |
| MT | 37.200.562 | 253,6 MiB |
| PA | 107.838.336 | 442,7 MiB |
| PB | 40.789.014 | 255,5 MiB |
| PE | 99.748.010 | 434,8 MiB |
| PI | 34.573.078 | 244,8 MiB |
| PR | 166.748.858 | 720,7 MiB |
| RJ | 238.124.870 | 787,2 MiB |
| RN | 31.389.297 | 286,4 MiB |
| RO | 17.797.342 | 207,8 MiB |
| RR | 4.948.697 | 165,6 MiB |
| RS | 148.022.084 | 632,7 MiB |
| SC | 81.127.565 | 425,1 MiB |
| SE | 22.728.906 | 233,5 MiB |
| SP | 781.508.516 | 1,99 GiB |
| TO | 14.575.591 | 223,9 MiB |

Os smokes anteriores com a janela global falharam em estados grandes por OOM ou pelo teto de spill de 2 GiB. A primeira agregação escalar, ainda com oito colunas separadas, ultrapassou 8 GiB de spill em SP. Depois de compactar a chave com prefixos de comprimento, as 26 UFs disponíveis concluíram sem OOM. SP é o caso medido mais próximo do limite operacional. Um smoke adicional de SP (136,5 s, pico 1,99 GiB) gravou o histórico no bucket normal `execucoes`; a API da UI confirmou a execução `20261107T212424Z-23c97421` como sucesso. O Parquet dessa execução continuou em bucket Silver descartável.

### Votação por seção, amostras históricas

Para cobrir layouts de eleições antigas, SP/2002 e SP/2010 foram processados com a SQL atualizada, concorrência 1 e limite de memória do container em 2 GiB. Cada execução publicou a saída em um bucket Silver descartável; nenhum ponteiro vigente foi alterado. `docker stats` foi consultado repetidamente durante cada execução; o valor abaixo é o máximo observado, não o pico exato do kernel.

| Ano/UF | Bytes Silver publicados | Tempo | Pico `docker stats` (amostras) | OOM | Execução no painel |
|---|---:|---:|---:|---|---|
| 2002/SP | 169.127.314 | 94,2 s | 344,5 MiB (25) | não | `20261108T002244Z-951b9d42` |
| 2010/SP | 212.938.583 | 119,1 s | 380,0 MiB (52) | não | `20261108T003147Z-b9b7d108` |
| 2012/SP | 126.380.036 | 77,8 s | 335,7 MiB (33) | não | `20261108T003608Z-0f77f1ba` |

As execuções foram confirmadas como sucesso pela API do painel. Para reproduzir os smokes, use um bucket descartável por ano:

```bash
TODEOLHO_S3_BUCKET_SILVER=silver-smoke-hist-2002-sp-rss \
  docker compose run --rm --build refino refinar silver tse-votacao-secao \
  --filtro ano=2002 --filtro uf=SP

TODEOLHO_S3_BUCKET_SILVER=silver-smoke-hist-2010-sp-rss \
  docker compose run --rm --build refino refinar silver tse-votacao-secao \
  --filtro ano=2010 --filtro uf=SP

TODEOLHO_S3_BUCKET_SILVER=silver-smoke-hist-2012-sp \
  docker compose run --rm --build refino refinar silver tse-votacao-secao \
  --filtro ano=2012 --filtro uf=SP
```

A cobertura medida agora inclui 2024 (26 UFs com snapshot Bronze), SP/2002, SP/2010 e SP/2012. O histórico completo de votação ocupa cerca de 16,4 GB no Bronze; os outros anos ainda aguardam smoke com o plano atualizado.

As consultas de candidaturas e bens verificam duplicatas agrupando somente as colunas de chave, reduzem a contagem de grupos duplicados a um escalar e aplicam a asserção às linhas projetadas. Isso evitou a janela global sobre as linhas completas: o smoke anterior de candidaturas com `COUNT(*) OVER (...)` alcançou 4.074.896 KiB (3,89 GiB) mesmo com concorrência 1. Despesas pagas usa pool de 256 MiB e spill de 8 GiB; um teste anterior com pool de 1 GiB atingiu 2.213.732 KiB (2,11 GiB), então o limite do pool foi reduzido. No container limitado a 2 GiB, a tarefa concluiu com código 0 e sem OOM, mas o pico `memory.peak` atingiu o teto exato de 2.097.152 KiB. O executor local não aplica cgroup, por isso o RSS amostrado localmente com pool de 512 MiB foi 1.723.348 KiB. A amostra anterior AC/RO publicou 156.259 e 349.871 linhas, com pico cgroup abaixo de 1,3 GiB; os novos resultados de AM/BA/CE/MG/PR/RJ/SP estão detalhados acima.
