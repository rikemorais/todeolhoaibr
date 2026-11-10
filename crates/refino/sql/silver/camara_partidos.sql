-- Contrato camara.partido.v1.
WITH classificados AS (
    SELECT *, COUNT(*) OVER (PARTITION BY id) AS ocorrencias_id
    FROM bronze
)
SELECT
    CASE WHEN ocorrencias_id = 1 AND NULLIF(TRIM(id), '') IS NOT NULL THEN id END AS id,
    sigla,
    nome,
    uri,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM classificados
