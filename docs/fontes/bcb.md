# Fonte `bcb` — séries macroeconômicas (Banco Central/SGS)

Séries macroeconômicas do **Banco Central**, via **SGS** (Sistema Gerenciador de Séries Temporais): juros, câmbio, inflação, atividade, crédito e indicadores fiscais. É a régua econômica que contextualiza o resto do acervo. Implementa a [issue #56](https://github.com/rikemorais/todeolhoaibr/issues/56).

## O que é ingerido

- Uma **requisição por série**, que traz a **história completa** em JSON, da API do SGS
  (`https://api.bcb.gov.br/dados/serie/bcdata.sgs.{codigo}/dados?formato=json`).
- Cada resposta é gravada **opaca** na camada Raw — a ingestão não interpreta o JSON (ver
  [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).

### Séries

| Recurso (`serie`) | Código SGS | O que é |
| --- | --- | --- |
| `selic-mensal` | 4390 | Taxa SELIC acumulada no mês (% a.m.) |
| `cdi-mensal` | 4391 | CDI acumulado no mês (% a.m.) |
| `cambio-dolar-mensal` | 3698 | Câmbio R$/US$ — venda, média mensal |
| `ipca-mensal` | 433 | IPCA — variação mensal |
| `ipca-12-meses` | 13522 | IPCA acumulado em 12 meses |
| `inpc-mensal` | 188 | INPC — variação mensal |
| `igpm-mensal` | 189 | IGP-M — variação mensal |
| `ibc-br` | 24364 | IBC-Br — índice de atividade econômica |
| `inadimplencia-credito-total` | 21082 | Inadimplência da carteira de crédito (%) |
| `credito-total-pib` | 20622 | Saldo de crédito / PIB (%) |
| `divida-liquida-setor-publico-pib` | 4513 | Dívida líquida do setor público / PIB (%) |

!!! note "Por que mensais"
    As séries de periodicidade **diária** (câmbio e juros diários, reservas internacionais) só
    podem ser consultadas em janelas de até 10 anos por requisição — não cabem no modelo de "um
    blob opaco com a série inteira". Por isso usamos as equivalentes **mensais**. Séries diárias (e
    estatísticas do PIX) podem entrar depois, com busca por janela de data.

## Layout na Raw

```text
raw/bcb/series/<serie>/_ultima_versao.json                                       # ponteiro do vigente
raw/bcb/series/<serie>/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.json
raw/bcb/series/<serie>/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

## Como rodar

É uma fonte **leve** (entra no `executar --todas`).

```bash
# Todas as séries
todeolho executar bcb

# Só uma série (filtro por etiqueta)
todeolho executar bcb --filtro serie=selic-mensal
```

## Idempotência

Como as demais fontes de API, a pré-checagem é por estado na Raw: uma série já ingerida e íntegra
não é rebaixada ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)). A API do SGS não envia
`ETag`, então a rodada compara pelo conteúdo guardado; cada ingestão grava o retrato atual da série.

Um limite de requisições por host (`api.bcb.gov.br`) evita o estrangulamento ao buscar várias
séries de uma vez.

## Fora de escopo desta camada (fica no refino)

- Interpretar o JSON (datas, valores), unir séries e montar painéis é trabalho do refino (camadas
  `silver`/`gold`) — ver [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md). A ingestão só
  preserva o retrato bruto de cada série.
