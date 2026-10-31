# 0004. Factory Method para adicionar fontes de dados

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

Adicionar novas APIs públicas será a mudança mais frequente do projeto. Cada API tem seus
próprios recursos, dependências e formas de paginar, mas validar e executar uma DAG é sempre igual.
Queremos que adicionar uma fonte seja seguro e não exija tocar no que já funciona.

## Decisão

Adotamos o padrão **Factory Method** do catálogo GoF:

| Papel no padrão | No projeto |
| --- | --- |
| Criador | trait `FonteDeDados` |
| Factory Method | `FonteDeDados::criar_tarefas` |
| Operação do criador que usa o método | `FonteDeDados::montar_dag` |
| Criadores concretos | `FonteIbge`, `FonteCamara` |
| Produto | trait `Tarefa` |
| Produto concreto | `TarefaDeIngestaoHttp` |

Uma fonte só declara quais tarefas existem e como dependem umas das outras. O código comum valida o
grafo e o executor o percorre sem saber qual é a fonte.

Adicionar uma fonte exige duas alterações: criar o módulo em `crates/fontes/src/fontes/` e registrá-lo
na função `catalogo()`. Um teste garante que toda fonte do catálogo monta uma DAG válida.

As diferenças de paginação ficam em `EstrategiaDePaginacao`, um Strategy complementar. Uma fonte
com uma origem diferente de HTTP, como arquivos CSV, pode criar outro produto implementando `Tarefa`.

## Consequências

- O executor e a CLI ficam fechados para modificação quando uma fonte é adicionada.
- Fontes novas e simples têm poucas dezenas de linhas.
- `catalogo()` continua sendo um ponto de alteração. É intencional: fica explícito o que o projeto
  ingere, sem descoberta automática ou macros.
- A indireção por `dyn Trait` tem custo desprezível perto do tempo gasto com rede.

## Alternativas consideradas

- **Strategy puro:** resolve a variação de comportamento, mas não dá um lugar para cada fonte criar
  várias tarefas e declarar suas dependências.
- **Template Method:** bom para um fluxo fixo de passos, mas engessa a fonte em um único fluxo, quando
  o que varia aqui é justamente o formato da DAG.
- **Abstract Factory:** serviria para famílias de produtos relacionados. Hoje só há um tipo de produto.
- **Builder:** bom para montar um objeto complexo, porém não resolve como novas fontes se conectam ao
  restante do sistema.
- **Fontes declaradas em YAML:** adiciona uma linguagem e um interpretador, e perde a verificação do
  compilador.
