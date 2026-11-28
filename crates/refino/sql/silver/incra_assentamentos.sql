-- Contrato incra-assentamentos.projetos.v1.
-- Assentamentos da reforma agrária (SIPRA/INCRA).
SELECT
    NULLIF(TRIM(codigo_projeto), '') AS codigo_projeto,
    NULLIF(TRIM(nome_projeto), '') AS nome_projeto,
    NULLIF(TRIM(municipio_sede), '') AS municipio_sede,
    TRY_CAST(REPLACE(REPLACE(NULLIF(TRIM(area_ha), ''), '.', ''), ',', '.') AS DOUBLE) AS area_ha,
    TRY_CAST(NULLIF(TRIM(familias_assentadas), '') AS BIGINT) AS familias_assentadas,
    TRY_CAST(NULLIF(TRIM(familias_assent), '') AS BIGINT) AS familias_assent,
    NULLIF(TRIM(fase), '') AS fase,
    NULLIF(TRIM(tipo), '') AS tipo,
    NULLIF(TRIM(numero), '') AS numero,
    NULLIF(TRIM(data_criacao), '') AS data_criacao,
    NULLIF(TRIM(forma_obtencao), '') AS forma_obtencao,
    NULLIF(TRIM(data_obtencao), '') AS data_obtencao,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM bronze
