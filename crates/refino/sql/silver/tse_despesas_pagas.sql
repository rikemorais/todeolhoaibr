-- Contrato tse.despesa.paga.2024.v1: agrupa por chave, remove cópias integrais e falha em conflitos.
WITH origem AS (
    SELECT
        TRIM("AA_ELEICAO") AS ano_texto,
        TRIM("CD_ELEICAO") AS codigo_eleicao_chave,
        TRIM("SQ_PRESTADOR_CONTAS") AS prestador_chave,
        arrow_try_cast(TRIM("SQ_DESPESA"), 'UInt32') AS despesa_chave,
        arrow_try_cast(TRIM("SQ_PARCELAMENTO_DESPESA"), 'UInt32') AS pagamento_chave,
        "CD_ELEICAO" AS codigo_eleicao_original,
        "SQ_PRESTADOR_CONTAS" AS prestador_original,
        TRIM("DT_PAGTO_DESPESA") AS data_texto,
        TRIM("VR_PAGTO_DESPESA") AS valor_texto,
        CASE WHEN regexp_like(TRIM("DT_PAGTO_DESPESA"), '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
            THEN regexp_replace(TRIM("DT_PAGTO_DESPESA"), '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1')
        END AS data_iso,
        regexp_replace(TRIM("VR_PAGTO_DESPESA"), ',', '.', 'g') AS valor_normalizado,
        _tdoa_json_original AS original,
        sha256(_tdoa_json_original) AS original_sha256
    FROM bronze
    WHERE regexp_like(upper(_tdoa_origem_url), 'DESPESAS_PAGAS_CANDIDATOS_2024_BRASIL([.]CSV|[.]TXT)?$')
), tipadas AS (
    SELECT *,
        arrow_try_cast(data_iso, 'Date32') AS data_pagamento_tipada,
        CASE WHEN regexp_like(valor_texto, '^[0-9]+,[0-9]{2}$')
            THEN TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) END AS valor_pago_tipada
    FROM origem
), agrupadas AS (
    SELECT
        ano_texto,
        codigo_eleicao_chave,
        prestador_chave,
        despesa_chave,
        pagamento_chave,
        COUNT(*) AS ocorrencias_chave,
        COUNT(ano_texto) AS anos_preenchidos,
        COUNT(data_pagamento_tipada) AS datas_validas,
        COUNT(valor_pago_tipada) AS valores_validos,
        MIN(original_sha256) AS hash_original_minimo,
        MAX(original_sha256) AS hash_original_maximo,
        MIN(codigo_eleicao_original) AS codigo_eleicao,
        MIN(prestador_original) AS sequencia_prestador,
        MIN(data_pagamento_tipada) AS data_pagamento,
        MIN(valor_pago_tipada) AS valor_pago,
        MIN(original) AS registro_original,
        MIN(_tdoa_origem_manifesto) AS _tdoa_origem_manifesto,
        MIN(_tdoa_origem_parte) AS _tdoa_origem_parte,
        MIN(_tdoa_origem_ordinal) AS _tdoa_origem_ordinal
    FROM tipadas
    GROUP BY ano_texto, codigo_eleicao_chave, prestador_chave, despesa_chave, pagamento_chave
), validadas AS (
    SELECT *
    FROM agrupadas
    WHERE tdoa_assert_true_named(ano_texto = '2024' AND anos_preenchidos = ocorrencias_chave, 'ano_do_recurso')
      AND tdoa_assert_true_named(codigo_eleicao_chave NOT IN ('', '-1', '-3')
          AND prestador_chave NOT IN ('', '-1', '-3'), 'campos_da_chave')
      AND tdoa_assert_true_named(despesa_chave IS NOT NULL AND pagamento_chave IS NOT NULL, 'sequencias_numericas')
      AND tdoa_assert_true_named(datas_validas = ocorrencias_chave, 'data_pagamento')
      AND tdoa_assert_true_named(valores_validos = ocorrencias_chave, 'valor_pago')
      AND tdoa_assert_true_named(hash_original_minimo = hash_original_maximo, 'chave_sem_conflito')
)
SELECT
    arrow_try_cast('2024', 'UInt16') AS ano_eleicao,
    codigo_eleicao,
    sequencia_prestador,
    despesa_chave AS sequencia_despesa,
    pagamento_chave AS sequencia_pagamento,
    data_pagamento,
    valor_pago,
    registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM validadas
