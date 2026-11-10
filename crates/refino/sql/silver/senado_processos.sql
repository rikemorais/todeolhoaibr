-- Contrato senado.processo.v1.
-- O ano vem do recurso Bronze; __tdoa_json_original contém a união dos campos
-- variáveis da origem, serializada pelo adaptador do executor SQL.
WITH identificados AS (
    SELECT
        NULLIF(TRIM(id), '') AS id,
        _tdoa_json_original AS processo_original,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal,
        COUNT(*) OVER (PARTITION BY NULLIF(TRIM(id), '')) AS ocorrencias_id
    FROM bronze
)
SELECT
    CASE WHEN ocorrencias_id = 1 THEN id END AS id,
    arrow_try_cast('{{ANO_RECURSO}}', 'UInt16') AS ano,
    processo_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM identificados
