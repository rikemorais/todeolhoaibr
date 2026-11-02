# Fonte `camara-tramitacoes` — histórico de tramitações

A fonte `camara-tramitacoes` ingere os passos oficiais de tramitação das proposições da Câmara, preservando o JSON sem transformação na Raw ([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)). É separada de `camara` e executada **sob demanda**, com o dado em `raw/camara/tramitacoes/`.

## Endpoints e cobertura

Em vez de percorrer a API REST proposição a proposição (`/proposicoes/{id}/tramitacoes`, um fan-out de centenas de milhares de requisições), a fonte baixa os **arquivos anuais em lote** dos Dados Abertos, um por ano desde 2001, via `TarefaDeArquivo`:

```text
https://dadosabertos.camara.leg.br/arquivos/proposicoesTramitacoes/json/proposicoesTramitacoes-{ano}.json
```

Cada arquivo traz todos os passos de tramitação das proposições daquele ano num único JSON `{ "dados": [ ... ] }` — o de 2001, por exemplo, tem ~31 mil passos em ~16 MB. Isso troca ~10 mil requisições por ano por **um download**, e a cobertura completa (2001 até o ano atual) por **~26 requisições**.

```bash
# Um ano específico
docker compose run --rm --build ingestao executar camara-tramitacoes --filtro ano=2024

# Cobertura completa: um arquivo por ano, de 2001 até o ano atual
docker compose run --rm --build ingestao executar camara-tramitacoes
```

O ano corrente tolera ausência (`aceitar_ausencia`): no começo de um ano o arquivo pode ainda não existir, e um `404` nele não falha a rodada.

## Layout na Raw

```text
raw/camara/tramitacoes/ano=AAAA/_ultima_versao.json
raw/camara/tramitacoes/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.json
raw/camara/tramitacoes/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

## Idempotência (incremental por ETag)

Os arquivos anuais são regerados diariamente e publicam `ETag`/`Last-Modified`. A pré-checagem condicional `If-None-Match` ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)) usa o `ETag` da rodada anterior: o ano **sem mudança** responde `304` e é pulado, reaproveitando o que já está na Raw. Assim a rodada é **incremental** — só rebaixa os anos cujo arquivo mudou.

## Interpretação do dado (fica no refino)

Cada passo traz `idTramitacao`, `dataHora`, `sequencia`, `uriProposicao`, `siglaOrgao`, `regime`, `descricaoTramitacao`, `codSituacao` e `despacho`, entre outros. A data e o órgão descrevem o passo; a situação (`codSituacao`) pode ser uma fotografia do estado da proposição. Preserve todos os campos e textos e **não** conclua que uma situação final, sozinha, representa um evento de arquivamento.

Exemplos consultados nas fichas oficiais em 2 de novembro de 2026:

- `PL 760/2020` (ID `2239677`): há um passo com `dataHora=2023-06-01T00:00`, `siglaOrgao=MESA`, `descricaoTramitacao=Arquivamento` e `despacho=Arquivado`.
- `REQ 3128/2026` (ID `2627570`): o estado é `Arquivada`, mas o último passo publicado é `Deferido o requerimento de retirada pelo(a) autor(a)` em `2026-07-08`; nenhum passo tem `descricaoTramitacao` que identifique um evento de arquivamento.

Situação e evento de arquivamento devem ser interpretados separadamente. A API não garante um campo de liderança/pessoa responsável; só atribua autoria quando ela estiver explícita. Achatar os passos numa tabela consultável e cruzar com a proposição (por `uriProposicao`) é trabalho do refino ([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)).

## Rate limit e execução

Os arquivos ficam no host `dadosabertos.camara.leg.br`, ao qual o cliente aplica um ritmo **local** de até 5 requisições por segundo (escolha operacional do projeto, não um limite oficial). Com ~26 downloads por rodada completa, o tempo passa a ser dominado pela transferência dos arquivos, não pelo número de requisições.

## Referências oficiais

- [Documentação do dado de tramitações](https://dadosabertos.camara.leg.br/howtouse/2023-12-26-dados-tramitacoes.html).
- [OpenAPI dos Dados Abertos da Câmara](https://dadosabertos.camara.leg.br/api/v2/api-docs).
- [Ficha oficial do PL 760/2020](https://www.camara.leg.br/proposicoesWeb/fichadetramitacao?idProposicao=2239677).
- [Ficha oficial do REQ 3128/2026](https://www.camara.leg.br/proposicoesWeb/fichadetramitacao?idProposicao=2627570).
