-- Contrato emendas-tesouro.pagamento-municipal.v1.
-- A origem não traz autor parlamentar; cada ocorrência e sua OB são preservadas.
WITH entrada AS (
    SELECT
        NULLIF(TRIM("Nome Ente"), '') AS nome_ente,
        NULLIF(TRIM("UF"), '') AS uf,
        NULLIF(TRIM("Código Siafi"), '') AS codigo_siafi,
        NULLIF(TRIM("Código IBGE"), '') AS codigo_ibge,
        NULLIF(TRIM("Data"), '') AS data_origem,
        NULLIF(TRIM("Ano"), '') AS ano_origem,
        NULLIF(TRIM("Mês"), '') AS mes_origem,
        NULLIF(TRIM("Tipo Ente"), '') AS tipo_ente,
        NULLIF(TRIM("OB"), '') AS ordem_bancaria,
        NULLIF(TRIM("CNPJ do Favorecido"), '') AS cnpj_favorecido,
        NULLIF(TRIM("Nome Favorecido"), '') AS nome_favorecido,
        NULLIF(TRIM("Nome Emenda"), '') AS nome_emenda,
        CAST(NULL AS VARCHAR) AS tipo_emenda,
        NULLIF(TRIM("Transferência Especial"), '') AS transferencia_especial_origem,
        NULLIF(TRIM("Categoria Econômica Despesa"), '') AS categoria_economica,
        NULLIF(TRIM("Valor"), '') AS valor_origem,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
), tipada AS (
    SELECT
        nome_ente,
        uf,
        codigo_siafi,
        codigo_ibge,
        data_origem,
        TRY_CAST(ano_origem AS SMALLINT) AS ano,
        COALESCE(
            TRY_CAST(mes_origem AS TINYINT),
            CASE LOWER(mes_origem)
                WHEN 'janeiro' THEN 1
                WHEN 'fevereiro' THEN 2
                WHEN 'março' THEN 3
                WHEN 'abril' THEN 4
                WHEN 'maio' THEN 5
                WHEN 'junho' THEN 6
                WHEN 'julho' THEN 7
                WHEN 'agosto' THEN 8
                WHEN 'setembro' THEN 9
                WHEN 'outubro' THEN 10
                WHEN 'novembro' THEN 11
                WHEN 'dezembro' THEN 12
            END
        ) AS mes,
        tipo_ente,
        ordem_bancaria,
        cnpj_favorecido,
        nome_favorecido,
        nome_emenda,
        tipo_emenda,
        transferencia_especial_origem,
        categoria_economica,
        TRY_CAST(REPLACE(valor_origem, ',', '.') AS DECIMAL(20, 2)) AS valor,
        ano_origem,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM entrada
)
SELECT
    nome_ente,
    uf,
    codigo_siafi,
    codigo_ibge,
    data_origem,
    ano,
    mes,
    tipo_ente,
    ordem_bancaria,
    cnpj_favorecido,
    nome_favorecido,
    nome_emenda,
    tipo_emenda,
    transferencia_especial_origem,
    categoria_economica,
    valor,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipada
WHERE tdoa_emendas_corresponde_filtro(ano_origem, uf)
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
