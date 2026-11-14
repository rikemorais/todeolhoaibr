# Emendas parlamentares

A fonte `emendas` preserva dois conjuntos oficiais na Raw, sem transformar os bytes:

| Origem | Recurso Raw | Grão e conteúdo |
| --- | --- | --- |
| Portal da Transparência (CGU) | `cgu/ano=AAAA` | Emenda, autor, localidade de gasto e valores empenhado, liquidado, pago e restos a pagar; páginas JSON por exercício desde 2015 |
| Tesouro Transparente | `repasses/tesouro` | CSV completo de repasses mensais desde janeiro de 2015, com município, UF, código IBGE, ordem bancária, favorecido, categoria e valor |

Os conjuntos têm grãos e semânticas diferentes. A API CGU reporta valores por emenda e localidade, enquanto o Tesouro registra pagamentos por ordem bancária e favorecido. Não associe uma ordem bancária a um autor ou código de emenda sem uma chave documentada pela origem. Nenhum conjunto é deduplicado na Raw.

## Acesso e atualização

O CSV do Tesouro é público e tem cerca de 67 MiB. É baixado como um arquivo integral e atualizado mensalmente. A CGU exige uma chave gratuita da API, obtida pelo [cadastro do Portal da Transparência](https://portaldatransparencia.gov.br/api-de-dados/cadastrar-email). Configure `PORTAL_TRANSPARENCIA_TOKEN` no `.env`; o cliente envia o segredo somente ao host `api.portaldatransparencia.gov.br`, no cabeçalho `chave-api-dados`. A origem tem limite configurado de 90 requisições por minuto. As páginas CGU terminam quando a API retorna um array vazio; a ingestão impõe um limite de segurança de páginas.

Para baixar tudo (CSV do Tesouro e todos os anos da CGU), rode:

```bash
docker compose run --rm --build ingestao executar emendas
```

A fonte é sob demanda e fica fora de `executar --todas`. Para selecionar apenas um ano CGU:

```bash
docker compose run --rm --build ingestao executar emendas --filtro ano=2025
```

Para baixar somente o CSV integral do Tesouro:

```bash
docker compose run --rm --build ingestao executar emendas --filtro origem=tesouro
```

`--filtro ano=...` seleciona as tarefas CGU; o CSV do Tesouro é um único arquivo histórico, portanto não é fragmentado por ano ou UF na Raw. A API CGU não oferece filtro de UF. O CSV do Tesouro contém UF e código IBGE, e as respostas CGU contêm `localidadeDoGasto` como texto; filtros geográficos e uma eventual associação devem acontecer no refino, após validar o contrato dos dados. A Raw preserva respostas completas conforme o [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md).

## Campos e idempotência

O CSV do Tesouro vem delimitado por ponto e vírgula em Latin-1. Entre os campos originais estão `Código IBGE`, `UF`, `Ano`, `Mês`, `OB`, `CNPJ do Favorecido` e `Valor`. `Data` vem como serial numérico do Excel no arquivo consultado. O CSV integral permanece como recebido; decodificação e conversão ficam para Bronze.

Na CGU, os recursos são separados por ano, com páginas dentro do manifesto `cgu/ano=AAAA`. O `codigoEmenda` identifica a emenda, mas pode haver mais de uma localidade por emenda; ele não é tratado como chave única da linha. Na Raw, os manifestos e validadores HTTP tornam cada página idempotente e permitem reaproveitar páginas que não mudaram. Repetições no conteúdo são preservadas.

Consulte a [pesquisa das fontes e seus limites](pesquisa-emendas.md), que registra os endpoints oficiais, os campos e as limitações de filtragem e relacionamento.
