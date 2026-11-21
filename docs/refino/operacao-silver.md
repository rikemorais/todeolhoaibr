# Operação da camada Silver

Este runbook cobre reconciliação, reprocessamento, inspeção e recuperação. As regras de publicação e os contratos por fonte ficam em [silver.md](silver.md). O diagrama Raw → Bronze → Silver → Gold → Diamond está em [camadas-dados.svg](../img/camadas-dados.svg).

## Reconciliação

O pipeline é somente de avanço: Silver lê o manifesto Bronze vigente e nunca altera Raw ou Bronze. Publique primeiro o Bronze mais recente e depois a família Silver desejada:

```bash
docker compose run --rm refino refinar bronze --todas
docker compose run --rm refino refinar silver FONTE
```

Para reconciliar todas as famílias habilitadas, inclusive as de maior volume:

```bash
docker compose run --rm refino refinar silver --todas
```

`silver --todas` não inclui `tse-votacao-secao`, que tem dezenas de GB. Rode essa família de forma explícita e use os filtros documentados em [silver.md](silver.md#execução-local). A família `tse-prestacao-contas` também é explícita por ter múltiplas relações internas e anos com contratos distintos.

Uma execução sem mudança no snapshot Bronze nem na versão da regra é ignorada. A alteração de dados de origem é reconciliada quando o Bronze recebe novo manifesto; uma mudança semântica no SQL exige incrementar `REGRA` no adaptador Rust. A combinação `id_execucao_bronze|regra=...` é a versão publicada.

## Conferir a publicação vigente

Suba o console DuckDB local, que gera views lendo apenas os ponteiros Silver vigentes:

```bash
docker compose up -d consulta
docker compose run --rm consulta sql
```

As views têm o formato `silver_<fonte>_<recurso>`. Os segmentos particionados são consolidados na view, mas execuções antigas não são globbed. A camada `Qualidade` lista perfis `qualidade_silver_<fonte>_<recurso>` gerados a partir dos mesmos manifestos vigentes. Selecione uma tabela no console ou consulte seu perfil em SQL. Troque os identificadores de exemplo pelos que aparecem no catálogo:

```sql
-- Schema e contagem corrente
DESCRIBE silver_bcb_series;
SELECT count(*) AS linhas FROM silver_bcb_series;

-- Nulos em campos obrigatórios e referências de linhagem
SELECT
  countif(data IS NULL) AS data_nula,
  countif(valor IS NULL) AS valor_nulo,
  countif(_tdoa_origem_manifesto IS NULL) AS manifesto_nulo,
  countif(_tdoa_origem_parte IS NULL) AS parte_nula,
  countif(_tdoa_origem_ordinal IS NULL) AS ordinal_nulo
FROM silver_bcb_series;

-- Exemplo de unicidade: adapte a chave à tabela consultada
SELECT serie, data, count(*) AS ocorrencias
FROM silver_bcb_series
GROUP BY serie, data
HAVING count(*) > 1;

-- Perfil atual com nulos, linhagem, chave candidata e contagens do manifesto
SELECT linhas, linhas_saida_manifesto, linhas_rejeitadas_manifesto, copias_removidas_manifesto,
       conversoes_opcionais_por_motivo_manifesto, status_metricas_manifesto
FROM qualidade_silver_bcb_series;
```

O perfil calcula contagem de linhas, nulos por coluna, linhas com linhagem incompleta e, para chaves candidatas registradas, linhas com chave incompleta e linhas excedentes por duplicidade. Chaves não registradas aparecem como não validadas; use a matriz em [silver.md](silver.md#cobertura-do-catálogo-bronze) para a política por tabela. O perfil é lazy: executar uma view faz a varredura da tabela atual selecionada e pode levar mais tempo em recursos volumosos. Ele nunca remove linhas. `DESCRIBE`, o perfil, a verificação de unicidade pela chave e a conferência da linhagem constituem o smoke por tabela.

O painel de execução mostra estado, duração, partes e bytes; o console DuckDB expõe os perfis das tabelas vigentes. Os campos `*_manifesto` somam somente manifestos vigentes com `qualidade` v2 completo. Se algum recurso ainda usa um manifesto antigo, ficam nulos e `status_metricas_manifesto` explica por quê; reprocessar com uma nova versão de regra cria métricas. Rejeições e conversões aparecem em mapas JSON por motivo quando o contrato SQL as anota; cópias removidas são contadas apenas se o contrato as marcar. Asserções obrigatórias continuam falhando antes da troca do ponteiro.

## Reprocessar

1. Confirme primeiro o snapshot Bronze vigente e o contrato a corrigir.
2. Se a origem mudou, rode `docker compose run --rm refino refinar bronze FONTE` e confirme que o ponteiro Bronze avançou.
3. Se a regra mudou, incremente sua constante `REGRA` no adaptador e rode `docker compose run --rm refino refinar silver FONTE`.
4. Confira schema, contagens, chave, nulos e linhagem na view DuckDB e confira a execução no painel.

Não altere ou remova arquivos Raw/Bronze para forçar reprocessamento. Não edite manifesto nem `_ultima_versao.json` manualmente durante uma execução. Um novo manifesto Bronze ou uma nova versão de regra já invalida a versão Silver e aciona a publicação.

## Falha e rollback

A publicação grava Parquet e manifesto sob uma nova execução e troca `_ultima_versao.json` somente no fim, sob guarda. Falha de SQL, schema, upload ou mudança concorrente do ponteiro Bronze deixa a versão Silver anterior apontada; objetos da tentativa podem permanecer órfãos e são seguros para limpeza posterior, após confirmar que nenhum ponteiro os referencia.

1. Interrompa o agendador Silver ou a tarefa que está repetindo falha.
2. Consulte o estado vigente no console e anote a chave do manifesto atual e sua versão `bronze|regra`.
3. Corrija a origem Bronze ou o SQL/versão da regra e execute uma reconciliação manual.
4. Se a nova publicação concluiu, mas a validação funcional falhou, interrompa novas execuções e restaure o conteúdo anterior do ponteiro Silver usando o cliente S3/backup operacional do ambiente. Aponte-o para o manifesto anterior que continua imutável; não regrave Parquet nem manifesto histórico.
5. Reabra as consultas DuckDB e confirme que elas leem a versão restaurada. Depois, reative o agendador somente quando a correção estiver pronta.

O repositório ainda não oferece um comando de rollback automático. A restauração manual do ponteiro requer acesso de escrita ao bucket Silver e deve ser feita por operador autorizado com backup do objeto atual. Raw e Bronze permanecem intactos durante todo o procedimento.

## Smoke local e recuperação

Para cada família, registre fonte/recurso, versão dos ponteiros Bronze e Silver, execução, schema, linhas, bytes, nulos, duplicatas da chave e linhagem. Execute a mesma reconciliação uma segunda vez: sem alteração de origem/regra, não deve haver nova publicação. Em ambiente descartável, provoque uma falha de contrato e confirme que a versão Silver anterior continua vigente; os testes automatizados de `tarefa_sql` cobrem erro SQL, mudança concorrente no Bronze e cancelamento durante upload.

Não faça esse ensaio destrutivo contra um bucket que contenha dados de usuário. Use armazenamento local descartável ou os testes com `ArmazenamentoEmMemoria`.
