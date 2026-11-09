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

Os nomes `_BR`, `_BRASIL`, `_brasil` e `_sup` são distintos no manifesto. Não
devem ser somados nem tratados como cópias sem comparar schema, conteúdo e
procedência. O contrato Silver precisa decidir por família e ano qual conjunto
publicar, quais registros são complementares ou retificações, e como medir
conflitos e duplicatas. A regra de 2024 não pode ser aplicada a esses anos
somente pela semelhança dos nomes.

Próxima validação: comparar o conteúdo de `_BR` e `_brasil` com as partes por UF,
verificar a correspondência entre UF declarada e arquivo de origem, medir
nulidade, unicidade e colisões das chaves candidatas e identificar a relação
semântica dos arquivos `_sup` antes de fixar tipagem e deduplicação. O schema
físico já foi conferido para todas as partes de 2014. As pesquisas de
[2002](pesquisa-tse-2002.md), [2004–2006](pesquisa-tse-2004-2006.md), [2008](pesquisa-tse-2008.md),
[2010–2012](pesquisa-tse-2010-2012.md) e [2024](pesquisa-tse-2024.md)
registram os demais recortes disponíveis.
