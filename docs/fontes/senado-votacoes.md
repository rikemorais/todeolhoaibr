# Fonte `senado-votacoes` — matérias e votações nominais

Votações do **Senado Federal** com o **voto nominal de cada senador** e a matéria votada — o par, no Senado, da [`camara-votacoes`](camara-votacoes.md). "Quais projetos foram votados e quem votou em quê". Implementa a [issue #87](https://github.com/rikemorais/todeolhoaibr/issues/87).

## Por que uma consulta por ano, sem fan-out

O serviço `GET /dadosabertos/votacao?dataInicio=&dataFim=` devolve um **array de votações com `votos[]` embutido** — o voto nominal de cada senador já vem junto, além da matéria (`identificacao`, `ementa`, `codigoMateria`). Basta **uma requisição por ano**; não é preciso o fan-out por id de votação que a antiga API por votação exigiria. A API legada de plenário por data foi desativada em 01/02/2026.

## O que é ingerido

Um array JSON por ano, de **1991** (primeiro ano com votação registrada; 1990 e antes voltam vazios) até o ano atual. A resposta é guardada **opaca** na Raw ([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)) — a ingestão não interpreta o conteúdo.

Cada votação traz, entre outros: `codigoSessaoVotacao`, `dataSessao`, `descricaoVotacao`, a matéria (`codigoMateria`, `identificacao`, `ementa`) e `votos[]` (por senador: `codigoParlamentar`, `nomeParlamentar` e o voto).

Documentação: <https://www12.senado.leg.br/dados-abertos/legislativo/plenario/votacoes-nominais>.

## Layout na Raw

```text
raw/senado/votacoes/ano=AAAA/_ultima_versao.json                                     # ponteiro do vigente
raw/senado/votacoes/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.json
raw/senado/votacoes/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

O `_ultima_versao.json` é o **ponteiro** para o manifesto da execução **vigente** de cada ano ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)).

## Fonte separada, `sob_demanda`

São dezenas de anos (1991→atual). Como a [`camara-votacoes`](camara-votacoes.md), é uma **fonte própria com `sob_demanda = true`**, para **não entrar no `executar --todas`** (é um backfill anual). O dado mora em `raw/senado/votacoes/`.

## Como rodar

```bash
# Todos os anos (a fonte é argumento posicional)
todeolho executar senado-votacoes

# Só um ano
todeolho executar senado-votacoes --filtro ano=2024
```

## Idempotência

Reusa a pré-checagem ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)). O serviço **não envia `ETag`/`Last-Modified`**, então não há requisição condicional (`If-None-Match`): a cada rodada o corpo é baixado e comparado por `sha256` ao da rodada anterior — idêntico, é reaproveitado (a execução reporta "sem alterações na origem"); diferente, é reingerido.

## Fora de escopo desta camada (fica no refino)

- **Cruzar matéria ↔ votação ↔ voto ↔ senador** numa base consultável (`silver`/`gold`) — ver [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md).
- **Catálogo completo de matérias** (inclusive as não votadas) não entra aqui: esta fonte cobre as **matérias votadas** (as que aparecem nas votações). Um acervo de todas as matérias caberia como fonte separada.
- **Análises** (adesão, presença, governismo) são do refino/consulta.
