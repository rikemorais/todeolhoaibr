# 0010. Seleção com aviso de espaço e ingestão de arquivos grandes

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

A fonte do TSE publica votos por seção eleitoral em 408 arquivos ZIP, de 1998 a 2026, somando 16,4 GB. O maior arquivo tem
859 MB. Descompactados, os arquivos crescem cerca de 8 vezes (medido em uma amostra). Isso expôs dois problemas:

- O projeto deve rodar em computadores modestos, e **muitos não têm esse espaço em disco**. A pessoa precisa saber quanto vai
  ocupar e escolher o que ingerir antes de começar.
- A ingestão carregava cada resposta inteira na memória, com limite de 256 MB e tempo limite de 90 segundos.

## Decisão

**Seleção genérica por etiquetas**

As tarefas declaram etiquetas, como `ano=2024` e `uf=AC`. Filtros, menu e validação são derivados delas. Uma fonte nova que
etiqueta suas tarefas ganha tudo isso sem código específico, o que mantém o princípio aberto/fechado do
[ADR 0004](0004-factory-method-para-fontes.md).

- `docker compose run --rm ingestao` abre um menu com o tamanho de cada fonte, ano e UF.
- `executar <fonte> --filtro ano=2024 --filtro uf=AC` faz o mesmo sem o menu. Filtros e valores desconhecidos são erro, nunca ignorados.
- Fontes muito grandes declaram `sob_demanda` e ficam fora de `executar --todas`, para que ninguém baixe 16 GB por acidente.

**Aviso de espaço antes de executar**

- O tamanho vem da origem, via `HEAD`. APIs que não o informam declaram um tamanho típico, medido em uma ingestão real.
- Arquivos que já estão atualizados contam como zero.
- O espaço livre é medido no volume real, montado como somente leitura na ingestão. Em Mac e Windows, esse é o disco virtual do Docker, não
  o do computador.
- A comparação usa o **pico**: o total mais o maior arquivo. Ao unir as partes, o armazenamento mantém partes e arquivo final ao mesmo
  tempo. Medimos cerca de 76 MiB acima do valor final em um arquivo de 92 MB.
- Se não couber, a execução é recusada, mesmo com `--sim`. Se couber, a pessoa confirma, a menos que use `--sim`. Sem terminal, `--sim` é obrigatório.
- Espaço desconhecido gera um aviso, sem ser tratado como suficiente.

**Arquivos grandes**

- `TarefaDeArquivo` transfere em fluxo, calculando tamanho e SHA-256 enquanto os bytes passam, sem descompactar nem alterar o ZIP.
- O envio é um multipart próprio, sequencial, com um buffer de 16 MiB por arquivo. O `put_object_stream` do `rust-s3` foi descartado porque dimensiona a
  concorrência pela memória livre da máquina, ignorando o limite do container.
- Transferências que caem recomeçam o arquivo, até 3 vezes. O objeto só fica visível na Raw ao final.
- Antes de cada execução, transferências interrompidas são descartadas. Sem isso, as partes de um processo morto ocupariam disco para sempre.

**Sem baixar de novo o que não mudou**

Cada recurso guarda o ETag da última ingestão em `_ultima_versao.json`. Isso evita repetir gigabytes e faz uma carga longa interrompida continuar de onde parou.

## Micro batches

Avaliamos micro batches como solução para o volume. Na ingestão, eles já estão presentes: um arquivo por tarefa e partes de 16 MiB. Isso resolve memória e retomada,
**mas não reduz o espaço em disco**, porque o total armazenado é o mesmo. O processamento contínuo em pequenos intervalos não se aplica, já que o TSE publica arquivos
estáticos após cada eleição. O formato deve ser reavaliado na transformação, em que processar um arquivo por vez e descartar o bruto pode de fato economizar espaço.

## Consequências

- **Exceção ao [ADR 0006](0006-layout-da-camada-raw.md):** `_ultima_versao.json` é sobrescrito. Ele é um ponteiro de controle, não um dado ingerido; os dados continuam imutáveis.
- Uma pessoa pode ingerir uma única UF de uma única eleição, com poucos megabytes.
- O menu faz cerca de 400 consultas ao TSE antes de abrir, o que levou alguns segundos no teste.
- O espaço avisado cobre só a camada Raw. A transformação precisará de espaço próprio: os CSVs chegam a 8 vezes o tamanho dos ZIPs.
- Duas ingestões simultâneas interferem entre si: a limpeza de uma cancela a transferência da outra, que recomeça o arquivo. Nenhum dado se perde, mas há retrabalho.
- O ETag indica mudança no arquivo, não no conteúdo, então uma republicação idêntica é baixada de novo.
- A seleção é feita pelo terminal. Levá-la ao painel exigiria autenticação ([ADR 0008](0008-painel-de-execucoes.md)).
- A lista de eleições é fixa no código e precisa ganhar uma linha a cada eleição. Eleições suplementares e o conjunto `detalhe_votacao_secao` ficaram fora.
- Para 1994 e 1996, o TSE não publica votos por seção.
- O painel deixa de desenhar o grafo acima de 24 tarefas, porque a tabela já mostra cada uma.

## Alternativas consideradas

- **Filtros por variável de ambiente, específicos do TSE:** chegaram a ser implementados, mas cada fonte nova exigiria código próprio.
- **Consultar o catálogo do TSE em tempo de execução:** não exigiria atualizar a lista, mas `plano` deixaria de funcionar sem rede. Conferimos que a lista fixa coincide com o catálogo: 408 de 408.
- **Descompactar e gravar CSV ou Parquet:** usaria muito mais disco ou já seria transformação, o que contraria o [ADR 0006](0006-layout-da-camada-raw.md).
- **Só avisar quando não couber, sem recusar:** o disco cheio poderia afetar o restante do computador ou do Docker.
- **Margem fixa em vez do pico:** seria menos exata para arquivos muito grandes ou muito pequenos.
