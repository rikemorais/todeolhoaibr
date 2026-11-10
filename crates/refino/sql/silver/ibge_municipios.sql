-- Contrato ibge.municipio.v1: uma linha por município do IBGE.
WITH origem AS (
    SELECT
        TRIM("id") AS id,
        TRIM("nome") AS nome,
        tdoa_json_get("microrregiao", 'id') AS microrregiao_id,
        tdoa_json_get("microrregiao", 'nome') AS microrregiao_nome,
        tdoa_json_get(tdoa_json_get("microrregiao", 'mesorregiao'), 'id') AS mesorregiao_id,
        tdoa_json_get(tdoa_json_get("microrregiao", 'mesorregiao'), 'nome') AS mesorregiao_nome,
        tdoa_json_get(tdoa_json_get(tdoa_json_get("microrregiao", 'mesorregiao'), 'UF'), 'id') AS uf_id,
        tdoa_json_get(tdoa_json_get(tdoa_json_get("microrregiao", 'mesorregiao'), 'UF'), 'sigla') AS uf_sigla,
        tdoa_json_get("regiao-imediata", 'id') AS regiao_imediata_id,
        tdoa_json_get("regiao-imediata", 'nome') AS regiao_imediata_nome,
        tdoa_json_get(tdoa_json_get("regiao-imediata", 'regiao-intermediaria'), 'id') AS regiao_intermediaria_id,
        tdoa_json_get(tdoa_json_get("regiao-imediata", 'regiao-intermediaria'), 'nome') AS regiao_intermediaria_nome,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal,
        COUNT(*) OVER (PARTITION BY TRIM("id")) AS ocorrencias
    FROM bronze
)
SELECT
    id,
    nome,
    microrregiao_id,
    microrregiao_nome,
    mesorregiao_id,
    mesorregiao_nome,
    uf_id,
    uf_sigla,
    regiao_imediata_id,
    regiao_imediata_nome,
    regiao_intermediaria_id,
    regiao_intermediaria_nome,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM origem
WHERE tdoa_assert_true_named(
    id IS NOT NULL AND TRIM(id) <> '', 'id_obrigatorio')
  AND tdoa_assert_true_named(ocorrencias = 1, 'id_unico')
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
