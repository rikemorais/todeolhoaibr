# Fonte `tse-candidaturas` — candidaturas e bens declarados

Registro de **candidaturas** e **bens declarados** dos candidatos do TSE, por eleição — o eixo político, incluindo a evolução patrimonial de quem concorreu ao longo das eleições. Implementa a [issue #58](https://github.com/rikemorais/todeolhoaibr/issues/58).

## O que é ingerido

Dois conjuntos por eleição (de dois em dois anos), nacionais, do repositório de dados abertos do TSE (`https://cdn.tse.jus.br/estatistica/sead/odsele/`):

| Conjunto (`tipo`) | Arquivo | Desde |
| --- | --- | --- |
| `candidaturas` | `consulta_cand/consulta_cand_{ano}.zip` | 1994 |
| `bens` | `bem_candidato/bem_candidato_{ano}.zip` | 2006 |

- `candidaturas` traz cargo, partido, UF, município e situação de cada candidato; `bens` traz o tipo e o valor declarado de cada bem.
- Cada ZIP é baixado como **arquivo grande** (em fluxo) e gravado **opaco** na Raw (ver
  [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)). O arquivo da eleição corrente,
  se ainda não publicado (HTTP 404), é tolerado; uma eleição histórica ausente continua sendo erro.

## Layout na Raw

Mesmo sendo uma fonte separada no catálogo, os arquivos ficam sob a fonte `tse`:

```text
raw/tse/candidaturas/ano=AAAA/...
raw/tse/bens/ano=AAAA/...
```

## Fonte separada, `sob_demanda`

É uma **fonte própria `tse-candidaturas` com `sob_demanda = true`**, separada da [`tse`](tse.md)
(votação por seção) e da [`tse-prestacao-contas`](tse-prestacao-contas.md), para que cada uma rode
sozinha. O dado mora em `raw/tse/`.

## Como rodar

```bash
# Tudo (candidaturas + bens, todas as eleições)
todeolho executar tse-candidaturas

# Só uma eleição
todeolho executar tse-candidaturas --filtro ano=2022

# Só um conjunto
todeolho executar tse-candidaturas --filtro tipo=bens
```

## Idempotência

Reusa a pré-checagem por versão/tamanho ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)): a
origem envia `ETag` (conferido no arquivo real), usado como validador autoritativo — um arquivo
revisado ganha ETag novo e é reingerido; um inalterado é reaproveitado.

## Fora de escopo desta camada (fica no refino)

- **Filtro por UF e evolução patrimonial:** cada ZIP é **nacional por eleição**; a UF e o valor dos
  bens são colunas dos CSVs. Como a Raw é opaca, o recorte por UF e o acompanhamento do patrimônio
  ao longo das eleições são do refino/consulta.
- **Deduplicação por chave:** a chave (`candidato + eleição`, e o item de bem) é tratada no refino
  (camada `silver`) — ver [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md). A idempotência
  **da ingestão** é por versão do arquivo (acima).
