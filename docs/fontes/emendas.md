# Emendas parlamentares

A fonte `emendas` preserva três conjuntos oficiais na Raw, sem transformar os bytes:

| Origem | Recurso Raw | Grão e conteúdo |
| --- | --- | --- |
| Portal da Transparência (CGU) | `cgu/ano=AAAA` | Emenda, autor, localidade de gasto e valores empenhado, liquidado, pago e restos a pagar; páginas JSON por exercício desde 2015 |
| Tesouro Transparente | `repasses/tesouro` | CSV completo de repasses mensais desde janeiro de 2015, com município, UF, código IBGE, ordem bancária, favorecido, categoria e valor |
| Transferegov — Gestão de Parcerias | `transferegov/beneficiario-emenda/ano=AAAA` | Beneficiário/programa por emenda, com autor, município/UF e GND3, GND4 e valor total; API pública paginada, cobertura observada desde 2025 |

Os conjuntos têm grãos e semânticas diferentes. A API CGU reporta valores por emenda e localidade, o Tesouro registra pagamentos por ordem bancária e favorecido, e o Transferegov traz beneficiários associados a programas de emenda. Não associe uma ordem bancária a um autor ou código de emenda sem uma chave documentada pela origem. Nenhum conjunto é deduplicado na Raw.

## Acesso e atualização

O CSV do Tesouro é público e tem cerca de 67 MiB. É baixado como um arquivo integral e atualizado mensalmente. A CGU exige uma chave gratuita da API, obtida pelo [cadastro do Portal da Transparência](https://portaldatransparencia.gov.br/api-de-dados/cadastrar-email). Configure `PORTAL_TRANSPARENCIA_TOKEN` no `.env` para consultar a CGU; o cliente envia o segredo somente ao host `api.portaldatransparencia.gov.br`, no cabeçalho `chave-api-dados`. A origem tem limite configurado de 90 requisições por minuto. As páginas CGU terminam quando a API retorna um array vazio; a ingestão impõe um limite de segurança de páginas. Tesouro e Transferegov são públicos e não exigem esse token.

Para baixar tudo (CSV do Tesouro, Transferegov e anos disponíveis da CGU), rode:

```bash
docker compose run --rm --build ingestao executar emendas
```

A fonte é sob demanda e fica fora de `executar --todas`. Para selecionar um ano CGU e Transferegov:

```bash
docker compose run --rm --build ingestao executar emendas --filtro ano=2025
```

Para baixar somente os beneficiários do Transferegov, sem token:

```bash
docker compose run --rm --build ingestao executar emendas --filtro origem=transferegov
```

Para baixar somente o CSV integral do Tesouro:

```bash
docker compose run --rm --build ingestao executar emendas --filtro origem=tesouro
```

`--filtro ano=...` seleciona as tarefas CGU e Transferegov; o CSV do Tesouro é um único arquivo histórico, portanto não é fragmentado por ano ou UF na Raw. A API CGU não oferece filtro de UF. O CSV do Tesouro contém UF e código IBGE, e as respostas CGU e Transferegov contêm município como texto. A API Transferegov observada cobre exercícios desde 2025 e devolve linhas no grão beneficiário/programa; nenhuma linha é deduplicada por município, parlamentar ou número de emenda. Os filtros Silver já existentes aplicam-se à CGU e ao Tesouro; Transferegov ainda não tem contrato Silver enquanto o cruzamento IBGE e a semântica dos valores são validados. A Raw preserva respostas completas conforme o [ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md).

## Campos e idempotência

O CSV do Tesouro vem delimitado por ponto e vírgula em Latin-1. Entre os campos originais estão `Código IBGE`, `UF`, `Ano`, `Mês`, `OB`, `CNPJ do Favorecido` e `Valor`. `Data` vem como serial numérico do Excel; `Mês` vem por extenso em português e a Silver converte o nome para o número correspondente. O CSV integral permanece como recebido; decodificação e conversão ficam para as camadas seguintes.

Na CGU, os recursos são separados por ano, com páginas dentro do manifesto `cgu/ano=AAAA`. O `codigoEmenda` identifica a emenda, mas pode haver mais de uma localidade por emenda; ele não é tratado como chave única da linha. Na Raw, os manifestos e validadores HTTP tornam cada página idempotente e permitem reaproveitar páginas que não mudaram. Repetições no conteúdo são preservadas.

Consulte a [pesquisa das fontes e seus limites](pesquisa-emendas.md), que registra os endpoints oficiais, os campos e as limitações de filtragem e relacionamento.

## Refino Silver

O Bronze decodifica `emendas` para `emendas-cgu`, `emendas-tesouro` e `emendas-transferegov`. A Silver publica contratos independentes, sem unir linhas entre essas origens:

- `emendas-cgu`: autor, código e número da emenda, localidade textual e valores reportados pela CGU.
- `emendas-tesouro`: repasses com UF, código IBGE, OB, favorecido e valor. A origem não identifica o parlamentar nem publica uma coluna separada de tipo de emenda; `tipo_emenda` fica nulo em vez de ser inferido de `Nome Emenda`.
- `emendas-transferegov`: uma linha por beneficiário-programa, com autor, emenda, município e UF textuais, valores GND3/GND4/total informados pela API e indicações preservadas como JSON. O contrato não agrega as linhas nem classifica os valores como pagamentos executados.

Para gerar Bronze nas três origens e publicar os contratos Silver já disponíveis:

```bash
docker compose run --rm refino refinar bronze emendas-cgu
docker compose run --rm refino refinar bronze emendas-tesouro
docker compose run --rm refino refinar bronze emendas-transferegov
docker compose run --rm refino refinar silver emendas --filtro ano=2025
docker compose run --rm refino refinar silver emendas-transferegov --filtro ano=2025 --filtro uf=MG
```

O contrato `emendas-transferegov-beneficiario-programa-v1` mantém a localidade como município/UF textual porque a API não fornece código IBGE. Na captura de 15/11/2026, 78.740 de 79.044 registros corresponderam ao IBGE após normalização de caixa, acentos e pontuação; as 304 linhas restantes usam 22 grafias ainda sem código confirmado. A coluna `municipio` não deve ser usada como chave IBGE. Os valores são publicados como `valor_gnd3`, `valor_gnd4` e `valor_total_emenda`, conforme os campos da API, sem afirmar que representam pagamentos executados. Não some nem deduplique as linhas: preserve a granularidade `id_beneficiario_programa` + `id_programa`.

Para filtrar por UF, use o contrato do Tesouro, que tem coluna estruturada para isso:

```bash
docker compose run --rm refino refinar silver emendas-tesouro --filtro ano=2025 --filtro uf=MG
```

O comando combinado `silver emendas --filtro uf=MG` processa somente o Tesouro e informa que a CGU foi ignorada, pois `localidadeDoGasto` não fornece uma UF estruturada confiável. Recortes são publicados em recursos próprios sob `filtros/`, sem substituir a publicação integral. A análise do snapshot do Tesouro atualizado em 04/11/2026 encontrou 16.899 OBs repetidas; por isso, as ocorrências são preservadas e OB não é usada como chave ou para remover linhas. Os detalhes e limites dessa validação estão na [pesquisa das fontes](pesquisa-emendas.md#tesouro-transparente-repasses-mensais).
