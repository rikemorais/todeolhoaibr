-- Contrato cnpj.simples.v1.
-- Tipa colunas de opção pelo Simples/MEI da Receita Federal.
SELECT
    NULLIF(TRIM(cnpj_basico), '')           AS cnpj_basico,
    NULLIF(TRIM(opcao_simples), '')         AS opcao_simples,
    CASE WHEN LENGTH(TRIM(data_opcao_simples)) = 8
         THEN to_date(TRIM(data_opcao_simples), '%Y%m%d')
         ELSE NULL END                      AS data_opcao_simples,
    CASE WHEN LENGTH(TRIM(data_exclusao_simples)) = 8
         THEN to_date(TRIM(data_exclusao_simples), '%Y%m%d')
         ELSE NULL END                      AS data_exclusao_simples,
    NULLIF(TRIM(opcao_mei), '')             AS opcao_mei,
    CASE WHEN LENGTH(TRIM(data_opcao_mei)) = 8
         THEN to_date(TRIM(data_opcao_mei), '%Y%m%d')
         ELSE NULL END                      AS data_opcao_mei,
    CASE WHEN LENGTH(TRIM(data_exclusao_mei)) = 8
         THEN to_date(TRIM(data_exclusao_mei), '%Y%m%d')
         ELSE NULL END                      AS data_exclusao_mei,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM bronze
WHERE tdoa_assert_true_named(
        NULLIF(TRIM(cnpj_basico), '') IS NOT NULL,
        'cnpj_basico_obrigatorio')
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
