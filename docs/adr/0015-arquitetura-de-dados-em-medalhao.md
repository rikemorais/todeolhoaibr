# 0015. Arquitetura de dados em medalhão: raw → bronze → silver → gold → diamond

- **Status:** Proposto
- **Data:** 2026-11-01

## Contexto

O [ADR 0014](0014-raw-opaca-e-refino-em-estagio-separado.md) decidiu que a camada Raw é opaca em bytes e que a transformação acontece num estágio separado, a jusante, mas **não nomeou as camadas** nem definiu quantos níveis existem. Com o catálogo de fontes crescendo e o objetivo analítico (cruzar tudo em `município + ano`), precisamos de uma nomenclatura clara, com responsabilidades separadas, para saber onde cada tipo de dado mora e o que cada estágio pode fazer.

## Decisão

Adotar uma **arquitetura em medalhão** com cinco camadas, cada uma em seu próprio bucket no versitygw ([ADR 0005](0005-armazenamento-s3-com-versitygw.md)). Cada estágio **lê a camada anterior e escreve a próxima, nunca reescreve a montante**.

- **`raw`** — bytes fiéis da origem, imutável, com manifesto de procedência. É a única que **existe hoje** ([ADR 0006](0006-layout-da-camada-raw.md), [ADR 0014](0014-raw-opaca-e-refino-em-estagio-separado.md)).
- **`bronze`** — o `raw` apenas **decodificado e estruturado**: descompactado, reencodado para UTF-8, posto em formato colunar, mantendo correspondência 1:1 com a origem, **sem regra de negócio**.
- **`silver`** — dados **limpos, tipados e conformados**: deduplicação, chaves canônicas (código IBGE do município + ano), padronização entre fontes. É o "refino/trusted" previsto no [ADR 0014](0014-raw-opaca-e-refino-em-estagio-separado.md).
- **`gold`** — dados **agregados e modelados para análise e consumo**: os cruzamentos por `município + ano`, indicadores derivados, prontos para o painel e as consultas.
- **`diamond`** — camada **premium de produtos finais curados**: índices compostos, datasets de correlação (desenvolvimento × eleito × destinação), *features* para modelos. O topo, com forte curadoria.

A ingestão atual entrega apenas o `raw`. As demais camadas serão materializadas por estágios de refino (crates próprios), fora do escopo da ingestão e da primeira versão de cada fonte.

## Consequências

- Dá nome e responsabilidade a cada nível: fica claro onde cada dado mora e o que cada estágio pode (e não pode) fazer. `bronze` só traduz formato; regra de negócio começa no `silver`.
- Define o alvo dos estágios de refino: `bronze` consome `raw`; `silver` consome `bronze`; e assim por diante. O visualizador de dados consumirá `gold`/`diamond`, não o `raw`.
- Como cada camada é fiel à anterior e nunca a reescreve, qualquer camada pode ser **reprocessada** com regra nova sem rebaixar a origem — reforça a imutabilidade do `raw` ([ADR 0006](0006-layout-da-camada-raw.md)).
- **Custo de armazenamento cresce** (o mesmo dado em vários níveis). Aceitamos em troca da clareza e da reprocessabilidade; camadas derivadas podem ser recalculadas e descartadas sem perda, já que o `raw` é a fonte da verdade.
- **`diamond` foge do medalhão clássico** (que vai até `gold`). Adotamos o nível extra de propósito, para separar produtos premium curados dos dados de consumo analítico do `gold`.
- Esta decisão **complementa** o [ADR 0014](0014-raw-opaca-e-refino-em-estagio-separado.md): ele proíbe transformação na ingestão e aqui nomeamos para onde ela vai.

## Alternativas consideradas

- **Medalhão clássico de três camadas (`bronze`/`silver`/`gold`), sem `raw` e sem `diamond`:** o `raw` já existe e é imutável, distinto do `bronze` (que já reencoda e descompacta); e o `diamond` separa produtos premium dos dados de consumo. Rejeitada por perder essas fronteiras.
- **Duas camadas (`raw` + `tratado`):** simples, mas junta "só traduzir formato" com "aplicar regra de negócio" e "agregar para consumo" num balaio só. Rejeitada por misturar responsabilidades.
- **Tabelas versionadas em um lakehouse (Delta/Iceberg):** resolveria versionamento e reprocessamento com elegância, mas é peso grande de infraestrutura, contra o [ADR 0007](0007-docker-para-maquinas-modestas.md). Rejeitada por ora.
