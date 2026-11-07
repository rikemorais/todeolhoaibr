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

Em 2026-11-07, foram executados os contratos SQL da branch `feat/refino-sql-tse-243` sobre o Bronze local, publicando apenas em buckets de smoke. A coleta externa amostrou `/proc/<pid>/status` a cada 50 ms durante todo o processo; os smokes usaram concorrência 1, pool DataFusion padrão de 512 MiB e não impuseram cgroup ao processo local. O serviço `refino` do Docker tem limite de 2 GiB.

| Família | Recursos | Linhas publicadas | Tempo total aproximado | Pico RSS observado |
|---|---:|---:|---:|---:|
| Candidaturas | 17 (1994–2026) | 3.703.307 | 11min40s | 905.420 KiB (884 MiB) |
| Bens declarados | 11 (2006–2026) | 5.101.128 | 7min13s | 527.448 KiB (515 MiB) |

As duas consultas verificam duplicatas agrupando somente as colunas de chave, reduzem a contagem de grupos duplicados a um escalar e aplicam a asserção às linhas projetadas. Isso evitou a janela global sobre as linhas completas: o smoke anterior de candidaturas com `COUNT(*) OVER (...)` alcançou 4.074.896 KiB (3,89 GiB) mesmo com concorrência 1; a nova forma ficou abaixo do limite de 2 GiB. Os valores são picos amostrados e pertencem ao perfil local `dev`; a medição em Docker/cgroup e a coleta de RSS das demais famílias TSE continuam pendentes.
