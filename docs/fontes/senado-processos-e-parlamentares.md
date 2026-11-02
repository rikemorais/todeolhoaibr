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

As duas fontes são `sob_demanda` por cobrirem um histórico extenso. Suas respostas JSON ficam opacas na camada Raw, sem normalização ou cruzamento nesta etapa ([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)); relacionar matérias, parlamentares, mandatos e votos pertence ao refino ([ADR 0015](../adr/0015-arquitetura-de-dados-em-medalhao.md)).

```bash
todeolho executar senado-processos --filtro ano=2024
todeolho executar senado-parlamentares --filtro legislatura=57
```

A ingestão segue a pré-checagem idempotente documentada no [ADR 0012](../adr/0012-pre-checagem-idempotente.md). A API pública não exige autenticação. O limite documentado é de 10 requisições por segundo; respostas HTTP 429 e 503 são tratadas pelo mecanismo comum de erros HTTP da ingestão.

Referência primária: [OpenAPI dos Dados Abertos Legislativos do Senado](https://legis.senado.leg.br/dadosabertos/v3/api-docs) e [Swagger UI](https://legis.senado.leg.br/dadosabertos/docs/).
