# Fonte `undp` — IDH global e índices compostos (UNDP/Human Development Report)

Índices compostos de desenvolvimento humano publicados pelo **UNDP/HDRO** (Human Development Report Office): **HDI**, GDI, GII, PHDI, MPI e seus componentes (expectativa de vida, escolaridade, renda per capita), por país e ano (1990-2023). Situa o IDHM brasileiro no contexto global. Implementa a issue [#64](https://github.com/rikemorais/todeolhoaibr/issues/64).

## O que é ingerido

- O **CSV completo** dos índices compostos e séries temporais do Human Development Report 2025, publicado pelo UNDP em
  [`hdr.undp.org/data-center/documentation-and-downloads`](https://hdr.undp.org/data-center/documentation-and-downloads).
- O arquivo é gravado **opaco** na camada Raw (ver [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).

### Recurso

| Recurso | Arquivo | O que é |
| --- | --- | --- |
| `indices-compostos` | `HDR25_Composite_indices_complete_time_series.csv` | Todos os índices compostos (HDI, GDI, GII, PHDI, MPI) e componentes, por país/ano (1990-2023) |

O CSV tem formato wide: cada linha é um país (`iso3`, `country`, `region`), com colunas `{indicador}_{ano}` (ex.: `hdi_2023`, `le_2023`, `eys_2023`, `mys_2023`, `gnipc_2023`). ~2 MB, ~200 países.

!!! note "Benchmark internacional"
    Esta fonte é uma **camada de comparação**: o vínculo geográfico é por **país** (ISO alpha-3), não por município IBGE. O join com dados brasileiros subnacionais exige agregação no nível nacional. Complementa a fonte `world-bank` (issue #63).

!!! note "Edição do relatório"
    A URL aponta para a edição **HDR 2025** (dados até 2023). Quando o UNDP publicar uma nova edição, a URL precisará ser atualizada. O CSV anterior não é removido do servidor.

## Layout na Raw

```text
raw/undp/indices-compostos/_ultima_versao.json
raw/undp/indices-compostos/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.csv
```

## Como rodar

```bash
todeolho executar undp
```

## Idempotência

A pré-checagem é por estado na Raw: o recurso já ingerido e íntegro não é rebaixado ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)). Um limite de requisições no host `hdr.undp.org` evita sufocá-lo.

## Fora de escopo desta camada (fica no refino)

- **Pivotear o formato wide para long:** o CSV tem uma coluna por indicador×ano; o refino transforma em linhas `(país, ano, indicador, valor)`.
- **Recorte por país e período:** separar Brasil e pares é do refino ([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)).
