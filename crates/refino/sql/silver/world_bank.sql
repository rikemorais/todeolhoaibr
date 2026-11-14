-- Contrato world-bank.indicador.v1.
-- Extrai país (ISO alpha-3), ano, valor e código do indicador dos registros
-- achatados do JSON da API v2. Campos `indicator` e `country` chegam como
-- JSON compacto do decodificador; o código e o nome são extraídos via
-- tdoa_json_get.
-- A API não inclui `unit` neste recorte; manter a coluna contratual como nula.
SELECT
    tdoa_json_get(indicator, 'id') AS codigo_indicador,
    tdoa_json_get(indicator, 'value') AS nome_indicador,
    tdoa_json_get(country, 'id') AS codigo_pais,
    tdoa_json_get(country, 'value') AS nome_pais,
    NULLIF(TRIM(countryiso3code), '') AS iso3,
    CAST(NULLIF(TRIM(date), '') AS INT) AS ano,
    TRY_CAST(value AS DECIMAL(18, 6)) AS valor,
    CAST(NULL AS VARCHAR) AS unidade,
    NULLIF(TRIM(obs_status), '') AS status_observacao,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM bronze
