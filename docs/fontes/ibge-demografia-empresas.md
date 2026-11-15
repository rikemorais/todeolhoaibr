# IBGE — Demografia das Empresas e Estatísticas de Empreendedorismo

Fonte: **ibge-demografia-empresas** (`FonteIbgeDemografiaEmpresas`)

A pesquisa *Demografia das Empresas* do IBGE acompanha **natalidade, mortalidade e
sobrevivência de empresas empregadoras** (com ao menos 1 assalariado no ano de
referência). Os microdados ficam na API SIDRA (`apisidra.ibge.gov.br/values`).

## Tabelas SIDRA ingeridas

| Recurso | Tabela | Conteúdo | Nível | Porte | Período |
|---------|--------|----------|-------|-------|---------|
| `natalidade-mortalidade-uf` | 9925 | Nascimento/morte de unidades locais | UF (N3) | Não | 2015–2022 |
| `sobrevivencia-uf` | 9950 | Taxas de sobrevivência 1–5 anos | UF (N3) | Não | 2017–2021 |
| `natalidade-mortalidade-porte` | 9924 | Nascimento/morte de empresas | Brasil (N1) | Sim (C370) | 2015–2022 |
| `sobrevivencia-porte` | 9949 | Taxas de sobrevivência 1–5 anos | Brasil (N1) | Sim (C370) | 2017–2021 |

Limitação do SIDRA: as tabelas com desagregação por UF **não** incluem porte, e as
que incluem porte são apenas nível Brasil. Não existe tabela cruzando UF × porte ×
tipo de evento.

## Classificações fixadas

- **C12762 = 117897** — CNAE 2.0 Total (todas as seções)
- **C370** (quando disponível) — Faixas de pessoal: Total / 1–9 / 10–49 / 50+
- **C371 / C369** — Tipo de evento: nascimento, morte, total de ativas

## Formato da resposta

JSON array. O primeiro elemento é a linha de cabeçalho com rótulos dos campos. Cada
objeto de dados contém campos `D1C`/`D1N` (localidade), `D2C`/`D2N` (variável),
`D3C`/`D3N` (período), `D4C`/`D4N` e `D5C`/`D5N` (classificações), além de `V`
(valor), `NC`/`NN` (nível territorial) e `MC`/`MN` (unidade de medida).

Valores especiais em `V`: `"..."` = dado indisponível; `"-"` = suprimido.

## Bronze

Decodificador: `DecodificadorJson` — grava cada campo como `Utf8`.

## Referências

- [IBGE — Demografia das Empresas](https://www.ibge.gov.br/estatisticas/economicas/servicos/22649-demografia-das-empresas-e-estatisticas-de-empreendedorismo.html)
- [SIDRA — Tabelas da pesquisa](https://sidra.ibge.gov.br/pesquisa/demografia-das-empresas/tabelas)
- [API SIDRA — Ajuda](https://apisidra.ibge.gov.br/home/ajuda)
