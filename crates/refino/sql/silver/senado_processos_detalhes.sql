-- Contrato senado.processo.detalhe.v1.
-- __tdoa_json_original preserva todas as colunas, inclusive campos novos da API.
WITH identificados AS (
    SELECT
        NULLIF(TRIM(id), '') AS id,
        CASE
            WHEN regexp_like(TRIM(ano), '^[0-9]{4}$') THEN arrow_try_cast(TRIM(ano), 'UInt16')
        END AS ano,
        _tdoa_json_original AS processo_original,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal,
        COUNT(*) OVER (PARTITION BY NULLIF(TRIM(id), '')) AS ocorrencias_id
    FROM bronze
)
SELECT
    CASE WHEN ocorrencias_id = 1 THEN id END AS id,
    ano,
    processo_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM identificados
