-- Contrato tse.prestacao_contas.despesa_paga.2022.v1.
-- O consolidado medido não tem chave repetida; preserve cada ocorrência e use linhagem técnica.
WITH origem AS (
    SELECT
        TRIM("AA_ELEICAO") AS ano_texto,
        TRIM("CD_ELEICAO") AS codigo_eleicao,
        TRIM("SQ_PRESTADOR_CONTAS") AS sequencia_prestador,
        TRIM("SQ_DESPESA") AS sequencia_despesa_texto,
        TRIM("SQ_PARCELAMENTO_DESPESA") AS sequencia_pagamento_texto,
        TRIM("VR_PAGTO_DESPESA") AS valor_texto,
        CASE
            WHEN regexp_like(TRIM("DT_PAGTO_DESPESA"), '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
                THEN regexp_replace(TRIM("DT_PAGTO_DESPESA"), '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1')
        END AS data_iso,
        regexp_replace(TRIM("VR_PAGTO_DESPESA"), ',', '.', 'g') AS valor_normalizado,
        _tdoa_json_original AS registro_original,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
    WHERE regexp_like(upper(_tdoa_origem_url), 'DESPESAS_PAGAS_CANDIDATOS_2022_BRASIL([.]CSV|[.]TXT)?$')
), tipadas AS (
    SELECT *,
        arrow_try_cast(ano_texto, 'UInt16') AS ano_eleicao,
        arrow_try_cast(sequencia_despesa_texto, 'UInt32') AS sequencia_despesa,
        arrow_try_cast(sequencia_pagamento_texto, 'UInt32') AS sequencia_pagamento,
        arrow_try_cast(data_iso, 'Date32') AS data_pagamento,
        CASE WHEN regexp_like(valor_texto, '^[0-9]+,[0-9]{2}$')
            THEN TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) END AS valor_pago
    FROM origem
)
SELECT
    ano_eleicao,
    codigo_eleicao,
    sequencia_prestador,
    sequencia_despesa,
    sequencia_pagamento,
    data_pagamento,
    valor_pago,
    registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipadas
WHERE tdoa_assert_true_named(ano_eleicao = 2022, 'ano_do_recurso')
  AND tdoa_assert_true_named(codigo_eleicao NOT IN ('', '-1', '-3'), 'codigo_eleicao')
  AND tdoa_assert_true_named(sequencia_prestador NOT IN ('', '-1', '-3'), 'sequencia_prestador')
  AND tdoa_assert_true_named(sequencia_despesa IS NOT NULL, 'sequencia_despesa')
  AND tdoa_assert_true_named(sequencia_pagamento IS NOT NULL, 'sequencia_pagamento')
  AND tdoa_assert_true_named(data_pagamento IS NOT NULL, 'data_pagamento')
  AND tdoa_assert_true_named(valor_pago IS NOT NULL, 'valor_pago')
