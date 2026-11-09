# Prestação de contas TSE 2014–2022: inventário Bronze

Este inventário lê somente os manifestos apontados por `_ultima_versao.json` no
Bronze local em 2026-11-09. Conta partes Parquet por nome de tabela interna;
não valida linhas, chaves, tipos nem equivalência entre partes por UF e
consolidados. Portanto, ainda não autoriza publicação Silver desses anos.

Todos os manifestos abaixo estão sob
`tse/prestacao-contas/ano=AAAA/data_ingestao=2026-11-05/execucao=20261105T041222Z-8947609b/_manifesto.json`,
substituindo `AAAA` pelo ano da primeira coluna. Essa chave fixa identifica o
snapshot contado mesmo quando `_ultima_versao.json` mudar.

| Ano | Partes | Tabelas internas e número de partes por prefixo |
| --- | ---: | --- |
| 2014 | 178 | `despesas_candidatos_2014`, `despesas_comites_2014`, `despesas_partidos_2014`, `receitas_candidatos_2014`, `receitas_comites_2014`, `receitas_partidos_2014`: **29 cada**; `despesas_candidatos_prestacao_contas_final_2014`, `despesas_partidos_prestacao_contas_final_2014`, `receitas_candidatos_prestacao_contas_final_2014`, `receitas_partidos_prestacao_contas_final_2014`: **1 `_sup` cada**. Total: 6 × 29 + 4 = 178. |
| 2016 | 112 | `despesas_candidatos_prestacao_contas_final_2016`, `despesas_partidos_prestacao_contas_final_2016`, `receitas_candidatos_prestacao_contas_final_2016`, `receitas_partidos_prestacao_contas_final_2016`: **28 cada** (26 UFs, `_brasil`, `_sup`). |
| 2018 | 116 | `despesas_contratadas_candidatos_2018`, `despesas_pagas_candidatos_2018`, `receitas_candidatos_2018`, `receitas_candidatos_doador_originario_2018`: **29 cada** (27 UFs, `_BR`, `_BRASIL`). |
| 2020 | 108 | `despesas_contratadas_candidatos_2020`, `despesas_pagas_candidatos_2020`, `receitas_candidatos_2020`, `receitas_candidatos_doador_originario_2020`: **27 cada** (26 UFs, `_BRASIL`). |
| 2022 | 116 | `despesas_contratadas_candidatos_2022`, `despesas_pagas_candidatos_2022`, `receitas_candidatos_2022`, `receitas_candidatos_doador_originario_2022`: **29 cada** (27 UFs, `_BR`, `_BRASIL`). |

O agrupamento remove somente o sufixo da parte (`_UF`, `_BR`, `_BRASIL`,
`_brasil` ou `_sup`); os prefixos acima são os nomes das tabelas internas
observadas no campo `url` do manifesto. Em 2014, as seis famílias principais
têm 27 UFs, `_BR` e `_brasil`; quatro arquivos `_sup` adicionais compõem o total.
Em 2018 e 2022 há 27 UFs, `_BR` e `_BRASIL` por família; `_BR` não aparece
em 2020.

## Recorte validado: 2014

Li os metadados dos 178 arquivos Parquet do manifesto fixado. Em cada uma das
seis famílias principais, as 29 partes (27 UFs, `_BR` e `_brasil`) têm schema
idêntico entre si. A tabela registra as linhas somadas nessas 29 partes; isso
não prova que `_BR` e `_brasil` sejam complementares ou intercambiáveis.

| Família | Partes | Schemas distintos | Linhas somadas |
| --- | ---: | ---: | ---: |
| `despesas_candidatos_2014` | 29 | 1 | 4.494.216 |
| `despesas_comites_2014` | 29 | 1 | 109.940 |
| `despesas_partidos_2014` | 29 | 1 | 110.029 |
| `receitas_candidatos_2014` | 29 | 1 | 854.978 |
| `receitas_comites_2014` | 29 | 1 | 18.346 |
| `receitas_partidos_2014` | 29 | 1 | 24.240 |

As quatro tabelas `*_prestacao_contas_final_2014` têm uma parte `_sup` cada,
schemas próprios (distintos das famílias principais) e, respectivamente,
4.526, 10, 659 e 3 linhas. São conjuntos pequenos, mas devem permanecer
separados até identificar sua relação semântica com as prestações ordinárias.
Todos os campos físicos observados são `Utf8` anuláveis no Parquet Bronze;
tipagem Silver exige contrato explícito por família.

### Comparação de conteúdo entre `_BR`, `_brasil` e UFs

