# Fonte `ipeadata` — séries socioeconômicas regionais (IPEA)

Séries socioeconômicas **já consolidadas** do Ipeadata (IPEA): **IDHM**, renda, desigualdade (Gini), pobreza e **vulnerabilidade social (IVS)**, por território. Complementa o eixo de desenvolvimento no join `município + ano`, evitando reconstruir tudo do Censo bruto. Implementa as issues [#32](https://github.com/rikemorais/todeolhoaibr/issues/32) e [#31](https://github.com/rikemorais/todeolhoaibr/issues/31) (IDHM).

## O que é ingerido

- Uma **requisição por série**, que traz **todos os valores** (todos os territórios e períodos) em JSON (OData), da API do Ipeadata
  (`https://www.ipeadata.gov.br/api/odata4/Metadados('{codigo}')/Valores`).
- Cada resposta é gravada **opaca** na camada Raw (ver [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).

### Séries

| Recurso (`serie`) | Código Ipeadata | O que é |
| --- | --- | --- |
| `renda-per-capita` | ADH_RDPC | Renda per capita média (Atlas DH – Censo), por município |
| `gini` | ADH_GINI | Índice de Gini (Atlas DH – Censo), por município |
| `proporcao-pobres` | ADH_PMPOB | Proporção de pessoas pobres (Atlas DH – Censo) |
| `proporcao-extremamente-pobres` | ADH_PIND | Proporção de pessoas extremamente pobres (Atlas DH – Censo) |
| `ivs` | AVS_IVS | Índice de Vulnerabilidade Social (Atlas da Vulnerabilidade Social) |
| `idhm` | ADH_IDHM | IDHM geral (Atlas DH – Censo), por município |
| `idhm-educacao` | ADH_IDHM_E | IDHM — dimensão educação (Atlas DH – Censo), por município |
| `idhm-longevidade` | ADH_IDHM_L | IDHM — dimensão longevidade (Atlas DH – Censo), por município |
| `idhm-renda` | ADH_IDHM_R | IDHM — dimensão renda (Atlas DH – Censo), por município |

!!! note "Procedência"
    São séries **derivadas pelo IPEA** a partir do Censo/PNAD; a fonte canônica de cada uma é o IBGE. As do Atlas DH são **decenais** (1991, 2000, 2010, por município); o IVS é por região/UF.

!!! note "IDHM: recálculo e limite temporal"
    O IDHM de 1991 e 2000 foi **recalculado** com a metodologia de 2010 pelo Atlas do Desenvolvimento Humano (PNUD/IPEA/Fundação João Pinheiro). **Não há edição baseada no Censo 2022** ainda — a série termina em 2010. Cada série traz ~16.776 observações (5.592 municípios × 3 anos).

## Layout na Raw

```text
raw/ipeadata/series/<serie>/_ultima_versao.json
raw/ipeadata/series/<serie>/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.json
```

## Como rodar

```bash
# Todas as séries
todeolho executar ipeadata

# Só uma série (filtro por etiqueta)
todeolho executar ipeadata --filtro serie=gini
```

## Idempotência

Como as demais fontes de API, a pré-checagem é por estado na Raw: uma série já ingerida e íntegra não é rebaixada ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)). Um limite de requisições no host `www.ipeadata.gov.br` evita sufocá-lo.

## Fora de escopo desta camada (fica no refino)

- **Recorte por município/UF e período:** cada resposta traz todos os territórios e períodos; o valor vem com o código territorial (`TERCODIGO`) e o nível (`NIVNOME`). Separar por município/UF e unir no `município + ano` é do refino ([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)).
