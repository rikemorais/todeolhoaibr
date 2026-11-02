# 0017. Organização das fontes por pasta, uma por fonte

- **Status:** Aceito
- **Data:** 2026-11-02

## Contexto

Cada fonte de dados pode ter vários recursos (a Câmara tem cadastros, despesas,
tramitações e votações; o Senado tem parlamentares, processos, detalhes e
votações). Até aqui, os recursos ficavam em arquivos soltos no mesmo diretório,
distinguidos por prefixo no nome (`camara_despesas.rs`, `senado_processos.rs`,
`camara_api.rs`). O `fontes/mod.rs` listava fontes e recursos lado a lado, e o
prefixo simulava um namespace que o sistema de módulos já oferece. Com 4–5
recursos por fonte, a pasta plana ficou difícil de navegar.

## Decisão

Cada fonte é uma pasta `crates/fontes/src/fontes/<fonte>/`. O `mod.rs` da pasta
faz a fiação (declara os módulos de recurso e re-exporta os tipos `Fonte*`) e
cada recurso é um arquivo `<recurso>.rs` sem prefixo. O cliente HTTP interno de
cada fonte fica em `api.rs` (privado). Fontes de recurso único também viram
pasta, por consistência (`ibge/localidades.rs`, `tse/votacoes.rs`).

A API pública do crate (`todeolho_fontes::Fonte*`, `catalogo()`) e os nomes de
`recurso` gravados na Raw não mudam: a reorganização é apenas física, de módulos.

## Consequências

- Navegação por fonte fica óbvia; adicionar um recurso é criar um arquivo na
  pasta e uma linha no `mod.rs` da fonte.
- Referências internas usam `super::api` em vez do módulo prefixado.
- Mais um nível de diretório e um `mod.rs` por fonte, inclusive para fontes de
  recurso único — custo aceito em troca da uniformidade.

## Alternativas consideradas

- **Manter o prefixo no nome do arquivo** (`camara_*`, `senado_*`): preserva a
  pasta plana, mas repete o nome da fonte em cada arquivo e mistura fontes e
  recursos no mesmo `mod.rs`. Rejeitada: o prefixo é um namespace manual que a
  pasta resolve de forma nativa.
