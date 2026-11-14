-- Contrato undp.indices-compostos.v1.
-- O CSV wide do UNDP traz colunas {indicador}_{ano} por país. O Bronze grava
-- todas como Utf8. Aqui extraímos os identificadores do país e preservamos o
-- registro original como JSON para unpivot no consumo.
SELECT
    NULLIF(TRIM(iso3), '') AS iso3,
    NULLIF(TRIM(country), '') AS pais,
    NULLIF(TRIM(hdicode), '') AS codigo_hdi,
    NULLIF(TRIM(region), '') AS regiao,
    TRY_CAST(NULLIF(TRIM(hdi_rank_2023), '') AS INT) AS ranking_hdi_2023,
    TRY_CAST(NULLIF(TRIM(hdi_2023), '') AS DECIMAL(10, 4)) AS hdi_2023,
    TRY_CAST(NULLIF(TRIM(le_2023), '') AS DECIMAL(10, 2)) AS expectativa_vida_2023,
    TRY_CAST(NULLIF(TRIM(eys_2023), '') AS DECIMAL(10, 2)) AS anos_escolaridade_esperados_2023,
    TRY_CAST(NULLIF(TRIM(mys_2023), '') AS DECIMAL(10, 2)) AS anos_escolaridade_media_2023,
    TRY_CAST(NULLIF(TRIM(gnipc_2023), '') AS DECIMAL(18, 2)) AS renda_per_capita_2023,
    TRY_CAST(NULLIF(TRIM(gdi_2023), '') AS DECIMAL(10, 4)) AS gdi_2023,
    TRY_CAST(NULLIF(TRIM(gii_2023), '') AS DECIMAL(10, 4)) AS gii_2023,
    TRY_CAST(NULLIF(TRIM(phdi_2023), '') AS DECIMAL(10, 4)) AS phdi_2023,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM bronze
