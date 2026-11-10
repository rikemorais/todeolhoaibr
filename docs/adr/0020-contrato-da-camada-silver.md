# 0020. Contratos, publicação e execução SQL da camada silver

- **Status:** Aceito
- **Data:** 2026-11-06

## Contexto

O bronze guarda Parquet com colunas textuais e conserva a granularidade da origem. A silver precisa tipar, limpar e deduplicar, mas as fontes variam em grão, formato e nível geográfico. O padrão de manifesto e ponteiro já permite publicar versões sem sobrescrever a montante.

## Decisão

Cada **tabela silver** terá contrato versionado com grão, chave de negócio, tipos, campos obrigatórios, política de nulos e duplicatas e regras de qualidade. O catálogo silver apontará para uma ou mais tabelas bronze, sem presumir que uma entrada do catálogo bronze produza uma única tabela.

O bucket `silver` usará partes Parquet, manifesto e ponteiro de versão vigente. O manifesto identificará o recurso, a tabela e a versão do bronze de entrada, a versão do contrato/regra e as contagens de entrada, saída, duplicatas e rejeições por motivo. O ponteiro só será publicado após todas as partes e o manifesto estarem íntegros. Uma nova versão do bronze **ou da regra** exige nova materialização; versões anteriores continuam disponíveis para consulta ou recuperação. Nenhuma transformação altera raw ou bronze.

### Política de qualidade e falha parcial

Cada contrato declara campos obrigatórios, conversões opcionais, chave de
negócio quando houver, política de duplicatas e limiares. O padrão é **zero
rejeições** para campo obrigatório ausente ou inválido e conversão obrigatória
falha. Para essas duas classes, um contrato pode declarar limite absoluto
inteiro por motivo: até o limite, a linha é excluída da saída e registrada na
quarentena; acima dele, toda a publicação falha. Isso exige justificativa,
versão de regra nova e fixture de aceitação e ultrapassagem do limite. Conflito
de chave e ausência de referência Bronze têm limite fixo zero, sem exceção:
qualquer ocorrência bloqueia o ponteiro. Não há limite percentual implícito.

Um campo opcional inválido pode virar nulo somente se o contrato nomear a
conversão e conservar o valor de origem em coluna original ou `registro_original`.
Essas conversões são contadas por motivo, mas não eliminam a linha. Valores
sentinela não são convertidos globalmente: cada família define quais significam
ausência. Duplicatas integrais só podem ser removidas quando o contrato comprovar
que são cópias e declarar a comparação completa; sem essa prova, manter todas
as ocorrências com a referência técnica da linha. Hash de conteúdo não é chave
de negócio.

O diagnóstico de rejeição contém motivo estável, versão da regra, manifesto
Bronze, parte, tabela interna e ordinal. A linha original permanece no Bronze;
o diagnóstico não precisa copiar identificadores pessoais. O destino planejado
é `silver/_quarentena/<fonte>/<recurso>/tabela=<nome>/execucao=<id>/`, separado da tabela
publicada. Contagens de entrada, saída, conversões opcionais, cópias removidas
e rejeições por motivo pertencem ao manifesto da execução. O publicador atual
ainda não grava esse diagnóstico estruturado nem todas essas métricas; os
contratos novos que dependam de rejeição recuperável precisam dessa capacidade
antes da ativação.

A unidade de publicação é a tabela Silver de um recurso. Partes e manifesto
são gravados antes de seu ponteiro; falha ou cancelamento antes dessa troca
mantém a versão vigente anterior. Tabelas irmãs oriundas do mesmo ZIP têm
ponteiros independentes: se uma falhar, as já publicadas continuam válidas.
O contrato alvo exige resultado observável por tabela. A CLI de votações do
Senado registra sessões e votos como tarefas separadas na mesma rodada. O
construtor público legado ainda pode publicar ambas como uma tarefa; outros
contratos irmãos podem rodar em invocações separadas, sem registro pai comum.
Nesses casos, o operador deve conferir o ponteiro de cada tabela e as execuções
individuais. Um consumidor que exija um snapshot conjunto deve esperar todas
apontarem para a mesma versão Bronze; não se presume transação entre tabelas.
Reprocessamento por nova versão Bronze ou nova regra conserva versões antigas
para recuperação.

