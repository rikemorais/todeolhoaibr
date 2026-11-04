# 0019. Disparo do bronze por reconciliação agendada

- **Status:** Aceito
- **Data:** 2026-11-04

## Contexto

O bronze ([ADR 0015](0015-arquitetura-de-dados-em-medalhao.md)) decodifica a `raw` para Parquet e é **idempotente**: o ponteiro do bronze guarda o validador da versão do `raw`, então reprocessar só gera trabalho quando o `raw` mudou ([ADR 0012](0012-pre-checagem-idempotente.md)). Falta um jeito de **disparar o bronze sempre que algo mudar na `raw`**, sem depender de alguém rodar o comando à mão.

O versitygw ([ADR 0005](0005-armazenamento-s3-com-versitygw.md)) não emite notificações de evento consumíveis de forma simples, e a ingestão é avulsa (menu, `executar`, ou cargas sob demanda), então nem toda mudança na `raw` vem de uma rodada de ingestão que poderíamos encadear.

## Decisão

Disparar o bronze por **reconciliação agendada**: um serviço roda `refinar bronze --todas --intervalo <segundos>` em laço. A cada passada, compara a versão vigente do `raw` com o ponteiro do bronze (por recurso) e **só reprocessa o que mudou**; quando nada mudou, a passada é quase instantânea e **não registra execução no painel** (para não poluir o histórico).

- O laço vive **no próprio binário** (`--intervalo`), não num `sh` de fora, porque a imagem da aplicação é distroless (sem shell) ([ADR 0007](0007-docker-para-maquinas-modestas.md)).
- Um serviço `agendador` no `docker compose` sobe por padrão, com `mem_limit`; o intervalo é configurável (`TODEOLHO_REFINO_INTERVALO`, padrão 1h).
- `refinar bronze --todas` (sem `--intervalo`) continua servindo para uma reconciliação manual única, que registra tudo no painel (inclusive as reaproveitadas).

## Consequências

- "Sempre que mudar" vira **convergência eventual**: o bronze fica, no pior caso, um intervalo atrás do `raw`. Para a natureza dos dados (cargas públicas periódicas), é adequado.
- Reusa a idempotência que já existe: a reconciliação é barata e segura de repetir.
- O painel fica limpo — só aparecem as passadas em que houve trabalho.
- O catálogo de refino cresce por fonte; `--todas` passa a cobrir cada fonte nova automaticamente.

## Alternativas consideradas

- **Notificações de evento do object storage (S3 events):** seria o disparo mais "imediato", mas o versitygw não as expõe de forma simples, e exigiria uma fila/worker a mais — peso contra o [ADR 0007](0007-docker-para-maquinas-modestas.md). Rejeitada por ora.
- **Encadear o bronze após cada ingestão:** cobre o fluxo normal, mas não pega mudanças na `raw` que não venham de uma rodada de ingestão (reingestão manual, correção). Fica como complemento possível, não como o mecanismo principal.
- **Timer do host (systemd):** funciona e integra com a operação atual, mas fica fora do repositório (não portável). O serviço no compose é portável e sobe junto com o stack.
