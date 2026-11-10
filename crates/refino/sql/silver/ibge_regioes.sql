-- Contrato ibge.regiao.v1: uma linha por região geográfica do IBGE.
WITH origem AS (
    SELECT
        TRIM("id") AS id,
        TRIM("sigla") AS sigla,
        TRIM("nome") AS nome,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal,
        COUNT(*) OVER (PARTITION BY TRIM("id")) AS ocorrencias
    FROM bronze
)
SELECT
    id,
    sigla,
    nome,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM origem
WHERE tdoa_assert_true_named(
    id IS NOT NULL AND TRIM(id) <> '', 'id_obrigatorio')
  AND tdoa_assert_true_named(ocorrencias = 1, 'id_unico')
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
