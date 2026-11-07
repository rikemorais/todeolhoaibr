-- Contrato tse.candidatura.bem.v1: uma linha por bem do consolidado BRASIL.
WITH origem AS (
    SELECT
        *,
        TRIM("ANO_ELEICAO") AS ano_texto,
        TRIM("CD_ELEICAO") AS codigo_eleicao,
        TRIM("SG_UF") AS uf,
        TRIM("SG_UE") AS unidade_eleitoral,
        TRIM("SQ_CANDIDATO") AS sequencia_candidato,
        COALESCE({{EXPR_OPCIONAL:NR_ORDEM_CANDIDATO}}, {{EXPR_OPCIONAL:NR_ORDEM_BEM_CANDIDATO}}, '') AS ordem_texto,
        COALESCE({{EXPR_OPCIONAL:DT_ULTIMA_ATUALIZACAO}}, {{EXPR_OPCIONAL:DT_ULT_ATUAL_BEM_CANDIDATO}}) AS data_atualizacao_texto,
        {{EXPR_OPCIONAL:DT_ELEICAO}} AS data_eleicao_texto,
        COUNT(*) OVER (
            PARTITION BY TRIM("CD_ELEICAO"), TRIM("SG_UF"), TRIM("SG_UE"),
                         TRIM("SQ_CANDIDATO"),
                         arrow_try_cast(COALESCE({{EXPR_OPCIONAL:NR_ORDEM_CANDIDATO}},
                             {{EXPR_OPCIONAL:NR_ORDEM_BEM_CANDIDATO}}, ''), 'UInt32')
        ) AS ocorrencias_chave,
        NULLIF(TRIM({{EXPR_OPCIONAL:VR_BEM_CANDIDATO}}), '') AS valor_texto
    FROM bronze
    WHERE regexp_like(upper(_tdoa_origem_url), '_BRASIL([.]CSV|[.]TXT)?$')
), preparados AS (
    SELECT *,
        CASE WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
            THEN regexp_replace(TRIM(data_eleicao_texto), '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1')
            WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{4}-[0-9]{2}-[0-9]{2}$') THEN TRIM(data_eleicao_texto)
            WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{2}-[0-9]{2}-[0-9]{4}$')
            THEN regexp_replace(TRIM(data_eleicao_texto), '^([0-9]{2})-([0-9]{2})-([0-9]{4})$', '$3-$2-$1')
            WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{8}$')
            THEN regexp_replace(TRIM(data_eleicao_texto), '^([0-9]{4})([0-9]{2})([0-9]{2})$', '$1-$2-$3') END AS data_eleicao_iso,
        CASE WHEN regexp_like(data_atualizacao_texto, '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
            THEN regexp_replace(data_atualizacao_texto, '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1')
            WHEN regexp_like(data_atualizacao_texto, '^[0-9]{4}-[0-9]{2}-[0-9]{2}$') THEN data_atualizacao_texto
            WHEN regexp_like(data_atualizacao_texto, '^[0-9]{2}-[0-9]{2}-[0-9]{4}$')
            THEN regexp_replace(data_atualizacao_texto, '^([0-9]{2})-([0-9]{2})-([0-9]{4})$', '$3-$2-$1')
            WHEN regexp_like(data_atualizacao_texto, '^[0-9]{8}$')
            THEN regexp_replace(data_atualizacao_texto, '^([0-9]{4})([0-9]{2})([0-9]{2})$', '$1-$2-$3') END AS data_atualizacao_iso,
        CASE WHEN valor_texto IS NOT NULL THEN regexp_replace(valor_texto, ',', '.', 'g') END AS valor_normalizado
    FROM origem
), validados AS (
    SELECT * FROM preparados
    WHERE tdoa_assert_true(TRY_CAST(ano_texto AS SMALLINT) = {{ANO_RECURSO}})
      AND tdoa_assert_true(codigo_eleicao <> '' AND uf <> '' AND unidade_eleitoral <> '' AND sequencia_candidato <> '')
      AND tdoa_assert_true(regexp_like(ordem_texto, '^[0-9]+$')
          AND arrow_try_cast(ordem_texto, 'UInt32') IS NOT NULL)
      AND tdoa_assert_true(ocorrencias_chave = 1)
      AND tdoa_assert_true(valor_texto IS NULL OR (
          regexp_like(valor_texto, '^-?([0-9]+(,[0-9]{1,2})?|,[0-9]{1,2})$')
          AND TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) IS NOT NULL
      ))
)
SELECT
    arrow_try_cast('{{ANO_RECURSO}}', 'UInt16') AS ano_eleicao,
    "CD_ELEICAO" AS codigo_eleicao,
    "SG_UF" AS uf,
    "SG_UE" AS unidade_eleitoral,
    "SQ_CANDIDATO" AS sequencia_candidato,
    arrow_try_cast(ordem_texto, 'UInt32') AS ordem_bem,
    CASE WHEN EXTRACT(YEAR FROM arrow_try_cast(data_eleicao_iso, 'Date32')) BETWEEN 1800 AND {{ANO_RECURSO}} + 1
        THEN arrow_try_cast(data_eleicao_iso, 'Date32') END AS data_eleicao,
    CASE WHEN EXTRACT(YEAR FROM arrow_try_cast(data_atualizacao_iso, 'Date32')) BETWEEN 1800 AND {{ANO_RECURSO}} + 1
        THEN arrow_try_cast(data_atualizacao_iso, 'Date32') END AS data_ultima_atualizacao,
    {{EXPR_OPCIONAL:CD_TIPO_BEM_CANDIDATO}} AS codigo_tipo_bem,
    {{EXPR_OPCIONAL:DS_TIPO_BEM_CANDIDATO}} AS tipo_bem,
    {{EXPR_OPCIONAL:DS_BEM_CANDIDATO}} AS descricao_bem,
    arrow_try_cast(valor_normalizado, 'Decimal128(18,2)') AS valor_bem,
    _tdoa_json_original AS registro_original
FROM validados
