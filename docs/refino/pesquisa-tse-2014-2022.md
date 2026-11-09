# Prestação de contas TSE 2014–2022: inventário Bronze

Este inventário lê somente os manifestos apontados por `_ultima_versao.json` no
Bronze local em 2026-11-09. Conta partes Parquet por nome de tabela interna;
não valida linhas, chaves, tipos nem equivalência entre partes por UF e
consolidados. Portanto, ainda não autoriza publicação Silver desses anos.

| Ano | Partes | Famílias observadas | Formas nacionais e arquivos especiais |
| --- | ---: | --- | --- |
| 2014 | 178 | despesas/receitas de candidatos, comitês e partidos (seis famílias) | Cada família tem `_BR` e `_brasil`, além de 26 UFs; quatro arquivos `_sup` aparecem nas famílias de candidatos/partidos. |
| 2016 | 112 | despesas/receitas de candidatos e partidos na prestação final (quatro famílias) | Cada família tem 26 UFs, `_brasil` e `_sup`. |
| 2018 | 116 | despesas contratadas, despesas pagas, receitas e doador originário de candidatos (quatro famílias) | Cada família tem 27 partes regionais, `_BR` e `_BRASIL`. |
| 2020 | 108 | As mesmas quatro famílias de 2018 | Cada família tem 26 UFs e `_BRASIL`; não há `_BR` no manifesto. |
| 2022 | 116 | As mesmas quatro famílias de 2018 | Cada família tem 27 partes regionais, `_BR` e `_BRASIL`. |

Os nomes `_BR`, `_BRASIL`, `_brasil` e `_sup` são distintos no manifesto. Não
devem ser somados nem tratados como cópias sem comparar schema, conteúdo e
procedência. O contrato Silver precisa decidir por família e ano qual conjunto
publicar, quais registros são complementares ou retificações, e como medir
conflitos e duplicatas. A regra de 2024 não pode ser aplicada a esses anos
somente pela semelhança dos nomes.

Próxima validação: ler os schemas físicos de cada família, comparar as formas
nacionais com as UFs e medir nulidade, unicidade e colisões das chaves candidatas
antes de fixar a tipagem e a deduplicação. As pesquisas de
[2004–2006](pesquisa-tse-2004-2006.md), [2008](pesquisa-tse-2008.md),
[2010–2012](pesquisa-tse-2010-2012.md) e [2024](pesquisa-tse-2024.md)
registram os demais recortes disponíveis.
