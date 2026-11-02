# Fonte `camara-votacoes` — proposições e votações nominais

Histórico legislativo da **Câmara dos Deputados**: proposições (PL, PEC, etc.), as votações e o **voto nominal de cada deputado** — "quais projetos foram votados e quem votou em quê". Implementa a [issue #86](https://github.com/rikemorais/todeolhoaibr/issues/86).

## Por que arquivos em lote, e não a API `/votacoes/{id}/votos`

O voto nominal, na API REST, só sai por votação (`/votacoes/{id}/votos`), o que exigiria um **fan-out por id** — incompatível com o modelo de tarefa linear da ingestão. A Câmara publica os mesmos dados como **arquivos anuais em lote** em `/arquivos/{conjunto}/csv/{conjunto}-{ano}.csv`, então cada `(conjunto, ano)` vira **um download** ([`TarefaDeArquivo`]), sem fan-out. É o mesmo padrão da [`camara-despesas`](camara-despesas.md).

## O que é ingerido

Um CSV por **conjunto** e por **ano**, baixado como **arquivo grande** (em fluxo) e gravado **opaco** na Raw — a ingestão não interpreta o conteúdo ([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)).

| Conjunto | O que traz | Desde |
| --- | --- | --- |
| `proposicoes` | Proposições (PL, PEC, PLP…): tipo, ementa, situação | 2001 |
| `proposicoesAutores` | Autoria das proposições | 2001 |
| `votacoes` | Votações: objeto, data, placar | 2001 |
| **`votacoesVotos`** | **Voto nominal de cada deputado** (Sim/Não/Obstrução/Abstenção) | 2001 |
| `votacoesObjetos` | Objetos de cada votação | 2001 |
| `votacoesProposicoes` | Vínculo votação ↔ proposição | 2001 |
| `votacoesOrientacoes` | Orientação de bancada por votação | 2003 |

O escopo começa em **2001**, primeiro ano com voto nominal. `proposicoes`/`votacoes` têm arquivos mais antigos (desde ~1990), mas sem os votos não servem ao objetivo desta fonte.

Documentação dos campos (Câmara): <https://dadosabertos.camara.leg.br/howtouse/2020-02-28-dados-votacoes.html>.

## Layout na Raw

Mesmo sendo uma fonte separada no catálogo, os arquivos ficam sob a fonte `camara`:

```text
raw/camara/votacoes/<conjunto>/ano=AAAA/_ultima_versao.json                                    # ponteiro do vigente
raw/camara/votacoes/<conjunto>/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-00001.csv
raw/camara/votacoes/<conjunto>/ano=AAAA/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

O `_ultima_versao.json` é o **ponteiro** que aponta para o manifesto da execução **vigente** de cada `(conjunto, ano)`, por onde um leitor a jusante começa ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)).

## Fonte separada, `sob_demanda`

São dezenas de arquivos anuais (7 conjuntos × ~25 anos). Como na [`camara-despesas`](camara-despesas.md), é uma **fonte própria com `sob_demanda = true`** para **não entrar no `executar --todas`**. O dado, ainda assim, mora em `raw/camara/votacoes/`.

## Como rodar

```bash
# Tudo (todos os conjuntos, todos os anos — a fonte é argumento posicional)
todeolho executar camara-votacoes

# Só os votos nominais de um ano
todeolho executar camara-votacoes --filtro conjunto=votacoesVotos --filtro ano=2024

# Um ano inteiro (todos os conjuntos daquele ano)
todeolho executar camara-votacoes --filtro ano=2024
```

Cada tarefa tem as etiquetas `ano` e `conjunto`, que o `--filtro` combina (todas precisam casar).

## Idempotência

Reusa a pré-checagem por versão/tamanho ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)): a origem envia um `ETag` (conferido no arquivo real), usado como **validador autoritativo** — um arquivo revisado ganha ETag novo e é reingerido; um inalterado é reaproveitado (a execução reporta "sem alterações na origem").

## Fora de escopo desta camada (fica no refino)

- **Cruzar proposição ↔ votação ↔ voto ↔ deputado** numa base consultável (camada `silver`/`gold`) — ver [ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md). A Raw é opaca, então aqui só se garante o arquivo íntegro e versionado.
- **Análises** (adesão partidária, governismo, presença) são do refino/consulta, não da ingestão.
