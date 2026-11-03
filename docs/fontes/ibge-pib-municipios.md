# Fonte `ibge-pib-municipios` — PIB per capita dos municípios

PIB e **PIB per capita** por município, série **anual**, do IBGE — preenche os anos entre os censos no eixo socioeconômico e alimenta o join `município + ano`. Implementa a [issue #30](https://github.com/rikemorais/todeolhoaibr/issues/30).

## O que é ingerido

Dois indicadores anuais, no nível **municipal** (`N6`, ~5.570 municípios), da API de agregados (v3) do IBGE:

| Indicador | Agregado/variável SIDRA | Período | Recurso na Raw |
| --- | --- | --- | --- |
| **PIB** a preços correntes (Mil Reais) | 5938 / 37 | 2002–2023 | `pib-municipios/ano=AAAA` |
| **População** residente estimada | 6579 / 9324 | 2001–2026 | `populacao-municipios/ano=AAAA` |

O **PIB per capita** (PIB ÷ população) é calculado no **refino**, unindo os dois no `município + ano` — a ingestão preserva as respostas **opacas** na Raw (ver [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).

!!! note "Uma tarefa por ano"
    A API do IBGE devolve **erro 500** ao pedir todos os municípios (`N6`) de vários períodos de uma vez. Por isso cada **ano** é uma tarefa própria, que baixa a série daquele ano (todos os municípios) em JSON. O PIB municipal sai com ~2 anos de defasagem, então o último ano de PIB fica atrás do de população.

## Layout na Raw

```text
raw/ibge/pib-municipios/ano=AAAA/_ultima_versao.json
raw/ibge/pib-municipios/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.json
raw/ibge/populacao-municipios/ano=AAAA/...
```

## Fonte separada, `sob_demanda`

É uma **fonte própria `ibge-pib-municipios` com `sob_demanda = true`**, separada da [`ibge`](ibge.md) (malha territorial e indicadores macro). São dezenas de anos no nível municipal (cada um ~700 KB), então mantê-la fora do `executar --todas` evita baixar tudo isso por acidente. O dado mora em `raw/ibge/`.

## Como rodar

```bash
# Tudo (PIB + população, todos os anos)
todeolho executar ibge-pib-municipios

# Só um ano
todeolho executar ibge-pib-municipios --filtro ano=2020

# Só um indicador
todeolho executar ibge-pib-municipios --filtro indicador=pib
```

## Idempotência

Como as demais fontes de API, a pré-checagem é por estado na Raw: um ano já ingerido e íntegro não é rebaixado ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)). Um limite de requisições no host `servicodados.ibge.gov.br` evita sufocá-lo ao buscar dezenas de anos.

## Fora de escopo desta camada (fica no refino)

- **PIB per capita e recorte por UF:** derivados no refino a partir do PIB, da população e do código do município (ver [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)). A ingestão só preserva o retrato bruto de cada ano.