A conferência do ponteiro Bronze antes de publicar e a proteção por recência
do ponteiro Silver são verificações sequenciais, não uma transação entre
buckets. A serialização local impede a inversão de ponteiros entre tarefas de
um mesmo processo, e a recusa de um ponteiro Silver antigo aparece como
falha da tarefa. O porto de armazenamento já oferece comparação condicional,
mas o adaptador S3 ainda usa a implementação padrão, sem troca atômica entre
processos. Como Bronze e Silver são buckets
distintos, nem essa troca garante que o Bronze não avance logo depois da
conferência. Até haver leitura que valide a versão Bronze referenciada pelo
ponteiro Silver, consumidores não devem tratar o ponteiro Silver isolado como
prova de atualidade em relação ao Bronze. A ativação de contratos que exijam
essa garantia depende dessas duas capacidades; a fase 0 define o requisito,
mas não declara que o armazenamento atual oferece transação entre buckets. A
implementação e os testes dessas garantias estão rastreados na
[issue #300](https://github.com/rikemorais/todeolhoaibr/issues/300).

As transformações Silver serão expressas preferencialmente em arquivos SQL versionados sob `crates/refino/sql/silver/` e executadas pelo Apache DataFusion dentro do processo Rust do `refino`. O código os incorpora com `include_str!`, então a imagem distroless não precisa de shell nem de SQL montado em volume. Cada contrato seleciona explicitamente colunas, conversões, regras e versão; não inferir schema por conveniência nem interpolar valores externos em SQL. Rust continua responsável pela decodificação Bronze, por funções de domínio que não fiquem claras em SQL e pela orquestração, linhagem e publicação.

O executor consulta exclusivamente as partes enumeradas pelo snapshot do manifesto vigente. Deve transmitir batches sem reter o conjunto inteiro em memória sem limite, carregar a identidade de origem (manifesto, parte e ordinal) explicitamente durante o plano e encaminhar o resultado ao publicador comum. Não depender da ordem de saída do plano. Os contratos SQL ativados projetam as três referências na linha publicada, e o publicador rejeita schema ou lote que as omita ou produza referências nulas. Relações que expandem uma linha usam também a posição filha, como `ordem_voto` no Senado. `registro_original` sozinho não identifica duas ocorrências iguais. O publicador grava Parquets e manifesto antes de trocar o ponteiro; a conferência sequencial do Bronze reduz a janela de desatualização, mas não equivale a uma transação entre buckets.

O DataFusion será embutido, sem introduzir um serviço externo ou substituir o agendador, a CLI, o painel ou o catálogo de contratos. Operações que precisem de spill devem usar o `DiskManager` com diretório temporário dedicado por execução sob a raiz temporária do processo, quota explícita e arquivos removidos ao descartar o contexto. O executor aplica limites de memória e de entrada, inicia com uma partição por consulta e deixa o paralelismo entre tarefas sob controle do Dagron; os valores podem ser ajustados pela configuração do executor e devem ser medidos em dados representativos. A configuração inicial é 512 MiB de memória, 2 GiB de spill, 100 GiB de entrada e batches de 8.192 linhas. Se algum limite for excedido, a consulta falha e a publicação não avança.

O cancelamento abandona o stream do DataFusion e descarta a execução e seus temporários. O executor só grava manifesto e ponteiro depois de consumir e fechar completamente o Parquet de saída; erro ou cancelamento antes disso mantém a versão vigente anterior. As transformações Rust existentes permanecem disponíveis até cada contrato SQL comprovar paridade e ser ativado individualmente, permitindo rollback por contrato.

O crate `refino` alinha Arrow e Parquet à versão usada pelo DataFusion, evitando duas versões incompatíveis de `RecordBatch` no pipeline. Parser SQL e leitura Parquet mantêm strings como `Utf8`, que corresponde ao tipo dos contratos Silver, em vez da otimização `Utf8View`. A dependência começa em DataFusion 55.2.0 e Arrow/Parquet 59.2.0; o lockfile resolve Arrow/Parquet 59.3.2. O DataFusion estabelece Rust 1.94 como versão mínima do workspace. Recursos de execução são habilitados explicitamente, sem recursos padrão não necessários. Mudanças futuras dessas versões precisam manter o alinhamento ou justificar um adaptador com custo medido.

O código IBGE do município será texto, nunca número. Tabelas nacionais, estaduais ou regionais manterão seu nível geográfico original; não receberão município artificial. A conformação dentro da fonte pertence à silver. Cruzamentos entre domínios e indicadores derivados pertencem à gold, conforme [ADR 0015](0015-arquitetura-de-dados-em-medalhao.md).

- Mudanças de schema e regra são rastreáveis e podem ser reprocessadas sem nova ingestão.
- O volume armazenado cresce com versões e registros rejeitados.
- Contratos e política de qualidade precisam ser validados com amostras de anos e formatos distintos antes de cada família entrar em produção.

## Alternativas consideradas

- **Inferir schema e chave automaticamente de cada Parquet:** simplifica o início, mas mudanças da origem poderiam alterar silenciosamente o contrato silver.
- **Usar apenas a versão do bronze para idempotência:** deixaria correções de regras sem gatilho de reprocessamento.
- **Forçar município + ano em todas as tabelas:** perderia dados nacionais, estaduais e eventos sem município; a chave é adequada a produtos analíticos posteriores, não a todo registro silver.
- **Manter transformação manual de arrays Arrow como padrão:** dá controle, mas torna projeções, casts, filtros e regras relacionais repetitivas e menos acessíveis a contribuidores que conhecem SQL.
- **Usar DataFusion como orquestrador ou substituir o publicador:** o engine executa planos de consulta, mas não conhece os contratos, o manifesto do snapshot, a linhagem nem a publicação transacional do projeto.
- **Carregar todas as partes em uma `MemTable` sem limite:** simplifica o protótipo, porém não é aceitável para fontes volumosas como TSE; a integração deve manter execução limitada e medir spill.

## Consequências

- SQL versionado passa a ser a forma recomendada de contribuir com contratos Silver e pode ser revisado separadamente das tarefas Rust.
- DataFusion e suas versões Arrow/Parquet aumentam a árvore de dependências e o tempo de compilação; recursos não usados ficam desativados.
- Cada migração precisa comprovar paridade semântica, qualidade, linhagem, idempotência e consumo operacional antes de remover a transformação anterior.
- Regras específicas de fonte podem continuar em Rust quando houver justificativa clara e testes de contrato.
