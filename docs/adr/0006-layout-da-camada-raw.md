# 0006. Layout imutável da camada Raw, com manifesto

- **Status:** Aceito
- **Data:** 2026-10-31

## Contexto

As etapas de transformação ainda serão definidas, mas vão depender da Raw. Ela precisa preservar o que
as APIs responderam, permitir reprocessar o passado e deixar claro se uma ingestão terminou. APIs
públicas também mudam de conteúdo sem aviso, então o histórico tem valor por si só.

## Decisão

Cada resposta é gravada byte a byte, sem interpretação nem alteração, em:

```text
<bucket>/<fonte>/<recurso>/data_ingestao=AAAA-MM-DD/execucao=<id>/parte-NNNNN.<ext>
<bucket>/<fonte>/<recurso>/data_ingestao=AAAA-MM-DD/execucao=<id>/_manifesto.json
```

- Toda execução cria um novo prefixo. Nada é sobrescrito.
- O `_manifesto.json` é gravado por último e **marca a ingestão como completa**. Um prefixo sem manifesto
  é de uma ingestão interrompida e não deve ser consumido.
- O manifesto registra a procedência: fonte, recurso, execução, horários e, para cada parte, a URL de
  origem, o tamanho, o SHA-256 e o tipo de conteúdo.
- Respostas de erro não são gravadas. Depois de esgotar as retentativas, a tarefa falha.
- Data e horários são sempre em UTC.
- O nome de cada caminho vem de `LocalizacaoRaw`, no `nucleo`, para que a transformação reutilize o
  mesmo contrato.

## Consequências

- Reprocessar é possível sem acessar as APIs de novo, e o histórico permite comparar datas.
- O padrão `chave=valor` é entendido por ferramentas como DuckDB e Spark.
- Sem retenção, o volume só cresce, inclusive quando nada mudou na origem. Política de retenção ou
  deduplicação pelo hash ficam para quando houver necessidade real.
- Cada resposta fica inteira em memória antes da gravação. Funciona para as respostas atuais (2,5 MB no
  maior caso). Fontes com arquivos grandes vão exigir streaming.

## Alternativas consideradas

- **Sobrescrever com a versão mais recente:** economiza espaço, mas perde o histórico.
- **Converter para Parquet ou agrupar as páginas em um único JSON:** já é transformação e impede
  verificar o dado contra a origem.
- **Particionar só por data:** duas execuções no mesmo dia se misturariam.
