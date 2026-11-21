# Cartão de pagamento do governo — Portal da Transparência

A fonte `transparencia-cpgf` ingere as transações mensais do cartão de pagamento
do governo federal (CPGF, CPCC e CPDC) via API v2 do Portal da Transparência,
particionadas por competência. O conteúdo original é preservado na Raw.

## Origem e autenticação

As consultas usam o endpoint `cartoes` da API de Dados do Portal da Transparência:

```
GET /api-de-dados/cartoes?mesExtratoInicio=MM/AAAA&mesExtratoFim=MM/AAAA&pagina=N
```

A resposta inclui favorecido (nome/CNPJ), data e valor de cada transação, tipo
do cartão (CPGF/CPCC/CPDC), portador, unidade gestora e órgão vinculado.

O Portal exige uma chave obtida pelo [cadastro da API](https://portaldatransparencia.gov.br/api-de-dados).
Configure-a como `PORTAL_TRANSPARENCIA_TOKEN`; o cliente envia o valor no cabeçalho
`chave-api-dados`, restrito ao host da API. O limite declarado é 90 requisições por
minuto. A mesma variável é compartilhada com as fontes `transparencia-beneficios`
e `emendas`.

## Escopo

Este recurso cobre o **Executivo federal**, não a cota parlamentar (CEAP) do
Legislativo. Dados de diárias (viagens a serviço) e contratos não estão incluídos
nesta fonte por exigirem iteração por código de órgão SIAFI.

## Particionamento e paginação

Cada competência (mês) é uma tarefa filtrável, de 2014-01 até dois meses antes
da execução:

```text
raw/transparencia/cartoes/competencia=AAAA-MM/
```

As páginas são lidas até a API retornar um array JSON vazio. O manifesto por
competência permite retomar e reaproveitar recursos de forma independente
([ADR 0012](../adr/0012-pre-checagem-idempotente.md)).

## Como executar

A fonte é `sob_demanda` e não entra em `executar --todas`. Use os filtros para
limitar a execução:

```bash
export PORTAL_TRANSPARENCIA_TOKEN='...'
docker compose run --rm --build ingestao executar transparencia-cpgf \
  --filtro competencia=2024-06
```

`competencia` usa `AAAA-MM`. A execução pode demorar devido ao limite de taxa da
API (90 req/min).
