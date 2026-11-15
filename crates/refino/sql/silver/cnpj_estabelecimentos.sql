-- Contrato cnpj.estabelecimentos.v1.
-- Tipa colunas do cadastro de estabelecimentos da Receita Federal.
WITH entrada AS (
    SELECT
        NULLIF(TRIM(cnpj_basico), '')       AS cnpj_basico,
        NULLIF(TRIM(cnpj_ordem), '')        AS cnpj_ordem,
        NULLIF(TRIM(cnpj_dv), '')           AS cnpj_dv,
        TRY_CAST(NULLIF(TRIM(matriz_filial), '') AS TINYINT UNSIGNED) AS matriz_filial,
        NULLIF(TRIM(nome_fantasia), '')     AS nome_fantasia,
        NULLIF(TRIM(situacao_cadastral), '') AS situacao_cadastral,
        NULLIF(TRIM(data_situacao), '')     AS data_situacao_raw,
        NULLIF(TRIM(motivo_situacao), '')   AS motivo_situacao,
        NULLIF(TRIM(cidade_exterior), '')   AS cidade_exterior,
        NULLIF(TRIM(pais), '')              AS pais,
        NULLIF(TRIM(data_inicio), '')       AS data_inicio_raw,
        NULLIF(TRIM(cnae_principal), '')    AS cnae_principal,
        NULLIF(TRIM(cnae_secundaria), '')   AS cnae_secundaria,
        NULLIF(TRIM(tipo_logradouro), '')   AS tipo_logradouro,
        NULLIF(TRIM(logradouro), '')        AS logradouro,
        NULLIF(TRIM(numero), '')            AS numero,
        NULLIF(TRIM(complemento), '')       AS complemento,
        NULLIF(TRIM(bairro), '')            AS bairro,
        NULLIF(TRIM(cep), '')              AS cep,
        NULLIF(TRIM(uf), '')               AS uf,
        NULLIF(TRIM(municipio), '')        AS municipio,
        NULLIF(TRIM(ddd1), '')             AS ddd1,
        NULLIF(TRIM(telefone1), '')        AS telefone1,
        NULLIF(TRIM(ddd2), '')             AS ddd2,
        NULLIF(TRIM(telefone2), '')        AS telefone2,
        NULLIF(TRIM(ddd_fax), '')          AS ddd_fax,
        NULLIF(TRIM(fax), '')              AS fax,
        NULLIF(TRIM(email), '')            AS email,
        NULLIF(TRIM(situacao_especial), '') AS situacao_especial,
        NULLIF(TRIM(data_situacao_especial), '') AS data_situacao_especial_raw,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
)
SELECT
    cnpj_basico,
    cnpj_ordem,
    cnpj_dv,
    matriz_filial,
    nome_fantasia,
    situacao_cadastral,
    CASE WHEN LENGTH(data_situacao_raw) = 8
         THEN to_date(data_situacao_raw, '%Y%m%d')
         ELSE NULL END                     AS data_situacao,
    motivo_situacao,
    cidade_exterior,
    pais,
    CASE WHEN LENGTH(data_inicio_raw) = 8
         THEN to_date(data_inicio_raw, '%Y%m%d')
         ELSE NULL END                     AS data_inicio,
    cnae_principal,
    cnae_secundaria,
    tipo_logradouro,
    logradouro,
    numero,
    complemento,
    bairro,
    cep,
    uf,
    municipio,
    ddd1,
    telefone1,
    ddd2,
    telefone2,
    ddd_fax,
    fax,
    email,
    situacao_especial,
    CASE WHEN LENGTH(data_situacao_especial_raw) = 8
         THEN to_date(data_situacao_especial_raw, '%Y%m%d')
         ELSE NULL END                     AS data_situacao_especial,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM entrada
WHERE tdoa_assert_true_named(cnpj_basico IS NOT NULL, 'cnpj_basico_obrigatorio')
  AND tdoa_assert_true_named(cnpj_ordem IS NOT NULL, 'cnpj_ordem_obrigatoria')
  AND tdoa_assert_true_named(cnpj_dv IS NOT NULL, 'cnpj_dv_obrigatorio')
  AND tdoa_assert_true_named(situacao_cadastral IS NOT NULL, 'situacao_cadastral_obrigatoria')
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
