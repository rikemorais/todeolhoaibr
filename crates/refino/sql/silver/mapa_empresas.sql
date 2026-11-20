-- Contrato mapa_empresas.movimentacao.uf.v1.
-- Uma observação mensal por UF, com contagens tipadas e saldo derivado.
WITH entrada AS (
    SELECT
        NULLIF(TRIM(split_part(split_part(_tdoa_origem_manifesto, '/periodo=', 2), '/', 1)), '') AS periodo,
        TRY_CAST(NULLIF(TRIM(dt_referencia), '') AS BIGINT) AS referencia_excel,
        TRY_CAST(NULLIF(TRIM(dt_geracao), '') AS BIGINT) AS geracao_excel,
        NULLIF(TRIM(cod_ibge), '') AS codigo_ibge,
        NULLIF(TRIM("UF"), '') AS uf,
        NULLIF(TRIM("Estado"), '') AS estado,
        TRY_CAST(NULLIF(TRIM("Empresas Abertas"), '') AS BIGINT) AS empresas_abertas,
        TRY_CAST(NULLIF(TRIM("Empresas Fechadas"), '') AS BIGINT) AS empresas_fechadas,
        TRY_CAST(NULLIF(TRIM("Empresas Ativas"), '') AS BIGINT) AS empresas_ativas,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
), tipada AS (
    SELECT
        periodo,
        CASE
            WHEN referencia_excel BETWEEN 1 AND 100000
            THEN CAST(to_timestamp_seconds((referencia_excel - 25569) * 86400) AS DATE)
            ELSE CAST(NULL AS DATE)
        END AS data_referencia,
        CASE
            WHEN geracao_excel BETWEEN 1 AND 100000
            THEN CAST(to_timestamp_seconds((geracao_excel - 25569) * 86400) AS DATE)
            ELSE CAST(NULL AS DATE)
        END AS data_geracao,
        codigo_ibge,
        uf,
        estado,
        empresas_abertas,
        empresas_fechadas,
        empresas_ativas,
        empresas_abertas - empresas_fechadas AS saldo,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal,
        COUNT(*) OVER (PARTITION BY periodo, uf) AS ocorrencias_chave
    FROM entrada
)
SELECT
    periodo,
    data_referencia,
    data_geracao,
    codigo_ibge,
    uf,
    estado,
    empresas_abertas,
    empresas_fechadas,
    empresas_ativas,
    saldo,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipada
WHERE tdoa_assert_true_named(
        periodo IS NOT NULL AND TRY_CAST(CONCAT(periodo, '-01') AS DATE) IS NOT NULL,
        'periodo_obrigatorio_valido')
  AND tdoa_assert_true_named(data_referencia IS NOT NULL, 'data_referencia_excel_valida')
  AND tdoa_assert_true_named(data_geracao IS NOT NULL, 'data_geracao_excel_valida')
  AND tdoa_assert_true_named(codigo_ibge IS NOT NULL, 'codigo_ibge_obrigatorio')
  AND tdoa_assert_true_named(uf IS NOT NULL, 'uf_obrigatoria')
  AND tdoa_assert_true_named(estado IS NOT NULL, 'estado_obrigatorio')
  AND tdoa_assert_true_named(empresas_abertas IS NOT NULL, 'empresas_abertas_obrigatorio')
  AND tdoa_assert_true_named(empresas_fechadas IS NOT NULL, 'empresas_fechadas_obrigatorio')
  AND tdoa_assert_true_named(empresas_ativas IS NOT NULL, 'empresas_ativas_obrigatorio')
  AND tdoa_assert_true_named(
        empresas_abertas >= 0 AND empresas_fechadas >= 0 AND empresas_ativas >= 0,
        'contagens_nao_negativas')
  AND tdoa_assert_true_named(ocorrencias_chave = 1, 'chave_periodo_uf_unica')
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
