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

**Perfil de acesso declarado pela fonte.** `FonteDeDados` ganha um método opcional `perfil_http()` (padrão: anônimo, sem limite), onde a fonte declara **que precisa de credencial** (identificada por nome, p.ex. `PORTAL_TRANSPARENCIA_TOKEN`) e **qual o limite de taxa** (requisições por intervalo). É aditivo: com implementação padrão, nenhuma fonte existente muda.

**Decoradores na infraestrutura.** A infraestrutura passa a oferecer, sobre o `ClienteHttp` base:

- um decorador que **injeta o cabeçalho de autenticação**, com o segredo lido do ambiente/config;
- um decorador que **aplica throttle por host** conforme o limite declarado.

Ambos implementam o próprio `ClienteHttp`, então o resto do sistema não percebe a diferença (composição, não herança).

**Raiz de composição monta o cliente certo por fonte.** A CLI/infra lê os segredos do ambiente (variáveis de ambiente/arquivo de config fora do versionamento), monta o cliente decorado conforme o `perfil_http()` da fonte e o injeta nas tarefas daquela fonte. O domínio nunca vê o segredo.

**Falha de autenticação é fatal para a fonte, nunca silenciosa.** Um 401/403 interrompe a ingestão daquela fonte com erro claro; não é tratado como "não modificado" nem pulado.

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
- **Biblioteca de rate limit global, única para todas as fontes:** um limite só não serve, porque cada API tem o seu; o throttle precisa ser por host/fonte. Rejeitada em favor do limite declarado no `perfil_http()`.
