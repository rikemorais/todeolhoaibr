# 0008. Painel próprio de execuções, alimentado por um histórico no armazenamento

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

Acompanhar as ingestões pelo terminal não basta: a saída some quando o comando termina, e não há como
consultar execuções anteriores ou entender rapidamente o que falhou. Precisamos de uma interface visual
que atenda aos mesmos princípios do projeto: Docker como único requisito e consumo baixo.

A ingestão é um processo que nasce e morre a cada comando. A interface precisa ficar disponível o tempo
todo, então os dois rodam em containers separados e precisam de um meio para trocar informações.

## Decisão

**Histórico de execuções**

- Cada execução é um JSON no bucket `execucoes`, separado do `raw`, porque é dado operacional, não dado
  ingerido. A chave começa pelo horário, então a ordem das chaves é a ordem cronológica.
- O registro nasce com todas as tarefas pendentes e é regravado a cada mudança de estado.
- O executor avisa sobre as tarefas pelo padrão **Observer** (`ObservadorDeExecucao`), sem saber que o histórico
  existe. Os métodos do observador não devolvem erro: uma falha ao publicar gera só um aviso no log e nunca
  derruba a ingestão.

**Execuções interrompidas**

Um processo morto não consegue avisar que morreu. Enquanto roda, a ingestão republica o registro a cada 30
segundos, e uma execução sem conclusão e sem atualização há mais de 5 minutos é exibida como "Interrompida". O
estado é calculado na leitura, e nada precisa corrigir o registro depois.

**Painel**

- Um crate novo, `todeolho-painel`, que depende só do `nucleo` e lê pela porta `ConsultaDeExecucoes`. O binário é
  o mesmo da ingestão: `todeolho painel`.
- Serve uma única página HTML, embutida no binário, sem etapa de build e sem nada carregado de fora.
- A página consulta a API a cada 2 segundos com execução em andamento, e a cada 8 segundos fora disso.
- Mostra as 50 execuções mais recentes e, em cada uma, a DAG, o estado, o volume, a duração e o erro das tarefas.
- Sem escolha explícita, acompanha a execução mais recente.

**Segurança**

- O painel é somente leitura: não dispara, altera nem apaga nada.
- A porta é publicada apenas em `127.0.0.1` e não há autenticação. **Ele não foi feito para ficar exposto na internet.**
- Mensagens de erro podem conter texto vindo de APIs externas. Por isso a página só insere dados como texto, nunca
  como HTML, e uma Content-Security-Policy restritiva serve como segunda barreira.

## Consequências

- Passam a existir um serviço sempre ativo (cerca de 3 MiB de memória, com limite de 64 MB) e o comando `docker compose
  up -d` no fluxo de uso.
- Execuções anteriores a esta decisão não aparecem no painel, pois nunca tiveram registro.
- Falhas anteriores à criação do registro, como o armazenamento fora do ar, aparecem só no terminal.
- A detecção de interrupção leva até 5 minutos, e uma máquina que hiberna durante uma ingestão pode aparecer como
  "Interrompida" até o processo voltar a publicar.
- Consultar por polling lista o bucket inteiro a cada ciclo. Com milhares de execuções isso pede índice ou retenção, o
  que não é necessário agora.
- O histórico fica no mesmo volume da Raw: `docker compose down -v` apaga os dois.
- O painel é a base natural para, no futuro, disparar ingestões, mas isso exigirá autenticação e um novo ADR.
- Existe mais JavaScript no projeto. Ele não tem testes automatizados: só a API tem. O comportamento da página foi
  verificado em um navegador real durante a implementação.

## Alternativas consideradas

- **`dagron-ui`:** vive dentro do processo que executa, então some ao fim da ingestão e não tem histórico. Também mostra
  uma DAG por vez e tem interface em inglês. Poderia complementar o painel, mas não substituí-lo.
- **Reconstruir o histórico a partir dos manifestos da Raw:** dispensaria o bucket novo, mas manifestos só existem para
  tarefas concluídas, justamente o oposto do que mais importa ver.
- **SSE ou WebSocket:** atualização instantânea não compensa aqui, já que os dados vêm de arquivos e o atraso de 2
  segundos é aceitável.
- **Grafana, Airflow ou similares:** pesados demais para o objetivo de rodar em máquinas modestas.
- **Frontend com React ou similar:** exigiria Node e uma etapa de build para uma página pequena.
- **Banco de dados para o histórico:** mais um serviço para manter, sem necessidade com este volume.
