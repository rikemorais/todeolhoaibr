# Fonte `ibge` — malha territorial (Localidades)

Regiões, estados e municípios do Brasil, da **API de Localidades do IBGE** — a base territorial para cruzar qualquer outra fonte por UF/município.

Documentação: <https://servicodados.ibge.gov.br/api/docs/localidades>.

## O que é ingerido

Três recursos JSON, sem paginação (a API devolve tudo de uma vez), via
`TarefaDeIngestaoHttp` e gravados **opacos** na Raw ([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)):

| Recurso | Endpoint | Depende de |
| --- | --- | --- |
| `regioes` | `/localidades/regioes` | — |
| `estados` | `/localidades/estados` | `regioes` |
| `municipios` | `/localidades/municipios` | `estados` |

As dependências ordenam a DAG (estados depois de regiões, municípios depois de estados), mas cada recurso é um arquivo independente na Raw.

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

A Raw guarda o JSON **opaco**. Interpretar os campos, achatar a hierarquia região → estado → município numa tabela consultável e cruzar com outras fontes por código de município é trabalho do refino ([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)).
