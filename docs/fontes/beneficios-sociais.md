# Benefícios sociais — Portal da Transparência

A fonte `transparencia-beneficios` ingere pagamentos mensais de Bolsa Família,
Auxílio Brasil, Novo Bolsa Família e BPC/LOAS, particionados por município e
competência. O conteúdo original é preservado na Raw, sem extração de CPF, nome ou
valor para tabelas analíticas.

## Origem e autenticação

As consultas usam os endpoints públicos do Portal da Transparência:

| Programa | Endpoint | Competências disponíveis na fonte |
| --- | --- | --- |
| Bolsa Família | `bolsa-familia-sacado-beneficiario-por-municipio` | 2013-01 a 2021-10 |
| Auxílio Brasil | `auxilio-brasil-sacado-beneficiario-por-municipio` | 2021-11 a 2023-02 |
| Novo Bolsa Família | `novo-bolsa-familia-sacado-beneficiario-por-municipio` | 2023-03 até dois meses antes da execução |
| BPC/LOAS | `bpc-beneficiario-por-municipio` | 2019-01 até dois meses antes da execução |

O Portal exige uma chave obtida pelo [cadastro da API](https://portaldatransparencia.gov.br/api-de-dados).
Configure-a como `PORTAL_TRANSPARENCIA_TOKEN`; o cliente envia o valor no cabeçalho
`chave-api-dados`, restrito ao host da API. O limite declarado é 90 requisições por
minuto. A lista municipal vem do recurso `ibge/municipios` já ingerido na Raw; execute
`ibge` antes desta fonte quando ainda não houver um manifesto IBGE íntegro.

As respostas incluem dados pessoais de beneficiários. Elas ficam opacas na Raw
([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)); esta fonte não
cria tabelas Silver nem disponibiliza dados nominais no painel. O uso e o acesso ao
bucket Raw devem seguir as regras de governança aplicáveis ao projeto.

## Particionamento e paginação

Cada competência é uma tarefa filtrável. Ao executar a tarefa, os códigos municipais
são lidos do manifesto vigente do IBGE e cada município gera um recurso com
manifesto próprio:

```text
raw/transparencia/beneficios/programa=<programa>/municipio=<codigo-ibge>/competencia=AAAA-MM/
```

As páginas são lidas até a API retornar um array vazio. A chave e o manifesto por
programa, município e competência permitem retomar e reaproveitar recursos de forma
independente ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)).

## Como executar

A fonte é `sob_demanda` e não entra em `executar --todas`. A API pode exigir milhares
de requisições para uma única competência nacional; use os filtros para limitar a
execução:

```bash
export PORTAL_TRANSPARENCIA_TOKEN='...'
docker compose run --rm --build ingestao executar transparencia-beneficios \
  --filtro programa=novo-bolsa-familia \
  --filtro competencia=2026-09
```

Valores aceitos para `programa`: `bolsa-familia`, `auxilio-brasil`,
`novo-bolsa-familia` e `bpc`. `competencia` usa `AAAA-MM`. A execução pode demorar
devido ao limite de taxa da API.
