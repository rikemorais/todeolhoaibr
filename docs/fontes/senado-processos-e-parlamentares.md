# Fontes `senado-processos` e `senado-parlamentares`

Estas fontes complementam [`senado-votacoes`](senado-votacoes.md) e implementam a [issue #94](https://github.com/rikemorais/todeolhoaibr/issues/94): catalogam processos legislativos que não chegaram a votação e os parlamentares por legislatura, para que o refino possa relacioná-los aos votos nominais.

## Processos legislativos

`senado-processos` consulta `GET /dadosabertos/processo?ano=AAAA`, uma vez por ano, de **1826** (instalação do Senado) ao ano corrente. A listagem inclui identificação da matéria, ementa, autoria, datas, situação e link para o documento quando publicados pela API. A API também oferece filtros de tipo (`sigla`) e consultas de detalhe (`/dadosabertos/processo/{id}`); a ingestão anual preserva a listagem completa para que matérias nunca votadas também sejam incluídas.

```bash
# Todos os anos
todeolho executar senado-processos

# Um ano
todeolho executar senado-processos --filtro ano=2024
```

Os dados ficam em `raw/senado/processos/ano=AAAA/`.

## Detalhes e histórico de tramitação

`senado-processos-detalhes` consulta a listagem de um ano e, em sequência, busca `GET /dadosabertos/processo/{id}` para cada processo retornado. A resposta de cada URL vira uma parte bruta independente no manifesto do ano; assim ficam preservados os campos de situação, informes legislativos e movimentações sem transformá-los. A listagem inicial também fica no mesmo recurso para registrar quais IDs foram percorridos.

```bash
# Todos os anos — pode gerar muitas requisições
todeolho executar senado-processos-detalhes

# Selecionar anos para controlar o volume
todeolho executar senado-processos-detalhes --filtro ano=2024
```

Os dados ficam em `raw/senado/processos-detalhes/ano=AAAA/`. Cada tarefa limita uma rodada a 10.000 respostas (a listagem mais até 9.999 detalhes); se um ano exceder esse limite, a tarefa falha sem publicar um manifesto parcial. O limite oficial da API é de 10 requisições por segundo e é compartilhado entre as fontes do Senado.

O detalhe informa a situação atual e sua data (`dataSituacaoAtual`), além de informes com datas e descrições. A cobertura foi conferida em dois exemplos da API: o processo `8237825` (`PL 823/2022`) tem situação atual `RTPA` em `2023-07-12` e um informe legislativo datado cuja descrição registra que a Presidência deferiu a retirada e encaminhou o projeto ao Arquivo; o processo `8614284` (`PL 1/2024`) aparece como `PRONTPAUT` e tramitando, sem evento de arquivamento no detalhe consultado. Assim, situação atual e evento de arquivamento são informações distintas.

A API não garante uma coluna estruturada para a data de arquivamento ou para a pessoa responsável. Preserve datas e descrições dos informes/movimentações; só atribua órgão ou responsável quando estiver explicitamente presente na resposta. Os exemplos validam o formato disponível, não garantem que todos os arquivamentos tenham evento estruturado ou identificável.

## Parlamentares

`senado-parlamentares` consulta `GET /dadosabertos/senador/lista/legislatura/{legislatura}`, uma vez por legislatura, da **49ª** (1991–1995) até a legislatura atual. A resposta contém a identificação parlamentar, incluindo partido quando informado, e os mandatos com UF e períodos/legislaturas. A API mantém rotas separadas para detalhes, mandatos e filiações (`/senador/{codigo}`, `/senador/{codigo}/mandatos` e `/senador/{codigo}/filiacoes`); esta fonte usa a listagem por legislatura, que já fornece o recorte de composição necessário aos dados de votação.

```bash
# Todas as legislaturas
todeolho executar senado-parlamentares

# Uma legislatura
todeolho executar senado-parlamentares --filtro legislatura=57
```

Os dados ficam em `raw/senado/parlamentares/legislatura=NN/`.

## Dados brutos e execução

As fontes são `sob_demanda` por cobrirem um histórico extenso. Suas respostas JSON ficam opacas na camada Raw, sem normalização ou cruzamento nesta etapa ([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)); relacionar matérias, parlamentares, mandatos e votos pertence ao refino ([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)).

```bash
todeolho executar senado-processos --filtro ano=2024
todeolho executar senado-parlamentares --filtro legislatura=57
```

A ingestão segue a pré-checagem idempotente documentada no [ADR 0012](../adr/0012-pre-checagem-idempotente.md). A API pública não exige autenticação. O limite documentado é de 10 requisições por segundo; respostas HTTP 429 e 503 são tratadas pelo mecanismo comum de erros HTTP da ingestão.

Referência primária: [OpenAPI dos Dados Abertos Legislativos do Senado](https://legis.senado.leg.br/dadosabertos/v3/api-docs) e [Swagger UI](https://legis.senado.leg.br/dadosabertos/docs/).
