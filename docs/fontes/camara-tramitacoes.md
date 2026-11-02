# Fonte `camara-tramitacoes` — histórico de tramitações

A fonte `camara-tramitacoes` ingere os passos oficiais de tramitação das proposições anuais da Câmara, preservando as respostas JSON sem transformação na Raw ([ADR 0014](../adr/0014-raw-opaca-e-refino-em-estagio-separado.md)). Ela é separada de `camara-votacoes` e executada sob demanda por causa do grande número de consultas.

## Endpoints e cobertura

Para cada ano desde 2001, a fonte percorre `GET /api/v2/proposicoes?ano=AAAA&itens=100`, seguindo os links `next` da API, e consulta `GET /api/v2/proposicoes/{id}/tramitacoes` para cada ID listado. A lista é paginada; a OpenAPI limita listagens a 100 itens por requisição. O endpoint de tramitações não declara parâmetros próprios de paginação. Cada resposta consultada vira uma parte bruta no manifesto `raw/camara/tramitacoes/ano=AAAA/`; as partes das tramitações têm URL com o ID da proposição.

Uma consulta de referência em 2026 encontrou 282 páginas de proposições para 2024 (até 28.200 IDs), antes das consultas individuais de histórico. O volume total depende do ano e do tamanho de cada histórico; comece com um único ano para avaliar tempo e armazenamento. Cada tarefa falha sem publicar manifesto parcial se ultrapassar 100.000 respostas.

```bash
# Escolher anos específicos para controlar o volume
todeolho executar camara-tramitacoes --filtro ano=2024

# Pode gerar centenas de milhares de requisições ao percorrer toda a cobertura
todeolho executar camara-tramitacoes
```

## Arquivamento e limites do dado

No endpoint de tramitações, cada item contém `dataHora`, `siglaOrgao`, `descricaoTramitacao`, `descricaoSituacao`, `codSituacao` e `despacho`, entre outros campos. A data e o órgão descrevem o passo; a situação pode ser uma fotografia do estado da proposição. Preserve todos os campos e textos e não conclua que uma situação final, sozinha, representa um evento de arquivamento.

Os exemplos abaixo foram consultados diretamente na API e nas fichas oficiais em 2 de novembro de 2026:

- `PL 760/2020` (ID `2239677`): a resposta da API contém um passo com `dataHora=2023-06-01T00:00`, `siglaOrgao=MESA`, `descricaoTramitacao=Arquivamento`, `descricaoSituacao=Arquivada` e `despacho=Arquivado`.
- `REQ 3128/2026` (ID `2627570`): o estado é `Arquivada`, mas o último passo publicado é `Deferido o requerimento de retirada pelo(a) autor(a)` em `2026-07-08`; não há passo cujo campo `descricaoTramitacao` identifique um evento de arquivamento.

Os exemplos demonstram que situação e evento de arquivamento devem ser interpretados separadamente. A API não garante um campo que identifique a liderança ou pessoa responsável; só atribua autoria quando ela estiver explicitamente na resposta. O histórico é provisório segundo a própria OpenAPI, e a ausência de um passo nomeado como arquivamento não significa que a proposição esteja em tramitação.

## Rate limit e execução

A OpenAPI e a FAQ consultadas não publicam uma quota fixa de requisições por segundo/minuto. Para reduzir pressão durante o fan-out, o cliente aplica um ritmo **local** compartilhado de até 5 requisições por segundo ao host `dadosabertos.camara.leg.br`; esse valor é uma escolha operacional do projeto, não um limite oficial da Câmara. A camada HTTP repete respostas transitórias, incluindo HTTP 429, com backoff limitado. Mesmo assim, a ingestão pode levar bastante tempo e deve ser selecionada por ano.

## Referências oficiais

- [OpenAPI dos Dados Abertos da Câmara](https://dadosabertos.camara.leg.br/api/v2/api-docs): contrato do endpoint, filtros por ano, paginação e limite geral de 100 itens.
- [Tutorial de resultados paginados](https://dadosabertos.camara.leg.br/howtouse/2017-05-16-js-resultados-paginados.html).
- [FAQ dos Dados Abertos](https://dadosabertos.camara.leg.br/faq/faq-home.html).
- [Ficha oficial do PL 760/2020](https://www.camara.leg.br/proposicoesWeb/fichadetramitacao?idProposicao=2239677) e [resposta JSON de tramitações](https://dadosabertos.camara.leg.br/api/v2/proposicoes/2239677/tramitacoes).
- [Ficha oficial do REQ 3128/2026](https://www.camara.leg.br/proposicoesWeb/fichadetramitacao?idProposicao=2627570) e [resposta JSON de tramitações](https://dadosabertos.camara.leg.br/api/v2/proposicoes/2627570/tramitacoes).
