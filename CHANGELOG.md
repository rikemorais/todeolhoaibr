# Histórico de mudanças

Todas as mudanças relevantes do projeto ficam registradas aqui.

O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/), e o projeto segue o
[Versionamento Semântico](https://semver.org/lang/pt-BR/). Veja o [ADR 0011](docs/adr/0011-versionamento-e-commits.md).

## [0.1.0] - 2026-10-31

Primeira versão, com a ingestão de dados públicos para a camada Raw.

### Adicionado

- Fontes de dados `ibge` (regiões, estados e municípios), `camara` (legislaturas, partidos e deputados em exercício) e `tse` (votos por seção eleitoral, de 1998 a 2026).
- Menu de terminal para escolher o que ingerir, com o tamanho de cada fonte, ano e UF.
- Filtros por etiqueta na linha de comando, como `--filtro ano=2024 --filtro uf=AC`.
- Verificação de espaço em disco antes de cada execução, recusando o que não cabe.
- Camada Raw imutável, compatível com S3, com manifesto de procedência e SHA-256 de cada arquivo.
- Transferência de arquivos grandes sem carregá-los na memória, com novas tentativas e limpeza de transferências interrompidas.
- Retomada de cargas interrompidas, sem baixar de novo o que não mudou na origem.
- Painel web de execuções em `http://localhost:8080`, com histórico, DAG e erros de cada tarefa.
- Navegador de arquivos em `http://localhost:7071`.
- Ambiente completo em Docker, sem necessidade de conta ou token.
- Registros de decisão de arquitetura em `docs/adr`.
- Integração contínua que roda os testes do workspace (`cargo test`) em cada pull request.
- Dependabot para `cargo`, `github-actions` e `docker`, com atualizações semanais.
- Licença dual MIT OR Apache-2.0.
