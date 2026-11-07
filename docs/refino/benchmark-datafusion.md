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
| Despesas pagas | 2024/BRASIL | 3.719.650 | 8min22s | 1.723.348 KiB (1,64 GiB) local; 2.097.152 KiB (2 GiB) cgroup |
| Votação por seção | 2024/AC + RO | 506.130 | 7,4s | 1.277.188 KiB (1,22 GiB) cgroup |

### Votação por seção, amostras 2024 por UF

Depois de trocar a janela global por uma agregação escalar compacta da chave e configurar pool de 256 MiB com spill de 8 GiB por recurso, estes smokes release foram repetidos em containers independentes com limite cgroup de 2 GiB. Os buckets eram descartáveis; nenhum ponteiro Silver vigente foi alterado.

| UF | Bytes Silver publicados | Duração | Pico cgroup |
|---|---:|---:|---:|
| AM | 46.979.533 | 10,9s | 288,6 MiB |
| BA | 180.520.001 | 35,0s | 700,4 MiB |
| CE | 103.169.274 | 19,3s | 441,1 MiB |
| MG | 337.143.798 | 57,1s | 1.040,5 MiB |
| PR | 166.748.858 | 31,6s | 720,7 MiB |
| RJ | 238.124.870 | 46,6s | 787,2 MiB |
| SP | 781.508.516 | 134,0s | 1,95 GiB |

Os smokes anteriores com a janela global falharam em estados grandes por OOM ou pelo teto de spill de 2 GiB. A primeira agregação escalar, ainda com oito colunas separadas, ultrapassou 8 GiB de spill em SP. Depois de compactar a chave com prefixos de comprimento, os sete estados acima concluíram sem OOM. SP é o caso medido mais próximo do limite operacional. O histórico completo de votação ocupa cerca de 16,4 GB no Bronze; estes recortes de 2024 não substituem a validação dos demais estados e anos.

As consultas de candidaturas e bens verificam duplicatas agrupando somente as colunas de chave, reduzem a contagem de grupos duplicados a um escalar e aplicam a asserção às linhas projetadas. Isso evitou a janela global sobre as linhas completas: o smoke anterior de candidaturas com `COUNT(*) OVER (...)` alcançou 4.074.896 KiB (3,89 GiB) mesmo com concorrência 1. Despesas pagas usa pool de 256 MiB e spill de 8 GiB; um teste anterior com pool de 1 GiB atingiu 2.213.732 KiB (2,11 GiB), então o limite do pool foi reduzido. No container limitado a 2 GiB, a tarefa concluiu com código 0 e sem OOM, mas o pico `memory.peak` atingiu o teto exato de 2.097.152 KiB. O executor local não aplica cgroup, por isso o RSS amostrado localmente com pool de 512 MiB foi 1.723.348 KiB. A amostra anterior AC/RO publicou 156.259 e 349.871 linhas, com pico cgroup abaixo de 1,3 GiB; os novos resultados de AM/BA/CE/MG/PR/RJ/SP estão detalhados acima.