Comparei todas as linhas por SHA-256 da serialização JSON dos campos físicos,
sem gravar ou exibir valores individuais. Nas seis famílias, todos os hashes
distintos de `_BR` e das UFs aparecem também em `_brasil`, e `_BR` não compartilha
hashes com as UFs. Porém, inclusão de hashes não garante a mesma multiplicidade
de linhas; por isso comparei também a contagem de cada hash nos dois recortes.
As 27 partes estaduais por família foram checadas contra a coluna `UF`: nenhuma
linha divergiu do sufixo de origem.

| Família | Linhas em `_brasil` | Hashes distintos | Linhas repetidas idênticas em `_brasil` |
| --- | ---: | ---: | ---: |
| `despesas_candidatos_2014` | 2.247.108 | 2.124.540 | 122.568 |
| `despesas_comites_2014` | 54.970 | 49.341 | 5.629 |
| `despesas_partidos_2014` | 55.370 | 50.086 | 5.284 |
| `receitas_candidatos_2014` | 427.489 | 427.212 | 277 |
| `receitas_comites_2014` | 9.173 | 9.115 | 58 |
| `receitas_partidos_2014` | 12.247 | 12.227 | 20 |

| Família | Linhas em `_brasil` | Linhas em `_BR` + UFs | Hashes com multiplicidade divergente |
| --- | ---: | ---: | ---: |
| `despesas_candidatos_2014` | 2.247.108 | 2.247.108 | 0 |
| `despesas_comites_2014` | 54.970 | 54.970 | 0 |
| `despesas_partidos_2014` | 55.370 | 54.659 | 551 |
| `receitas_candidatos_2014` | 427.489 | 427.489 | 0 |
| `receitas_comites_2014` | 9.173 | 9.173 | 0 |
| `receitas_partidos_2014` | 12.247 | 11.993 | 254 |

Quatro famílias têm multiplicidade idêntica por hash em `_brasil` e na soma de
`_BR` com as UFs. Em `despesas_partidos_2014`, `_brasil` tem 551 hashes que não
aparecem nos outros recortes e 711 linhas a mais; em `receitas_partidos_2014`,
tem 254 hashes adicionais e 254 linhas a mais. Portanto, escolher `_brasil`
como fonte única descartaria a possibilidade de reconciliar esses registros
adicionais. Também há repetições exatas dentro dos próprios arquivos `_brasil`,
sobretudo nas despesas. Não as removi nem inferi uma chave de lançamento: falta
determinar se representam cópias da origem ou lançamentos distintos. A
evidência é limitada às seis famílias ordinárias de 2014; não define o
tratamento dos conjuntos `_sup`.

Os nomes `_BR`, `_BRASIL`, `_brasil` e `_sup` são distintos no manifesto. Não
devem ser somados nem tratados como cópias sem comparar schema, conteúdo e
procedência. O contrato Silver precisa decidir por família e ano qual conjunto
publicar, quais registros são complementares ou retificações, e como medir
conflitos e duplicatas. A regra de 2024 não pode ser aplicada a esses anos
somente pela semelhança dos nomes.

### Chaves compostas preliminares

