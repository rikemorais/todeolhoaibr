# 0013. Autenticação e limite de taxa no cliente HTTP

- **Status:** Proposto
- **Data:** 2026-11-01

## Contexto

Até agora toda fonte HTTP é anônima: `ClienteHttp` ([`crates/nucleo/src/portas.rs`](../../crates/nucleo/src/portas.rs)) só faz GET, GET condicional, `HEAD` e download em fluxo, sem noção de credencial nem de limite de requisições. Isso basta para IBGE, Câmara e TSE.

O crescimento do catálogo quebra essa premissa. Fontes como o Portal da Transparência (CGU) exigem **token pessoal** no cabeçalho e impõem **limite de requisições**; outras APIs podem exigir chave. Ao mesmo tempo:

- a imensa maioria das fontes continua anônima e não pode ficar mais complicada por causa de poucas autenticadas;
- segredos não podem ser embutidos no código, nem vazar para a camada Raw, para o `_ultima_versao.json`, para o manifesto ou para os logs;
- estourar o limite de uma API derruba a ingestão daquela fonte e, se for agressivo, pode nos bloquear.

## Decisão

Tratar autenticação e limite de taxa como **preocupações de borda, resolvidas na infraestrutura**, sem poluir o porto nem o caminho anônimo.

**O porto continua limpo.** `ClienteHttp` não ganha parâmetro de credencial em cada chamada. Autenticar em toda chamada obrigaria toda fonte anônima a lidar com o assunto (viola ISP).

**Perfil de acesso declarado pela fonte.** `FonteDeDados` ganha um método opcional `perfil_http()` (padrão: anônimo, sem limite). A fonte declara:

- **a credencial**, pelo **nome da variável de ambiente** (p.ex. `PORTAL_TRANSPARENCIA_TOKEN`), **o nome do cabeçalho** e **o esquema** (p.ex. cabeçalho `chave-api-dados`, ou `Authorization: Bearer <token>`) — não só o nome, para que a infraestrutura monte exatamente a requisição que a API exige;
- **o limite de taxa** (requisições por intervalo) e **o host** a que ele se aplica.

É aditivo: com a implementação padrão, nenhuma fonte existente muda.

**A autenticação é configurada no adapter HTTP, não num decorador sobre o porto.** O porto `ClienteHttp` só recebe URL (e `aceita` nos GETs), então um invólucro que delega aos seus métodos **não teria como injetar cabeçalho**. Por isso a credencial é aplicada **dentro do adapter de infraestrutura** (o cliente `reqwest`), configurado com os cabeçalhos padrão da fonte; o rate limit é uma camada (`tower`) no mesmo adapter. Cada fonte autenticada recebe um adapter configurado com o seu perfil; o resto do sistema continua vendo apenas `ClienteHttp`.

**Ordem explícita: throttle por dentro, retentativa por fora.** O throttle é a camada **mais interna**, de modo que **toda tentativa HTTP — inclusive as novas tentativas** do `ClienteHttpComRetentativas` — passe por ele. A retentativa envolve o throttle, nunca o contrário, senão os reenvios furariam o limite declarado.

**O limite é por host e o estado é compartilhado.** O throttle é chaveado pelo **host**, com estado **compartilhado entre todos os clientes que acessam o mesmo host**, para que fontes distintas que batem na mesma API não somem tráfego acima do limite e tomem 429.

**A credencial não vaza em redirecionamento.** O adapter autenticado **não segue redirect para outro host carregando o cabeçalho de credencial**: ou remove o cabeçalho antes de seguir para host diferente, ou não segue redirect cross-host automaticamente. O segredo só vai ao host para o qual foi configurado.

**Raiz de composição monta o cliente certo por fonte.** A CLI/infra lê os segredos do ambiente (variáveis de ambiente/arquivo de config fora do versionamento), monta o adapter configurado conforme o `perfil_http()` da fonte e o injeta nas tarefas daquela fonte. O domínio nunca vê o segredo.

**Falha de autenticação não é silenciosa.** Um 401/403 marca a tarefa como **falha, com erro claro** — nunca é tratado como "não modificado" nem pulado. O executor atual segue com as demais tarefas da DAG e registra o erro (ver [ADR 0003](0003-dagron-core-com-executor-proprio.md)), então o efeito imediato é *a tarefa daquele recurso* parar, não a fonte inteira. Abortar toda a DAG da fonte no primeiro 401/403 é uma evolução possível do executor, registrada como pendência, não prometida aqui.

## Consequências

- Fontes anônimas não mudam em nada: `perfil_http()` tem padrão anônimo e o `ClienteHttp` base continua o mesmo.
- O porto ganha, no máximo, um método opcional em `FonteDeDados` (com padrão), preservando o Open/Closed do [ADR 0004](0004-factory-method-para-fontes.md).
- Passa a existir um mecanismo de configuração de segredos por ambiente; é preciso documentar a origem de cada token e garantir que não entre no Git, na Raw nem nos logs.
- O throttle **desacelera de propósito** algumas fontes; o executor já é assíncrono e com novas tentativas, então tolera isso, mas a duração da ingestão dessas fontes cresce.
- Rotação/expiração de token vira operação manual de atualizar a variável de ambiente; não há renovação automática nesta decisão.
- Mantém a disciplina do [ADR 0002](0002-monorepo-em-camadas.md): o segredo e o throttle vivem na infraestrutura, o domínio só declara a necessidade.

## Alternativas consideradas

- **Cabeçalho de credencial em cada chamada de `ClienteHttp`:** simples de ligar, mas empurra o assunto para toda fonte anônima e espalha `Option<token>` pelo porto inteiro. Rejeitada por violar ISP e sujar o caminho comum.
- **Ler o token dentro de cada módulo de fonte:** traz segredo e detalhe de infraestrutura para o lado do domínio (`fontes`) e duplica a lógica de auth/throttle em cada fonte. Rejeitada por acoplamento e repetição.
- **Proxy reverso externo injetando token e limitando taxa:** tira o assunto do código, mas adiciona um componente de operação e contraria o [ADR 0007](0007-docker-para-maquinas-modestas.md) (Docker como única dependência, máquinas modestas). Rejeitada por ora; pode ser reconsiderada se o número de fontes autenticadas crescer muito.
- **Biblioteca de rate limit global, única para todas as fontes:** um limite só não serve, porque cada API tem o seu; o throttle precisa ser **por host, com estado compartilhado** entre as fontes que batem no mesmo host. Rejeitada em favor do limite declarado no `perfil_http()`.
- **Decorador puro sobre `ClienteHttp` para injetar o cabeçalho:** não funciona, porque o porto não aceita cabeçalho; o invólucro teria de reimplementar o cliente. Por isso a credencial é configurada no adapter, não como decorador do porto.
