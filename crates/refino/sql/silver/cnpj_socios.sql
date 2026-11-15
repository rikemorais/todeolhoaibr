-- Contrato cnpj.socios.v1.
-- Tipa colunas do quadro societário da Receita Federal.
SELECT
    NULLIF(TRIM(cnpj_basico), '')           AS cnpj_basico,
    TRY_CAST(NULLIF(TRIM(identificador_socio), '') AS TINYINT UNSIGNED) AS identificador_socio,
    NULLIF(TRIM(nome_socio), '')            AS nome_socio,
    NULLIF(TRIM(cnpj_cpf_socio), '')        AS cnpj_cpf_socio,
    NULLIF(TRIM(qualificacao), '')          AS qualificacao,
    CASE WHEN LENGTH(TRIM(data_entrada)) = 8
         THEN to_date(TRIM(data_entrada), '%Y%m%d')
         ELSE NULL END                      AS data_entrada,
    NULLIF(TRIM(pais), '')                  AS pais,
    NULLIF(TRIM(representante_legal), '')   AS representante_legal,
    NULLIF(TRIM(nome_representante), '')    AS nome_representante,
    NULLIF(TRIM(qualificacao_representante), '') AS qualificacao_representante,
    TRY_CAST(NULLIF(TRIM(faixa_etaria), '') AS TINYINT UNSIGNED) AS faixa_etaria,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM bronze
WHERE tdoa_assert_true_named(
        NULLIF(TRIM(cnpj_basico), '') IS NOT NULL,
        'cnpj_basico_obrigatorio')
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
