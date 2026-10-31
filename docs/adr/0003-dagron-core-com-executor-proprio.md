# 0003. `dagron-core` para o grafo, com executor próprio

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

O projeto foi concebido para usar o Dagron. Ao analisar a versão 0.1.1, verificamos que o nome
`dagron` não existe no crates.io e que o projeto se divide assim:

- `dagron-core` (crates.io): motor de grafo, com construção, ordenação topológica e análise.
- `dagron-ui` (crates.io): painel web de acompanhamento.
- `dagron` (PyPI): bindings Python, onde ficam as estratégias de execução.

Em Rust, portanto, o Dagron não executa tarefas.

## Decisão

Usamos o `dagron-core` como motor de grafo e implementamos um `ExecutorDeDag` assíncrono:

- `DagDeIngestao` encapsula o Dagron e é o único tipo do projeto que o importa.
- O executor percorre `topological_levels()` nível a nível, com concorrência limitada por nível
  (`TODEOLHO_CONCORRENCIA`, 2 por padrão).
- A falha de uma tarefa não interrompe as outras. Só os seus descendentes são ignorados, e o
  processo termina com código de saída diferente de zero.

Não há agendador. A execução é disparada sob demanda pela CLI.

## Consequências

- Ciclos, dependências inexistentes e nomes duplicados são rejeitados ao montar o grafo, antes de
  qualquer acesso à rede.
- Assumimos a manutenção de um executor pequeno, de responsabilidade bem delimitada.
- Executar por níveis é simples, mas um nível só começa quando o anterior termina inteiro, mesmo
  que uma tarefa específica já pudesse rodar. É aceitável para as DAGs atuais, que são pequenas.
- O Dagron é um projeto jovem (versão 0.1.x). O encapsulamento limita o impacto de trocá-lo.

## Alternativas consideradas

- **Bindings Python do Dagron:** trariam a execução pronta, mas contrariam a decisão de usar Rust e
  aumentam a imagem e o consumo de memória.
- **Airflow ou Dagster:** completos, porém pesados demais para rodar em máquinas modestas.
- **`petgraph` direto:** é o que o `dagron-core` usa por baixo, mas exigiria escrever a validação de
  ciclos e os níveis que já recebemos prontos.
- **`dagron-ui`:** pode ser adotado depois para acompanhar as execuções; ainda não é necessário.
