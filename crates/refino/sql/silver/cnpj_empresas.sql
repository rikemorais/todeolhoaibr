-- Contrato cnpj.empresas.v1.
-- Tipa colunas do cadastro de empresas da Receita Federal (Bronze all-string).
SELECT
    NULLIF(TRIM(cnpj_basico), '')           AS cnpj_basico,
    NULLIF(TRIM(razao_social), '')          AS razao_social,
    NULLIF(TRIM(natureza_juridica), '')     AS natureza_juridica,
    NULLIF(TRIM(qualificacao_responsavel), '') AS qualificacao_responsavel,
    TRY_CAST(
        REPLACE(NULLIF(TRIM(capital_social), ''), ',', '.')
        AS DECIMAL(18, 2)
    )                                       AS capital_social,
    NULLIF(TRIM(porte_empresa), '')         AS porte_empresa,
    NULLIF(TRIM(ente_federativo), '')       AS ente_federativo,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM bronze
WHERE tdoa_assert_true_named(
        NULLIF(TRIM(cnpj_basico), '') IS NOT NULL,
        'cnpj_basico_obrigatorio')
  AND tdoa_assert_true_named(
        NULLIF(TRIM(razao_social), '') IS NOT NULL,
        'razao_social_obrigatoria')
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
