-- Contrato conab-safras.produtos.v1.
-- Resumo nacional de safra de grãos da CONAB (fonte: Levantamento de Safra).
SELECT
    NULLIF(TRIM(produto), '') AS produto,
    NULLIF(TRIM(safra), '') AS safra,
    TRY_CAST(NULLIF(TRIM(area_mil_ha), '') AS DOUBLE) AS area_mil_ha,
    TRY_CAST(NULLIF(TRIM(produtividade_kg_ha), '') AS DOUBLE) AS produtividade_kg_ha,
    TRY_CAST(NULLIF(TRIM(producao_mil_t), '') AS DOUBLE) AS producao_mil_t,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM bronze
