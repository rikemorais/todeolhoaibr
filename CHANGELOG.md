# Histórico de mudanças

Todas as mudanças relevantes do projeto ficam registradas aqui.

O formato segue o [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/), e o projeto segue o
[Versionamento Semântico](https://semver.org/lang/pt-BR/). Veja o [ADR 0011](docs/adr/0011-versionamento-e-commits.md).

## [Não lançado]

### Adicionado

- Fonte `camara-despesas` (sob demanda) com as despesas da CEAP (Cota para o Exercício da Atividade Parlamentar): um arquivo anual, desde 2008, filtrável por `ano` e guardado em `raw/camara/despesas/` ([issue #25](https://github.com/rikemorais/todeolhoaibr/issues/25)).

## [0.3.0] - 2026-11-01

### Adicionado

- Publicação do ponteiro `_ultima_versao.json` resistente a sobrescrita por uma rodada mais antiga, comparando o início da execução ([issue #16](https://github.com/rikemorais/todeolhoaibr/issues/16)).
- Gate de cobertura de documentação no CI (`cargo doc` com `-D warnings`) e `#![warn(missing_docs)]` nas bibliotecas, para a documentação não regredir ([issue #20](https://github.com/rikemorais/todeolhoaibr/issues/20)).
- Gate de formatação e lint no CI (`cargo fmt --check` e `cargo clippy -D warnings`), barrando PRs fora do padrão ([issue #4](https://github.com/rikemorais/todeolhoaibr/issues/4)).
- Diagramas de arquitetura e de fluxo da ingestão (Archify), em `docs/diagramas`, com prévia no README ([issue #7](https://github.com/rikemorais/todeolhoaibr/issues/7)).

## [0.2.1] - 2026-10-31

### Adicionado

- Documentação completa (100%) da API pública das bibliotecas (`nucleo`, `infraestrutura`, `fontes`, `painel`) e os primeiros doctests verificados pelo `cargo test` ([issue #6](https://github.com/rikemorais/todeolhoaibr/issues/6)).

## [0.2.0] - 2026-10-31

Pré-checagem idempotente: rodar o processo de novo não rebaixa nem duplica o que já está na camada Raw.

### Adicionado

- Pré-checagem idempotente na ingestão: antes de buscar, confere a Raw (manifesto de procedência com SHA-256 e o tamanho de cada parte) e só (re)ingere o que está **ausente, menor, desatualizado ou incompleto** ([issue #13](https://github.com/rikemorais/todeolhoaibr/issues/13)). A integridade das partes é por tamanho (detecta ausência e truncamento), barata em memória; ver [ADR 0012](docs/adr/0012-pre-checagem-idempotente.md).
- Requisição condicional (`If-None-Match`) por página nas APIs paginadas: 304 reaproveita a página guardada, 200 reingere; páginas inalteradas não são reescritas.
- O painel mostra, por tarefa, se foi reaproveitada ou reingerida e o motivo.

### Alterado

- O porto `ClienteHttp` ganha requisição condicional e `ArmazenamentoRaw` ganha a consulta de `tamanho` (HEAD), usada na verificação de integridade sem baixar o arquivo. Ver [ADR 0012](docs/adr/0012-pre-checagem-idempotente.md).

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
