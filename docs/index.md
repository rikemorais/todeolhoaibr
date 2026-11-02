# Tô de Olho AI

Ingestão de **dados públicos brasileiros** (político-socioeconômicos) para
acompanhar gastos, representatividade e desenvolvimento — do voto à execução
orçamentária. Projeto em **Rust**, com arquitetura hexagonal, camada Raw opaca
e refino em medalhão.

!!! note "Como esta documentação se organiza"
    - **Arquitetura** — as decisões que moldam o projeto, registradas como
      [ADRs](adr/README.md).
    - **Fontes de dados** — cada fonte pública ingerida, o que coletamos e como.
    - **Diagramas** — visão dos crates e do fluxo de ingestão.
    - **Referência de API** — a documentação de código (`rustdoc`) dos crates,
      publicada em [`/api`](api/index.md).

## Começando

```bash
# Lista as fontes disponíveis (offline)
docker compose run --rm ingestao listar

# Planeja uma ingestão sem executar
docker compose run --rm ingestao plano ibge

# Executa a ingestão de uma fonte
docker compose run --rm --build ingestao executar ibge
```

## Arquitetura em uma frase

Cada **fonte** declara suas tarefas (Factory Method); o **executor** resolve a
DAG e grava os bytes **opacos** na camada **Raw** de um armazenamento S3
(versitygw). O refino (bronze → silver → gold → diamond) é estágio separado.
Veja os [ADRs](adr/README.md) para o porquê de cada escolha.
