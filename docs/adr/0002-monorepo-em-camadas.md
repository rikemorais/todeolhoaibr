# 0002. Monorepo Cargo em camadas, aplicando SOLID e POO

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

O projeto vai crescer em três frentes: ingestão, transformação e apresentação. Queremos que
as regras de domínio não dependam de tecnologias específicas e que cada parte seja testável
sem rede e sem Docker.

Rust não tem herança de classes. A orientação a objetos é expressa com `struct`s que
encapsulam estado, `trait`s como contratos e `dyn Trait` para polimorfismo.

## Decisão

Um único repositório com um workspace Cargo, em que as dependências apontam para o núcleo:

```text
              todeolho-cli (raiz de composição)
              /            |                 \
todeolho-fontes   todeolho-infraestrutura    |
              \            |                 /
                    todeolho-nucleo
```

| Crate | Responsabilidade | Conhece |
| --- | --- | --- |
| `nucleo` | Contratos, DAG, executor e contrato da camada Raw | Dagron, nenhuma API, nenhum S3 |
| `fontes` | O que ingerir de cada API pública | Só o `nucleo` |
| `infraestrutura` | S3 e HTTP concretos | Só o `nucleo` e as bibliotecas de rede |
| `cli` | Liga tudo e expõe os comandos | Todos |

Como cada princípio SOLID aparece:

- **S (responsabilidade única):** `TarefaDeIngestaoHttp` orquestra uma ingestão, a estratégia
  de paginação interpreta páginas, o adaptador S3 grava e o executor percorre o grafo.
- **O (aberto/fechado):** novas fontes não alteram o executor (ver [ADR 0004](0004-factory-method-para-fontes.md)).
- **L (substituição de Liskov):** qualquer `ArmazenamentoRaw` ou `ClienteHttp` serve no lugar de
  outro. Os testes usam versões em memória sem nenhuma adaptação.
- **I (segregação de interfaces):** portas pequenas e separadas, em vez de um único serviço amplo.
- **D (inversão de dependência):** `nucleo` define as portas e `infraestrutura` as implementa. A
  fronteira é garantida pelo compilador, porque `fontes` nem tem acesso a `reqwest` ou ao S3.

## Consequências

- As etapas futuras entram como novos crates, sem reorganizar os atuais.
- Os testes de domínio rodam em milissegundos, sem infraestrutura.
- Há mais `Cargo.toml` para manter do que em um crate único.
- A divisão em crates é aplicada pelo compilador, não apenas por convenção.

## Alternativas consideradas

- **Crate único com módulos:** mais simples, mas nada impediria o domínio de importar o S3.
- **Crate por fonte:** isolamento desnecessário agora; cada fonte tem poucas dezenas de linhas.
- **Crate separado para o executor:** nada além do `nucleo` usaria essa separação no momento.
