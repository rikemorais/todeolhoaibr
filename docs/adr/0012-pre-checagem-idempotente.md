# 0012. Pré-checagem idempotente antes de ingerir

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

A ingestão precisa ser idempotente: rodar o processo de novo não deve rebaixar nem duplicar o que já temos. Hoje isso só vale em parte.

- A `TarefaDeArquivo` (downloads de arquivo, como o TSE) pula o que não mudou comparando o ETag da origem com `_ultima_versao.json`, mas, sem ETag, sempre rebaixa, e não detecta uma carga anterior truncada ou corrompida.
- A `TarefaDeIngestaoHttp` (APIs paginadas, como IBGE e Câmara) não tem checagem nenhuma: cada rodada rebaixa todas as páginas e reescreve o manifesto.
- Nada compara o que está na Raw com a origem em tamanho ou integridade, então "menor", "desatualizado" e "diferente" não são tratados (issue #13).

## Decisão

Uma pré-checagem comum às tarefas decide, **antes de buscar**, se o recurso precisa ser reingerido.

**Regra pura e estado separados**

- `decisao::decidir` é regra pura, sem I/O: recebe o que a origem informou (sem baixar, via `ClienteHttp::informacoes`) e o estado na Raw, e devolve reaproveitar ou reingerir, com o motivo (`AusenteNaRaw`, `IncompletaOuCorrompida`, `Menor`, `Desatualizada`).
- `estado_raw::estado_na_raw` tem o I/O: localiza o manifesto vigente pelo ponteiro `_ultima_versao.json`, confere completude e integridade, e monta o estado.

**Validador autoritativo, com requisição condicional**

- O validador da origem (ETag ou `Last-Modified`) é autoritativo: se bate com o guardado, reaproveita; se difere, reingere.
- Em tarefas de arquivo, `estado_na_raw_com_origem` também retorna a URL do manifesto vigente, sem alterar o contrato público de `EstadoNaRaw`. Se a URL de origem muda, o arquivo é reingerido mesmo que o novo host reutilize o mesmo ETag opaco.
- Para APIs paginadas, a releitura usa **requisição condicional** (`If-None-Match`/`If-Modified-Since`) por página: 304 reaproveita a parte já gravada, 200 reingere aquela página. O manifesto novo é remontado a partir das partes reaproveitadas e reingeridas.

**Fallback onde não há validador**

- Origem sem validador, ou que ignora o condicional: cai para a comparação por **tamanho**, e, por fim, pela **assinatura (sha256)** registrada no manifesto, para preservar a idempotência de escrita mesmo quando o fetch não pode ser evitado.

**Integridade pelo tamanho, barata**

- O manifesto, gravado por último, é o marcador de completude ([ADR 0006](0006-layout-da-camada-raw.md)).
- Em paginação, se uma execução falhar antes de gravar o novo manifesto, remove apenas as partes novas gravadas por essa execução. Partes reutilizadas do manifesto anterior são preservadas.
- A integridade das partes é conferida pelo **tamanho** (`ArmazenamentoRaw::tamanho`, um `HEAD`), não relendo o conteúdo. Reler um ZIP de centenas de MB a cada rodada estouraria o limite de 256 MB do contêiner ([ADR 0010](0010-selecao-e-arquivos-grandes.md)). O `sha256` do manifesto continua sendo a procedência gravada na ingestão, usada na comparação com a origem, não para re-hashear o arquivo local.

## Consequências

- O porto `ClienteHttp` ganha requisição condicional, e `ArmazenamentoRaw` ganha `tamanho`. As implementações de infraestrutura passam a usar `HEAD`.
- `_ultima_versao.json` passa a valer também para as APIs paginadas, não só para arquivos. Mantém a exceção à imutabilidade já aberta no [ADR 0010](0010-selecao-e-arquivos-grandes.md): é ponteiro de controle, não dado ingerido.
- A integridade por tamanho detecta parte ausente ou truncada, mas não corrupção que preserve o tamanho. Aceitamos isso em troca de uma pré-checagem barata e segura em memória.
- APIs paginadas por **cursor**, cujas URLs mudam a cada rodada, não casam página a página; esses recursos caem no fallback por assinatura, com menos economia de banda.
- Como no [ADR 0010](0010-selecao-e-arquivos-grandes.md), o validador indica mudança no arquivo, não no conteúdo: uma republicação idêntica com novo ETag é reingerida.
- `_ultima_versao.json` guarda o `iniciada_em` da execução e é publicado por `publicar_ponteiro`, que protege as rodadas concorrentes com CAS e reavalia a recência após conflitos (ver issue #16). No backend S3, a gravação condicional usa `If-Match`/`If-None-Match`; conflitos entre processos são serializados pelo backend em vez de depender da guarda em memória, que só coordena tarefas do mesmo processo. Leituras e gravações desse objeto de controle têm timeout de 15 segundos e retries limitados. A publicação repete conflitos até 64 tentativas ou 30 segundos; ao esgotar o limite, devolve erro que orienta repetir a tarefa. O método padrão de `ArmazenamentoRaw::gravar_condicional` preserva adaptadores legados por leitura e gravação, cuja garantia vale apenas dentro do processo; não deve ser usado quando vários processos compartilham o armazenamento, que precisa implementar CAS atômico e timeout curto.
- `iniciada_em` pressupõe relógios sincronizados entre os hosts que publicam no mesmo recurso. Ao trocar uma URL configurada no código, o rollout precisa primeiro pausar agendadores e filas que iniciariam imagens antigas, aguardar as execuções antigas terminarem e confirmar que não há workers antigos em execução ou pendentes. Só então a nova imagem pode iniciar; os agendamentos só são reativados depois da troca. O ponteiro não tem um token de geração que consiga cercar binários já implantados: um processo antigo que volte a executar depois da troca ainda pode publicar dados da URL anterior.
- O bronze associa sua versão de origem ao `id_execucao` do manifesto Raw selecionado, não ao ETag opaco. Assim, uma nova captura Raw é refinada mesmo que a origem reutilize o ETag; manifesto e versão também vêm da mesma leitura de ponteiro. O CAS S3 depende de o serviço respeitar `If-Match`/`If-None-Match` e fornecer ETag que identifique a representação do objeto. A API S3 compatível não oferece uma precondição universal por `VersionId`; um backend que reutiliza o mesmo ETag para conteúdos diferentes não pode garantir CAS por conteúdo e precisa de um adaptador próprio com condição atômica por versão.
- Snapshots cuja publicação perdeu ou teve resposta ambígua não são removidos imediatamente: um marcador agenda a coleta após 24 horas, e a varredura confirma que nenhum ponteiro ainda os referencia. Isso preserva leitores que já carregaram um ponteiro antes de uma publicação ser substituída. Snapshots sem manifesto continuam elegíveis para a limpeza imediata porque nenhum ponteiro válido pode referenciá-los.

## Alternativas consideradas

- **Política de frescor por tempo** (pular se ingerido há menos de N dias): barata, mas pode servir dado velho sem perceber que a origem mudou. Rejeitada por sacrificar a corretude.
- **Baixar e comparar o hash antes de gravar:** garante idempotência de escrita, mas não evita o fetch, que é o principal custo. Fica só como fallback.
- **Verificar o sha256 de cada parte relendo o conteúdo a cada rodada:** correto, mas caro em memória e tempo para arquivos grandes, contrariando o [ADR 0010](0010-selecao-e-arquivos-grandes.md). Preferimos a checagem por tamanho.
