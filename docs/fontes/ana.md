# Fonte `ana` — Inventário de estações hidrológicas (ANA/SNIRH)

Inventário completo de estações da rede hidrometeorológica nacional, operado pela **ANA** (Agência Nacional de Águas e Saneamento Básico) via SNIRH. Implementa a issue [#48](https://github.com/rikemorais/todeolhoaibr/issues/48).

## O que é ingerido

- **Estações fluviométricas** (tipo 1): catálogo de estações que medem vazão e nível dos rios. Inclui bacia, sub-bacia, rio, município, UF, coordenadas, responsável, operadora e status. ~61 MB XML.
- **Estações pluviométricas** (tipo 2): catálogo de estações que medem precipitação (chuva). Mesma estrutura. ~76 MB XML.
- Dados gravados **opacos** na Raw (ver [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).

### Recursos

| Recurso | Tipo | O que é |
| --- | --- | --- |
| `estacoes-fluviometricas` | 1 | Estações de vazão/nível (~61 MB) |
| `estacoes-pluviometricas` | 2 | Estações de chuva (~76 MB) |

### API

A API é REST via `ServiceANA.asmx/HidroInventario` com query params, retornando XML (DataSet). Sem autenticação.

```
GET https://telemetriaws1.ana.gov.br/ServiceANA.asmx/HidroInventario?tpEst={tipo}&...
```

O XML traz campos: `BaciaCodigo`, `SubBaciaCodigo`, `RioCodigo`, `RioNome`, `EstadoCodigo`, `nmEstado`, `MunicipioCodigo`, `nmMunicipio`, `Latitude`, `Longitude`, `Codigo`, `Nome`, `TipoEstacao`, `StatusEstacao`, entre outros.

!!! warning "Sob demanda"
    ~137 MB total: a fonte não entra em `--todas`. Execute explicitamente com `todeolho executar ana`.

## Layout na Raw

```text
raw/ana/estacoes-fluviometricas/_ultima_versao.json
raw/ana/estacoes-fluviometricas/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.xml
raw/ana/estacoes-pluviometricas/_ultima_versao.json
raw/ana/estacoes-pluviometricas/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.xml
```

## Como rodar

```bash
todeolho executar ana
```

## Idempotência

A pré-checagem é por estado na Raw ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)). Rate limit conservador (1 req/2s) para não sobrecarregar o serviço da ANA.

## Granularidade geográfica

Cada estação traz `MunicipioCodigo` e `nmEstado`, permitindo recorte municipal e estadual direto — sem necessidade de join externo.

## Fora de escopo desta camada (fica no refino ou em evolução futura)

- **Parse do XML DataSet**: transformar as `Table` em linhas tabulares fica no refino.
- **Séries históricas** (`HidroSerieHistorica`): medições ao longo do tempo por estação — volume potencialmente enorme; será avaliado como fonte separada.
- **SAR (reservatórios)**: Sistema de Acompanhamento de Reservatórios é uma SPA sem API pública direta.
- **Outorgas (CNARH)**: cadastro de outorgas é outro sistema (webservice separado).
