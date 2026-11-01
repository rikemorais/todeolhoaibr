# Diagramas

Diagramas interativos (HTML autossuficiente) gerados com [Archify](https://github.com/tt-a1i/archify).
Abra os `.html` no navegador.

| Arquivo | O que mostra |
|---|---|
| `arquitetura-crates.html` | Arquitetura dos crates (nucleo, fontes, infraestrutura, painel, cli) e as fronteiras externas (APIs públicas, S3), no monorepo em camadas da [ADR 0002](../adr/0002-monorepo-em-camadas.md). |
| `fluxo-ingestao.html` | Fluxo da ingestão idempotente: pré-checagem (origem + estado na Raw) → decisão → ingestão → camada Raw ([ADR 0012](../adr/0012-pre-checagem-idempotente.md)). |

Cada diagrama tem seu `.json`-fonte ao lado (a especificação validada). Para regenerar após editar o JSON:

```bash
# instala a skill no projeto (não versionada; ver .gitignore)
npx -y skills@latest add https://archify.si --skill archify --agent claude-code --copy --yes

# renderiza + valida
node .claude/skills/archify/bin/archify.mjs finalize architecture docs/diagramas/arquitetura-crates.json docs/diagramas/arquitetura-crates.html --quality showcase
node .claude/skills/archify/bin/archify.mjs finalize dataflow      docs/diagramas/fluxo-ingestao.json    docs/diagramas/fluxo-ingestao.html    --quality showcase
```
