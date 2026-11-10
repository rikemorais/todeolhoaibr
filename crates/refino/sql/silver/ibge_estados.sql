-- Contrato ibge.estado.v1: uma linha por unidade federativa do IBGE.
WITH origem AS (
    SELECT
        TRIM("id") AS id,
        TRIM("sigla") AS sigla,
        TRIM("nome") AS nome,
        tdoa_json_get("regiao", 'id') AS regiao_id,
        tdoa_json_get("regiao", 'sigla') AS regiao_sigla,
        tdoa_json_get("regiao", 'nome') AS regiao_nome,
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
    regiao_id,
    regiao_sigla,
    regiao_nome,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM origem
WHERE tdoa_assert_true_named(
    id IS NOT NULL AND TRIM(id) <> '', 'id_obrigatorio')
  AND tdoa_assert_true_named(ocorrencias = 1, 'id_unico')
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
