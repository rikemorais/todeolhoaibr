# Fonte `ibge` — malha territorial (Localidades) e indicadores macro

A fonte `ibge` reúne dois conjuntos de recursos:

- **Localidades** — regiões, estados e municípios do Brasil, da **API de Localidades** — a base territorial para cruzar qualquer outra fonte por UF/município.
- **Indicadores macro** — inflação (IPCA e INPC), crescimento (PIB trimestral) e mercado de trabalho (PNAD Contínua), da **API de agregados v3 (SIDRA)** — a base de comparação temporal para os eixos de renda e empresas.

- **Origem (Localidades):** `https://servicodados.ibge.gov.br/api/v1/localidades` (ex.: `.../regioes`) — <https://servicodados.ibge.gov.br/api/docs/localidades>.
- **Origem (Indicadores):** `https://servicodados.ibge.gov.br/api/v3/agregados` — <https://servicodados.ibge.gov.br/api/docs/agregados-por-municipio>.

## Localidades

### O que é ingerido

Três recursos JSON, sem paginação (a API devolve tudo de uma vez), via
`TarefaDeIngestaoHttp` e gravados **opacos** na Raw ([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)):

| Recurso | Endpoint | Depende de |
| --- | --- | --- |
| `regioes` | `/localidades/regioes` | — |
| `estados` | `/localidades/estados` | `regioes` |
| `municipios` | `/localidades/municipios` | `estados` |

As dependências ordenam a DAG (estados depois de regiões, municípios depois de estados), mas cada recurso é um arquivo independente na Raw.

## Indicadores macro

### O que é ingerido

Cada indicador é um recurso que baixa a **série completa** (todos os períodos) de uma variável de um agregado do SIDRA, sem paginação, via `TarefaDeIngestaoHttp` e gravado **opaco** na Raw. O nível territorial é Nacional (`N1`) ou por UF (`N3`) — nunca municipal.

| Recurso | Indicador | Agregado / variável | Nível | Periodicidade |
| --- | --- | --- | --- | --- |
| `ipca` | IPCA — variação mensal | 1737 / 63 | `N1` Brasil | mensal |
| `inpc` | INPC — variação mensal | 1736 / 44 | `N1` Brasil | mensal |
| `pib-trimestral` | PIB — índice de volume encadeado | 1620 / 583 | `N1` Brasil | trimestral |
| `pnad-desocupacao` | PNAD Contínua — taxa de desocupação | 4093 / 4099 | `N3` por UF | trimestral |

A URL de cada recurso pede todos os períodos e todas as localidades do nível, por exemplo:

```text
https://servicodados.ibge.gov.br/api/v3/agregados/1737/periodos/all/variaveis/63?localidades=N1[all]
```

Os indicadores não têm dependências entre si: cada um é um arquivo independente na Raw.

## Layout na Raw

```text
raw/ibge/<recurso>/_ultima_versao.json
raw/ibge/<recurso>/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.json
raw/ibge/<recurso>/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

## Como rodar

```bash
docker compose run --rm --build ingestao executar ibge
```

Fonte **leve** (~2,4 MB) e **não** `sob_demanda`: entra no `executar --todas`.

## Idempotência

Pré-checagem por versão/tamanho ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)): requisição condicional `If-None-Match` com o `ETag` da rodada anterior; `304` reaproveita o que já está na Raw.

## Fora de escopo desta camada (fica no refino)

A Raw guarda o JSON **opaco**. Interpretar os campos — achatar a hierarquia região → estado → município numa tabela consultável, cruzar por código de município, e desdobrar as séries do SIDRA em observações por indicador, recorte e período — é trabalho do refino ([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)).
