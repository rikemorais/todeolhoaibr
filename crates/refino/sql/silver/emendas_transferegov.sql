-- Contrato emendas-transferegov.beneficiario-programa.v1.
-- Uma linha por registro beneficiário-programa da origem. Valores conservam os nomes
-- e o significado publicados pelo Transferegov; não são classificados como pagamentos.
WITH entrada AS (
    SELECT
        {{EXPR_OPCIONAL:id_beneficiario_emenda_parlamentar_programa}} AS id_beneficiario_programa_origem,
        {{EXPR_OPCIONAL:id_programa}} AS id_programa_origem,
        {{EXPR_OPCIONAL:nr_cnpj_beneficiario_emenda}} AS cnpj_beneficiario_origem,
        {{EXPR_OPCIONAL:nm_beneficiario_emenda}} AS nome_beneficiario_origem,
        {{EXPR_OPCIONAL:nm_municipio_beneficiario_emenda}} AS municipio_origem,
        {{EXPR_OPCIONAL:sg_uf_beneficiario_emenda}} AS uf_origem,
        {{EXPR_OPCIONAL:nm_natureza_juridica_beneficiario_emenda}} AS natureza_juridica_beneficiario_origem,
        {{EXPR_OPCIONAL:aa_emenda}} AS ano_origem,
        {{EXPR_OPCIONAL:cd_parlamentar}} AS codigo_parlamentar_origem,
        {{EXPR_OPCIONAL:nr_emenda}} AS numero_emenda_origem,
        {{EXPR_OPCIONAL:nm_parlamentar}} AS nome_parlamentar_origem,
        {{EXPR_OPCIONAL:in_tipo_emenda_parlamentar}} AS tipo_emenda_origem,
        {{EXPR_OPCIONAL:vl_gnd3}} AS valor_gnd3_origem,
        {{EXPR_OPCIONAL:vl_gnd4}} AS valor_gnd4_origem,
        {{EXPR_OPCIONAL:vl_total_emenda}} AS valor_total_emenda_origem,
        {{EXPR_OPCIONAL:indicacoes_beneficiario}} AS indicacoes_beneficiario_origem,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
), tipada AS (
    SELECT
        TRY_CAST(NULLIF(TRIM(id_beneficiario_programa_origem), '') AS BIGINT) AS id_beneficiario_programa,
        TRY_CAST(NULLIF(TRIM(id_programa_origem), '') AS BIGINT) AS id_programa,
        NULLIF(TRIM(cnpj_beneficiario_origem), '') AS cnpj_beneficiario,
        NULLIF(TRIM(nome_beneficiario_origem), '') AS nome_beneficiario,
        NULLIF(TRIM(municipio_origem), '') AS municipio,
        UPPER(NULLIF(TRIM(uf_origem), '')) AS uf,
        NULLIF(TRIM(natureza_juridica_beneficiario_origem), '') AS natureza_juridica_beneficiario,
        TRY_CAST(NULLIF(TRIM(ano_origem), '') AS SMALLINT) AS ano,
        NULLIF(TRIM(codigo_parlamentar_origem), '') AS codigo_parlamentar,
        NULLIF(TRIM(numero_emenda_origem), '') AS numero_emenda,
        NULLIF(TRIM(nome_parlamentar_origem), '') AS nome_parlamentar,
        NULLIF(TRIM(tipo_emenda_origem), '') AS tipo_emenda,
        TRY_CAST(NULLIF(TRIM(valor_gnd3_origem), '') AS DECIMAL(20, 2)) AS valor_gnd3,
        TRY_CAST(NULLIF(TRIM(valor_gnd4_origem), '') AS DECIMAL(20, 2)) AS valor_gnd4,
        TRY_CAST(NULLIF(TRIM(valor_total_emenda_origem), '') AS DECIMAL(20, 2)) AS valor_total_emenda,
        NULLIF(TRIM(indicacoes_beneficiario_origem), '') AS indicacoes_beneficiario_origem,
        ano_origem,
        uf_origem,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM entrada
)
SELECT
    id_beneficiario_programa,
    id_programa,
    cnpj_beneficiario,
    nome_beneficiario,
    municipio,
    uf,
    natureza_juridica_beneficiario,
    ano,
    codigo_parlamentar,
    numero_emenda,
    nome_parlamentar,
    tipo_emenda,
    valor_gnd3,
    valor_gnd4,
    valor_total_emenda,
    indicacoes_beneficiario_origem,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipada
WHERE tdoa_assert_true_named(
        id_beneficiario_programa IS NOT NULL AND ano IS NOT NULL
            AND codigo_parlamentar IS NOT NULL
            AND numero_emenda IS NOT NULL AND municipio IS NOT NULL AND uf IS NOT NULL,
        'identificadores_autoria_e_localidade_obrigatorios')
  AND tdoa_emendas_corresponde_filtro(ano_origem, uf_origem)
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
