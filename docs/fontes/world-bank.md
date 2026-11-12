# Fonte `world-bank` — indicadores de desenvolvimento por país (World Bank Open Data)

Indicadores de desenvolvimento publicados pelo **Banco Mundial** (World Bank Open Data), por país e ano: PIB, Gini, pobreza, população, expectativa de vida, alfabetização e desemprego. Complementa o eixo de comparação internacional, ligando o Brasil a pares por ISO 3166-1 alpha-3 + ano. Implementa a issue [#63](https://github.com/rikemorais/todeolhoaibr/issues/63).

## O que é ingerido

- Uma **requisição por indicador**, que traz **todos os valores** (todos os países e períodos) em JSON, da API v2 do World Bank
  (`https://api.worldbank.org/v2/country/all/indicator/{codigo}?format=json&per_page=20000`).
- Cada resposta é gravada **opaca** na camada Raw (ver [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).

### Indicadores

| Recurso (`indicador`) | Código World Bank | O que é |
| --- | --- | --- |
| `pib-per-capita` | NY.GDP.PCAP.CD | PIB per capita (US$ corrente) |
| `pib` | NY.GDP.MKTP.CD | PIB total (US$ corrente) |
| `gini` | SI.POV.GINI | Índice de Gini |
| `pobreza-extrema` | SI.POV.DDAY | Proporção abaixo de US$ 2,15/dia |
| `populacao` | SP.POP.TOTL | População total |
| `expectativa-vida` | SP.DYN.LE00.IN | Expectativa de vida ao nascer |
| `alfabetizacao` | SE.ADT.LITR.ZS | Taxa de alfabetização adulta |
| `desemprego` | SL.UEM.TOTL.ZS | Taxa de desemprego |

!!! note "Benchmark internacional"
    Esta fonte é uma **camada de comparação**: o vínculo geográfico é por **país** (ISO alpha-3), não por município IBGE. O join com dados brasileiros subnacionais exige agregação no nível nacional.

!!! note "Cobertura temporal"
    Cada indicador cobre períodos diferentes. O Gini, por exemplo, começa em 1963 mas tem dados para apenas ~170 países com valores não nulos. O PIB per capita cobre ~266 entidades (incluindo agregados regionais do Banco Mundial).

## Layout na Raw

```text
raw/world-bank/indicadores/<indicador>/_ultima_versao.json
raw/world-bank/indicadores/<indicador>/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.json
```

## Como rodar

```bash
# Todos os indicadores
todeolho executar world-bank

# Só um indicador (filtro por etiqueta)
todeolho executar world-bank --filtro indicador=gini
```

## Idempotência

Como as demais fontes de API, a pré-checagem é por estado na Raw: um indicador já ingerido e íntegro não é rebaixado ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)). Um limite de requisições no host `api.worldbank.org` evita sufocá-lo.

## Fora de escopo desta camada (fica no refino)

- **Recorte por país e período:** cada resposta traz todos os países e períodos; o valor vem com o código ISO alpha-3 (`countryiso3code`) e o ano (`date`). Separar por país e unir no cruzamento internacional é do refino ([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)).
