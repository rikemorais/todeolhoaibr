# 0020. Contrato e publicação da camada silver

- **Status:** Proposto
- **Data:** 2026-11-05

## Contexto

O bronze guarda Parquet com colunas textuais e conserva a granularidade da origem. A silver precisa tipar, limpar e deduplicar, mas as fontes variam em grão, formato e nível geográfico. O padrão de manifesto e ponteiro já permite publicar versões sem sobrescrever a montante.

## Decisão proposta

Cada **tabela silver** terá contrato versionado com grão, chave de negócio, tipos, campos obrigatórios, política de nulos e duplicatas e regras de qualidade. O catálogo silver apontará para uma ou mais tabelas bronze, sem presumir que uma entrada do catálogo bronze produza uma única tabela.

O bucket `silver` usará partes Parquet, manifesto e ponteiro de versão vigente. O manifesto identificará o recurso, a tabela e a versão do bronze de entrada, a versão do contrato/regra e as contagens de entrada, saída, duplicatas e rejeições por motivo. O ponteiro só será publicado após todas as partes e o manifesto estarem íntegros. Uma nova versão do bronze **ou da regra** exige nova materialização; versões anteriores continuam disponíveis para consulta ou recuperação. Nenhuma transformação altera raw ou bronze.

Rejeições serão observáveis e recuperáveis, com motivo e referência à origem. A política exata de armazenamento e os limites aceitáveis de rejeição serão definidos por contrato antes da implementação. Não descartar linhas silenciosamente.

O código IBGE do município será texto, nunca número. Tabelas nacionais, estaduais ou regionais manterão seu nível geográfico original; não receberão município artificial. A conformação dentro da fonte pertence à silver. Cruzamentos entre domínios e indicadores derivados pertencem à gold, conforme [ADR 0015](0015-arquitetura-de-dados-em-medalhao.md).

## Consequências

- Mudanças de schema e regra são rastreáveis e podem ser reprocessadas sem nova ingestão.
- O volume armazenado cresce com versões e registros rejeitados.
- Contratos e política de qualidade precisam ser validados com amostras de anos e formatos distintos antes de cada família entrar em produção.

## Alternativas consideradas

- **Inferir schema e chave automaticamente de cada Parquet:** simplifica o início, mas mudanças da origem poderiam alterar silenciosamente o contrato silver.
- **Usar apenas a versão do bronze para idempotência:** deixaria correções de regras sem gatilho de reprocessamento.
- **Forçar município + ano em todas as tabelas:** perderia dados nacionais, estaduais e eventos sem município; a chave é adequada a produtos analíticos posteriores, não a todo registro silver.
