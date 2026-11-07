-- Contrato camara.partido.v1.
WITH classificados AS (
    SELECT *, COUNT(*) OVER (PARTITION BY id) AS ocorrencias_id
    FROM bronze
)
SELECT
    CASE WHEN ocorrencias_id = 1 AND NULLIF(TRIM(id), '') IS NOT NULL THEN id END AS id,
    sigla,
    nome,
    uri
FROM classificados
