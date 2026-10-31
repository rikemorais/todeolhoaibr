# 0011. Versionamento semântico e Conventional Commits

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

O projeto vai para o GitHub e deve receber contribuições. O histórico precisa contar o que mudou e permitir reverter uma mudança sem levar outras
junto, e quem usa o projeto precisa saber se uma nova versão pode quebrar o que já funciona.

## Decisão

**Commits**

- Seguimos [Conventional Commits 1.0.0](https://www.conventionalcommits.org/pt-br/v1.0.0/): `tipo(escopo): descrição`.
- Tipos, por serem parte da especificação, ficam em inglês: `feat`, `fix`, `docs`, `build`, `chore`, `refactor`, `test`. Escopo e descrição ficam em
  português, conforme o [ADR 0001](0001-registrar-decisoes-com-adrs.md).
- Os escopos são os crates e áreas do projeto: `nucleo`, `fontes`, `infraestrutura`, `painel`, `cli`, `docker`, `compose`, `adr`.
- Cada commit é atômico: uma única mudança coerente, que compila e passa nos testes sozinha.
- Mudanças incompatíveis são marcadas com `!` e o rodapé `BREAKING CHANGE:`.

**Versões**

- Seguimos [SemVer 2.0.0](https://semver.org/lang/pt-BR/). A versão fica em `workspace.package`, no `Cargo.toml`, e é a mesma para todos os crates.
- O projeto está em desenvolvimento inicial (`0.y.z`). Nessa fase, a API pode mudar: mudança incompatível ou funcionalidade nova sobe `y`, e correção sobe `z`.
- A versão `1.0.0` será lançada quando a ingestão, a transformação e a apresentação estiverem estáveis para uso público.
- Cada versão vira uma tag `vX.Y.Z` em `main` e uma seção no `CHANGELOG.md`.

**Contrato público**

Para efeito do SemVer, o contrato público é o que quem usa o projeto vê: os comandos da CLI, as variáveis do `.env` e o layout da camada Raw. Os crates internos
não são publicados e podem mudar livremente.

**Integração**

Pull requests entram em `main` por merge ou rebase, nunca por squash, para preservar os commits atômicos.

## Consequências

- O `CHANGELOG.md` é escrito à mão, porque o histórico inicial organiza o código por camadas e não serve de lista de novidades para quem usa o projeto.
- Não há verificação automática das mensagens de commit. Ela pode entrar quando houver integração contínua.
- O histórico anterior a este ADR foi organizado de uma vez, por camada e funcionalidade, e não na ordem cronológica em que o trabalho aconteceu.

## Alternativas consideradas

- **Commits e tipos totalmente em português:** quebraria as ferramentas que entendem Conventional Commits.
- **Versionar cada crate separadamente:** custo sem benefício, já que os crates não são publicados nem usados fora do monorepo.
- **Squash no merge:** deixa `main` mais curta, mas perde a capacidade de reverter uma mudança isolada.
- **Começar em `1.0.0`:** prometeria uma estabilidade que o projeto ainda não tem, pois a transformação nem foi definida.
