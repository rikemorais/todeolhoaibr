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
- Para APIs paginadas, a releitura usa **requisição condicional** (`If-None-Match`/`If-Modified-Since`) por página: 304 reaproveita a parte já gravada, 200 reingere aquela página. O manifesto novo é remontado a partir das partes reaproveitadas e reingeridas.

**Fallback onde não há validador**

- Origem sem validador, ou que ignora o condicional: cai para a comparação por **tamanho**, e, por fim, pela **assinatura (sha256)** registrada no manifesto, para preservar a idempotência de escrita mesmo quando o fetch não pode ser evitado.

**Integridade pelo tamanho, barata**

- O manifesto, gravado por último, é o marcador de completude ([ADR 0006](0006-layout-da-camada-raw.md)).
- A integridade das partes é conferida pelo **tamanho** (`ArmazenamentoRaw::tamanho`, um `HEAD`), não relendo o conteúdo. Reler um ZIP de centenas de MB a cada rodada estouraria o limite de 256 MB do contêiner ([ADR 0010](0010-selecao-e-arquivos-grandes.md)). O `sha256` do manifesto continua sendo a procedência gravada na ingestão, usada na comparação com a origem, não para re-hashear o arquivo local.

## Consequências

- O porto `ClienteHttp` ganha requisição condicional, e `ArmazenamentoRaw` ganha `tamanho`. As implementações de infraestrutura passam a usar `HEAD`.
- `_ultima_versao.json` passa a valer também para as APIs paginadas, não só para arquivos. Mantém a exceção à imutabilidade já aberta no [ADR 0010](0010-selecao-e-arquivos-grandes.md): é ponteiro de controle, não dado ingerido.
- A integridade por tamanho detecta parte ausente ou truncada, mas não corrupção que preserve o tamanho. Aceitamos isso em troca de uma pré-checagem barata e segura em memória.
- APIs paginadas por **cursor**, cujas URLs mudam a cada rodada, não casam página a página; esses recursos caem no fallback por assinatura, com menos economia de banda.
- Como no [ADR 0010](0010-selecao-e-arquivos-grandes.md), o validador indica mudança no arquivo, não no conteúdo: uma republicação idêntica com novo ETag é reingerida.
- `_ultima_versao.json` guarda o `iniciada_em` da execução e é publicado por `publicar_ponteiro`, que não deixa uma rodada mais antiga sobrescrever uma mais nova (ver issue #16). A atomicidade sob escritas estritamente simultâneas exigiria escrita condicional (CAS) no porto, ainda não disponível no cliente S3; o guard por recência resolve o caso sequencial sem introduzir regressão.

## Alternativas consideradas

- **Política de frescor por tempo** (pular se ingerido há menos de N dias): barata, mas pode servir dado velho sem perceber que a origem mudou. Rejeitada por sacrificar a corretude.
- **Baixar e comparar o hash antes de gravar:** garante idempotência de escrita, mas não evita o fetch, que é o principal custo. Fica só como fallback.
- **Verificar o sha256 de cada parte relendo o conteúdo a cada rodada:** correto, mas caro em memória e tempo para arquivos grandes, contrariando o [ADR 0010](0010-selecao-e-arquivos-grandes.md). Preferimos a checagem por tamanho.
