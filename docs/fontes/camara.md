# Fonte `camara` — API da Câmara dos Deputados

Dados cadastrais da Câmara: legislaturas, partidos e deputados em exercício, da
**API de Dados Abertos** (v2). É a fonte "leve" da Câmara — as de volume (votações,
despesas) são separadas e `sob_demanda`.

- **Origem (base da API):** `https://dadosabertos.camara.leg.br/api/v2` (ex.: `.../deputados?itens=100`).
- **Documentação:** <https://dadosabertos.camara.leg.br/swagger/api.html>.

## O que é ingerido

Três recursos JSON **paginados** (a API devolve links `rel: "next"`), percorridos
página a página por `TarefaDeIngestaoHttp` e gravados **opacos** na Raw
([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)):

| Recurso | Endpoint | Depende de |
| --- | --- | --- |
| `legislaturas` | `/legislaturas?itens=100` | — |
| `partidos` | `/partidos?itens=100` | — |
| `deputados` | `/deputados?itens=100` | `legislaturas`, `partidos` |

## Layout na Raw

```text
raw/camara/<recurso>/_ultima_versao.json
raw/camara/<recurso>/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.json  # uma parte por página
raw/camara/<recurso>/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

!!! note "Outras fontes sob a mesma pasta"
    As fontes [`camara-votacoes`](camara-votacoes.md), [`camara-despesas`](camara-despesas.md)
    e [`camara-tramitacoes`](camara-tramitacoes.md) gravam sob `raw/camara/votacoes/`,
    `raw/camara/despesas/` e `raw/camara/tramitacoes/`, mas são **fontes separadas**
    no catálogo (para serem `sob_demanda`). Esta aqui é só o cadastro.

## Como rodar

```bash
docker compose run --rm --build ingestao executar camara
```

Fonte **leve** (~200 KB) e **não** `sob_demanda`: entra no `executar --todas`.

## Idempotência

Pré-checagem por página ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)):
cada página usa `If-None-Match` com o `ETag` da rodada anterior; páginas inalteradas
são reaproveitadas, sem regravar.

## Fora de escopo desta camada (fica no refino)

A Raw guarda as páginas JSON **opacas**. Juntar as páginas, interpretar os campos e
cruzar legislaturas ↔ partidos ↔ deputados numa base consultável é trabalho do refino
([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)).