O TSE informa que a estrutura pode variar por eleição e orienta consultar o
`leia-me.pdf` de cada pacote ([grupo oficial de prestação de contas](https://dadosabertos.tse.jus.br/group/about/prestacao-de-contas-eleitorais)).
O `LEIAME.pdf` do snapshot consultado descreve `#NULO`/`#NE` e os sentinelas
numéricos `-1`/`-3`, mas não define chave de lançamento para estas seis famílias.
Por isso, as combinações abaixo são sondas de unicidade, não contratos aceitos:

- Despesas: código da eleição, CNPJ do prestador, sequência do candidato/comitê/diretório, tipo e número do documento.
- Receitas de candidatos: código da eleição, CNPJ do prestador, sequência do candidato, número do recibo eleitoral e número do documento.
- Receitas de comitês e partidos: código da eleição, CNPJ do prestador, sequência do comitê/diretório, tipo e número do documento.

Medi as partes do manifesto Bronze fixado em 2026-11-05: `_BR` mais 27 UFs
(28 partes por família) e `_brasil` separadamente (uma parte). Campos vazios,
`#NULO`, `#NE`, `-1` e `-3` foram tratados como incompletos, sem imprimir linhas
ou valores pessoais. `Grupos repetidos` conta chaves completas com mais de uma
linha; as linhas incompletas não entram nessa contagem.

| Família | Recorte | Linhas | Chaves incompletas | Chaves completas distintas | Grupos repetidos | Linhas em grupos repetidos |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `despesas_candidatos_2014` | `_BR` + UFs | 2.247.108 | 365.944 | 982.230 | 125.254 | 1.024.188 |
| `despesas_candidatos_2014` | `_brasil` | 2.247.108 | 365.944 | 982.230 | 125.254 | 1.024.188 |
| `despesas_comites_2014` | `_BR` + UFs | 54.970 | 8.393 | 26.741 | 3.807 | 23.643 |
| `despesas_comites_2014` | `_brasil` | 54.970 | 8.393 | 26.741 | 3.807 | 23.643 |
| `despesas_partidos_2014` | `_BR` + UFs | 54.659 | 22.479 | 17.654 | 2.274 | 16.800 |
| `despesas_partidos_2014` | `_brasil` | 55.370 | 22.993 | 17.764 | 2.306 | 16.919 |
| `receitas_candidatos_2014` | `_BR` + UFs | 427.489 | 266.497 | 160.992 | 0 | 0 |
| `receitas_candidatos_2014` | `_brasil` | 427.489 | 266.497 | 160.992 | 0 | 0 |
| `receitas_comites_2014` | `_BR` + UFs | 9.173 | 1.937 | 7.236 | 0 | 0 |
| `receitas_comites_2014` | `_brasil` | 9.173 | 1.937 | 7.236 | 0 | 0 |
| `receitas_partidos_2014` | `_BR` + UFs | 11.993 | 2.813 | 9.180 | 0 | 0 |
| `receitas_partidos_2014` | `_brasil` | 12.247 | 3.067 | 9.180 | 0 | 0 |

Nenhuma combinação cobre todas as linhas com uma chave única. As três famílias
de despesas têm colisões mesmo entre as linhas cujos campos estão completos.
A combinação de recibo e número do documento é única nas 160.992 linhas
completas de receitas de candidatos, mas outras 266.497 não têm todos esses
campos. Usar apenas `Numero Recibo Eleitoral` deixa 2.958 linhas incompletas e
4.342 grupos repetidos (15.926 linhas nesses grupos); acrescentar o número do
documento elimina as colisões no subconjunto completo, mas não resolve a falta
de identidade nas outras linhas. Receitas de comitês e partidos não colidem
entre as linhas completas, mas têm linhas sem todos os campos da combinação. Em
`receitas_partidos_2014`, `_brasil` tem 254 linhas a mais e também 254 chaves
incompletas a mais que `_BR` + UFs; esta agregação não prova que sejam as mesmas
linhas que diferem na comparação por hash. Não removi nem agrupei registros.
Esses resultados rejeitam as combinações testadas para uso como chave única;
uma identidade de negócio ainda precisa de evidência específica da origem.

### Recortes validados: 2016–2022

Validei o snapshot Bronze fixado em 2026-11-05 para as quatro famílias de cada
ano. Cada família tem um único schema entre os Parquets do manifesto: 2016 tem
26 UFs, `_brasil` e `_sup`; 2018 tem 27 UFs, `_BR` e `_BRASIL`; 2020 tem 26
UFs e `_BRASIL`; 2022 tem 27 UFs, `_BR` e `_BRASIL`. O campo `UF` não foi
normalizado nem excluído do hash.

Para comparar conteúdo, calculei SHA-256 de `to_json(linha)` e comparei a
multiplicidade de cada hash. Assim, zero grupos divergentes significa que os
dois recortes contêm exatamente as mesmas linhas, inclusive suas repetições.
`Repetidas` conta ocorrências além da primeira por hash dentro de cada recorte;
nenhuma foi removida. Os valores são agregados e não incluem linhas ou dados
pessoais.

| Ano / família | Linhas UFs | Linhas `_BR` | Linhas `_BRASIL` / `_brasil` | Repetidas UFs / `_BR` / nacional | Grupos hash divergentes | Ocorrências divergentes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 2016 `despesas_candidatos_prestacao_contas_final` | 5.588.616 | — | 5.588.616 | 63.612 / — / 63.612 | 0 | 0 |
| 2016 `despesas_partidos_prestacao_contas_final` | 275.916 | — | 282.727 | 14.252 / — / 14.404 | 6.659 | 6.811 |
| 2016 `receitas_candidatos_prestacao_contas_final` | 3.004.707 | — | 3.004.707 | 3.317 / — / 3.317 | 0 | 0 |
| 2016 `receitas_partidos_prestacao_contas_final` | 139.230 | — | 140.496 | 1.172 / — / 1.172 | 1.266 | 1.266 |
| 2018 `despesas_contratadas_candidatos` | 1.716.410 | 7.643 | 1.724.053 | 25.809 / 83 / 25.892 | 0 | 0 |
| 2018 `despesas_pagas_candidatos` | 1.644.740 | 8.635 | 1.653.375 | 0 / 0 / 0 | 0 | 0 |
| 2018 `receitas_candidatos` | 328.657 | 1.026 | 329.683 | 281 / 1 / 282 | 0 | 0 |
| 2018 `receitas_candidatos_doador_originario` | 86.319 | 65.129 | 151.448 | 1.063 / 346 / 1.409 | 0 | 0 |
| 2020 `despesas_contratadas_candidatos` | 4.107.531 | — | 4.107.531 | 55.321 / — / 55.321 | 0 | 0 |
| 2020 `despesas_pagas_candidatos` | 3.086.924 | — | 3.086.924 | 0 / — / 0 | 0 | 0 |
| 2020 `receitas_candidatos` | 2.339.073 | — | 2.339.073 | 1.432 / — / 1.432 | 0 | 0 |
| 2020 `receitas_candidatos_doador_originario` | 662.577 | — | 662.577 | 1.084 / — / 1.084 | 0 | 0 |
| 2022 `despesas_contratadas_candidatos` | 2.200.603 | 9.232 | 2.209.835 | 42.455 / 596 / 43.051 | 0 | 0 |
| 2022 `despesas_pagas_candidatos` | 2.404.915 | 7.048 | 2.411.963 | 0 / 0 / 0 | 0 | 0 |
| 2022 `receitas_candidatos` | 281.928 | 393.016 | 674.944 | 248 / 53 / 301 | 0 | 0 |
| 2022 `receitas_candidatos_doador_originario` | 115.737 | 83.504 | 199.241 | 2.137 / 152 / 2.289 | 0 | 0 |

Em 2018 e 2022, `_BRASIL` equivale exatamente à soma de `_BR` e UFs nas quatro
famílias. Em 2020, `_BRASIL` equivale exatamente às UFs nas quatro famílias.
Em 2016, `_brasil` equivale às UFs para despesas e receitas de candidatos. Para
despesas de partidos, `_brasil` contém 6.811 ocorrências de hash adicionais;
para receitas de partidos, contém 1.266. Não encontrei hashes presentes apenas
nas UFs nesses dois recortes nacionais. Preservei os registros divergentes.

#### Cobertura dos lançamentos partidários de 2016

Agrupei as duas famílias de partidos pelo campo físico `UF`, comparando o
consolidado `_brasil` com a soma das 26 partes por UF do mesmo manifesto. Todas
as UFs presentes nas partes têm a mesma contagem no consolidado. A diferença
inteira está em `BR` e `DF`, que não têm parte territorial própria nesse
snapshot:

| Família | `UF=BR` só em `_brasil` | `UF=DF` só em `_brasil` | Total adicional |
| --- | ---: | ---: | ---: |
| `despesas_partidos_prestacao_contas_final_2016` | 6.716 | 95 | 6.811 |
| `receitas_partidos_prestacao_contas_final_2016` | 1.215 | 51 | 1.266 |

Todas as linhas `BR` dessas famílias têm `Tipo diretorio = Direção Nacional`;
as `DF` têm `Tipo diretorio = Direção Estadual/Distrital`. A comparação anterior
por hash e multiplicidade confirma que o consolidado contém todas as linhas das
partes por UF e que somente essas ocorrências adicionais diferem. Portanto,
**para este snapshot**, `_brasil` é o recorte completo das quatro famílias de
2016 e as partes por UF são subconjuntos; somá-los duplicaria lançamentos. Essa
observação não define uma chave de negócio, a semântica de `_sup` nem uma regra
automática para outros anos ou versões do recurso.

As quatro partes `_sup` de 2016 têm zero linhas neste snapshot Bronze. Isso não
altera a conclusão da [pesquisa oficial sobre `_sup`](pesquisa-tse-2014-sup.md):
o TSE publica esse recurso separadamente, mas não documenta sua relação linha a
linha com a prestação regular. Zero linhas no snapshot local não demonstra que
o pacote oficial esteja vazio nem define a semântica do recurso.

### Próximas validações

Em 2014, investigar as variantes e multiplicidades divergentes em
`despesas_partidos_2014` e `receitas_partidos_2014` e obter evidência oficial ou
de conteúdo para a identidade das linhas. A semântica dos arquivos `_sup` foi
pesquisada em [nota separada](pesquisa-tse-2014-sup.md): o TSE os publica como
recurso distinto, mas não documenta a relação linha a linha com a prestação
regular; devem permanecer separados. Para 2016–2022, schemas, contagens e hashes
de conteúdo agora foram comparados por família e recorte. Em 2016, a cobertura
partidária adicional de `BR` e `DF` sustenta usar `_brasil` como fonte completa
deste snapshot, condicionada à revalidação em novas versões. A identidade das
chaves de negócio não foi determinada por estas comparações de conteúdo. As pesquisas de [2002](pesquisa-tse-2002.md),
[2004–2006](pesquisa-tse-2004-2006.md), [2008](pesquisa-tse-2008.md),
[2010–2012](pesquisa-tse-2010-2012.md) e [2024](pesquisa-tse-2024.md)
registram os demais recortes disponíveis.
