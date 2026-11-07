-- Contrato tse.despesa.paga.2024.v1: primeira linha de cópias integrais, conflito interrompe.
WITH origem AS (
    SELECT
        *,
        TRIM("AA_ELEICAO") AS ano_texto,
        TRIM("CD_ELEICAO") AS codigo_eleicao,
        TRIM("SQ_PRESTADOR_CONTAS") AS sequencia_prestador,
        TRIM("SQ_DESPESA") AS despesa_texto,
        TRIM("SQ_PARCELAMENTO_DESPESA") AS pagamento_texto,
        TRIM("DT_PAGTO_DESPESA") AS data_texto,
        TRIM("VR_PAGTO_DESPESA") AS valor_texto,
        COUNT(*) OVER (
            PARTITION BY TRIM("CD_ELEICAO"), TRIM("SQ_PRESTADOR_CONTAS"),
                         TRY_CAST(TRIM("SQ_DESPESA") AS UINT),
                         TRY_CAST(TRIM("SQ_PARCELAMENTO_DESPESA") AS UINT)
        ) AS ocorrencias_chave,
        MIN(_tdoa_json_original) OVER (
            PARTITION BY TRIM("CD_ELEICAO"), TRIM("SQ_PRESTADOR_CONTAS"),
                         TRY_CAST(TRIM("SQ_DESPESA") AS UINT),
                         TRY_CAST(TRIM("SQ_PARCELAMENTO_DESPESA") AS UINT)
        ) AS original_minimo,
        MAX(_tdoa_json_original) OVER (
            PARTITION BY TRIM("CD_ELEICAO"), TRIM("SQ_PRESTADOR_CONTAS"),
                         TRY_CAST(TRIM("SQ_DESPESA") AS UINT),
                         TRY_CAST(TRIM("SQ_PARCELAMENTO_DESPESA") AS UINT)
        ) AS original_maximo,
        ROW_NUMBER() OVER (
            PARTITION BY TRIM("CD_ELEICAO"), TRIM("SQ_PRESTADOR_CONTAS"),
                         TRY_CAST(TRIM("SQ_DESPESA") AS UINT),
                         TRY_CAST(TRIM("SQ_PARCELAMENTO_DESPESA") AS UINT)
            ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
        ) AS ordem_copia
    FROM bronze
), preparados AS (
    SELECT *,
        CASE WHEN regexp_like(data_texto, '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
            THEN regexp_replace(data_texto, '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1')
        END AS data_iso,
        regexp_replace(valor_texto, ',', '.', 'g') AS valor_normalizado
    FROM origem
), validados AS (
    SELECT *
    FROM preparados
    WHERE tdoa_assert_true(TRY_CAST(ano_texto AS SMALLINT) = 2024)
      AND tdoa_assert_true(codigo_eleicao <> '' AND sequencia_prestador <> '')
      AND tdoa_assert_true(regexp_like(despesa_texto, '^[0-9]+$') AND TRY_CAST(despesa_texto AS UINT) IS NOT NULL)
      AND tdoa_assert_true(regexp_like(pagamento_texto, '^[0-9]+$') AND TRY_CAST(pagamento_texto AS UINT) IS NOT NULL)
      AND tdoa_assert_true(original_minimo = original_maximo)
      AND tdoa_assert_true(regexp_like(data_texto, '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
          AND arrow_try_cast(data_iso, 'Date32') IS NOT NULL)
      AND tdoa_assert_true(regexp_like(valor_texto, '^[0-9]+,[0-9]{2}$')
          AND TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) IS NOT NULL)
)
SELECT
    arrow_try_cast('2024', 'UInt16') AS ano_eleicao,
    "CD_ELEICAO" AS codigo_eleicao,
    "SQ_PRESTADOR_CONTAS" AS sequencia_prestador_contas,
    arrow_try_cast(despesa_texto, 'UInt32') AS sequencia_despesa,
    arrow_try_cast(pagamento_texto, 'UInt32') AS sequencia_pagamento,
    arrow_try_cast(data_iso, 'Date32') AS data_pagamento,
    arrow_try_cast(valor_normalizado, 'Decimal128(18,2)') AS valor_pago,
    _tdoa_json_original AS registro_original
FROM validados
WHERE tdoa_assert_true(ordem_copia = 1)
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
