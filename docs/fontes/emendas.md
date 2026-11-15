# Emendas parlamentares

A fonte `emendas` preserva dois conjuntos oficiais na Raw, sem transformar os bytes:

| Origem | Recurso Raw | Grão e conteúdo |
| --- | --- | --- |
| Portal da Transparência (CGU) | `cgu/ano=AAAA` | Emenda, autor, localidade de gasto e valores empenhado, liquidado, pago e restos a pagar; páginas JSON por exercício desde 2015 |
| Tesouro Transparente | `repasses/tesouro` | CSV completo de repasses mensais desde janeiro de 2015, com município, UF, código IBGE, ordem bancária, favorecido, categoria e valor |

Os conjuntos têm grãos e semânticas diferentes. A API CGU reporta valores por emenda e localidade, enquanto o Tesouro registra pagamentos por ordem bancária e favorecido. Não associe uma ordem bancária a um autor ou código de emenda sem uma chave documentada pela origem. Nenhum conjunto é deduplicado na Raw.

## Acesso e atualização

O CSV do Tesouro é público e tem cerca de 67 MiB. É baixado como um arquivo integral e atualizado mensalmente. A CGU exige uma chave gratuita da API, obtida pelo [cadastro do Portal da Transparência](https://portaldatransparencia.gov.br/api-de-dados/cadastrar-email). Configure `PORTAL_TRANSPARENCIA_TOKEN` no `.env` para consultar a CGU; o cliente envia o segredo somente ao host `api.portaldatransparencia.gov.br`, no cabeçalho `chave-api-dados`. A origem tem limite configurado de 90 requisições por minuto. As páginas CGU terminam quando a API retorna um array vazio; a ingestão impõe um limite de segurança de páginas. O filtro `origem=tesouro` seleciona apenas a tarefa pública e não exige esse token.

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

`--filtro ano=...` seleciona as tarefas CGU; o CSV do Tesouro é um único arquivo histórico, portanto não é fragmentado por ano ou UF na Raw. A API CGU não oferece filtro de UF. O CSV do Tesouro contém UF e código IBGE, e as respostas CGU contêm `localidadeDoGasto` como texto. Os filtros são aplicados nos contratos Silver documentados abaixo; associação entre autoria e pagamentos depende de uma chave ainda não identificada. A Raw preserva respostas completas conforme o [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md).

## Campos e idempotência

O CSV do Tesouro vem delimitado por ponto e vírgula em Latin-1. Entre os campos originais estão `Código IBGE`, `UF`, `Ano`, `Mês`, `OB`, `CNPJ do Favorecido` e `Valor`. `Data` vem como serial numérico do Excel; `Mês` vem por extenso em português e a Silver converte o nome para o número correspondente. O CSV integral permanece como recebido; decodificação e conversão ficam para as camadas seguintes.

Na CGU, os recursos são separados por ano, com páginas dentro do manifesto `cgu/ano=AAAA`. O `codigoEmenda` identifica a emenda, mas pode haver mais de uma localidade por emenda; ele não é tratado como chave única da linha. Na Raw, os manifestos e validadores HTTP tornam cada página idempotente e permitem reaproveitar páginas que não mudaram. Repetições no conteúdo são preservadas.

Consulte a [pesquisa das fontes e seus limites](pesquisa-emendas.md), que registra os endpoints oficiais, os campos e as limitações de filtragem e relacionamento.

## Refino Silver

O Bronze decodifica `emendas` para `emendas-cgu` e `emendas-tesouro`. A Silver publica contratos independentes, sem unir linhas entre as origens:

- `emendas-cgu`: autor, código e número da emenda, localidade textual e valores reportados pela CGU.
- `emendas-tesouro`: repasses com UF, código IBGE, OB, favorecido e valor. A origem não identifica o parlamentar nem publica uma coluna separada de tipo de emenda; `tipo_emenda` fica nulo em vez de ser inferido de `Nome Emenda`.

Para publicar os dois contratos e aplicar filtro anual:

```bash
docker compose run --rm refino refinar bronze emendas-cgu
docker compose run --rm refino refinar bronze emendas-tesouro
docker compose run --rm refino refinar silver emendas --filtro ano=2025
```

Para filtrar por UF, use o contrato do Tesouro, que tem coluna estruturada para isso:

```bash
docker compose run --rm refino refinar silver emendas-tesouro --filtro ano=2025 --filtro uf=MG
```

O comando combinado `silver emendas --filtro uf=MG` processa somente o Tesouro e informa que a CGU foi ignorada, pois `localidadeDoGasto` não fornece uma UF estruturada confiável. Recortes são publicados em recursos próprios sob `filtros/`, sem substituir a publicação integral. A análise do snapshot do Tesouro atualizado em 04/11/2026 encontrou 16.899 OBs repetidas; por isso, as ocorrências são preservadas e OB não é usada como chave ou para remover linhas. Os detalhes e limites dessa validação estão na [pesquisa das fontes](pesquisa-emendas.md#tesouro-transparente-repasses-mensais).
