# 0016. Documentação como site estático com Material for MkDocs no GitHub Pages

- **Status:** Aceito
- **Data:** 2026-11-02

## Contexto

A documentação do projeto é majoritariamente **prosa em Markdown**: ADRs,
descrição de fontes, READMEs de diagramas e guias. Hoje ela só é lida como
arquivos renderizados pelo GitHub — sem navegação, sem busca e sem versão.
Queremos um site publicado no **GitHub Pages**, com boa navegação e busca, e
que também exponha a **referência de API** (`rustdoc`) dos crates.

Sendo um projeto Rust, a opção "nativa" seria o mdBook; mas a preferência da
equipe é o **Material for MkDocs** (Python), cujo tema, busca e recursos de
conteúdo (admonitions, abas, mermaid, lightbox) atendem melhor à prosa que
temos.

## Decisão

Publicar a documentação com **Material for MkDocs** no **GitHub Pages**:

- `mkdocs.yml` na raiz, com `docs_dir` padrão (`docs/`), reusando o conteúdo que
  já existe (ADRs, fontes, diagramas).
- **Referência de API** gerada por `cargo doc --no-deps --workspace` e publicada
  sob `/api`, linkada no menu.
- **Versionamento com `mike`**: a `main` publica a versão `dev`; cada tag de
  release `vX.Y.Z` publica a série `X.Y` com o alias `latest` (o padrão do
  site). Permite que consumidores presos a uma versão antiga leiam a doc
  correspondente.
- **Deploy por GitHub Actions** para o branch `gh-pages` (modelo do `mike`);
  o Pages serve esse branch.
- O **toolchain Python fica isolado** em `requirements-docs.txt`, usado só no
  job de docs — não toca o build Rust.

## Consequências

- Ganhamos navegação, busca client-side, temas claro/escuro e versionamento,
  sem servidor (site estático).
- Prosa e referência de API convivem no mesmo site, cada uma na ferramenta certa
  (`mkdocs` para guias, `rustdoc` para a API).
- Passa a haver **Python** no repositório (ainda que só para docs): uma segunda
  cadeia de dependências a manter e atualizar; mitigado com versões pinadas.
- Um branch `gh-pages` gerenciado pelo `mike` (não editar à mão).
- O Pages precisa ser configurado uma vez para servir o branch `gh-pages`.

## Alternativas consideradas

- **mdBook** (nativo Rust, sem Python): dispensaria o toolchain Python, mas tema,
  busca e recursos de conteúdo são mais pobres que o Material, que é a
  preferência da equipe.
- **Somente `rustdoc`**: cobre a API, mas não a prosa (ADRs, fontes, guias), que
  é o grosso da nossa documentação.
- **Deixar o GitHub renderizando os `.md`**: é o estado atual — sem navegação,
  busca ou versão.
- **MathJax via `polyfill.io`** (presente no modelo de referência): descartado —
  o domínio `polyfill.io` foi comprometido em 2024 (risco de supply-chain) e não
  temos conteúdo LaTeX que justifique MathJax.
