# Referência de API (rustdoc)

A documentação de código dos crates é gerada por `cargo doc` e publicada junto
com este site. Em produção (GitHub Pages) os links abaixo levam às páginas do
`rustdoc`. Para vê-los ao rodar `mkdocs serve` **localmente**, gere e **copie** a
saída para `docs/api/` antes (o `cargo doc` escreve em `target/doc`, não em
`docs/api`):

```bash
cargo doc --no-deps --workspace
cp -r target/doc/. docs/api/
rm -f docs/api/index.html   # preserva esta landing
```

| Crate | Papel | API |
| --- | --- | --- |
| `todeolho-nucleo` | Domínio: traits `FonteDeDados`/`Tarefa`, DAG, portas | [todeolho_nucleo](todeolho_nucleo/index.html) |
| `todeolho-fontes` | Catálogo de fontes públicas e tipos de tarefa | [todeolho_fontes](todeolho_fontes/index.html) |
| `todeolho-infraestrutura` | Adaptadores: S3, cliente HTTP, disco | [todeolho_infraestrutura](todeolho_infraestrutura/index.html) |
| `todeolho-painel` | Painel de execuções | [todeolho_painel](todeolho_painel/index.html) |
| `todeolho-cli` | Interface de linha de comando (binário `todeolho`) | [todeolho](todeolho/index.html) |

!!! tip "Como é gerada"
    O job de documentação roda `cargo doc --no-deps --workspace` e copia
    `target/doc/` para `docs/api/` antes do build do MkDocs. Assim a referência
    de API entra em cada versão publicada pelo `mike`.
