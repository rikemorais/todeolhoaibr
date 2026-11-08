-- Contrato tse.candidatura.registro.v1: uma linha por candidatura do consolidado BRASIL.
WITH origem AS (
    SELECT
        *,
        TRIM("ANO_ELEICAO") AS ano_texto,
        TRIM("CD_ELEICAO") AS codigo_eleicao,
        TRIM("SG_UF") AS uf,
        TRIM("SG_UE") AS unidade_eleitoral,
        TRIM("CD_CARGO") AS codigo_cargo,
        TRIM("NR_CANDIDATO") AS numero_candidato,
        TRIM("SQ_CANDIDATO") AS sequencia_candidato,
        {{EXPR_OPCIONAL:DT_ELEICAO}} AS data_eleicao_texto,
        {{EXPR_OPCIONAL:DT_NASCIMENTO}} AS data_nascimento_texto,
        {{EXPR_OPCIONAL:NR_IDADE_DATA_POSSE}} AS idade_texto,
        TRIM("SQ_CANDIDATO") AS sequencia_candidato_chave
    FROM bronze
    WHERE regexp_like(upper(_tdoa_origem_url), '_BRASIL([.]CSV|[.]TXT)?$')
), contagens_chave AS (
    SELECT COUNT(*) AS chaves_duplicadas
    FROM (
        SELECT codigo_eleicao, uf, unidade_eleitoral, codigo_cargo, numero_candidato,
               sequencia_candidato_chave
        FROM origem
        GROUP BY codigo_eleicao, uf, unidade_eleitoral, codigo_cargo, numero_candidato, sequencia_candidato_chave
        HAVING COUNT(*) > 1
    ) AS duplicadas
), normalizadas AS (
    SELECT origem.*,
        contagens_chave.chaves_duplicadas,
        CASE
            WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
                THEN regexp_replace(TRIM(data_eleicao_texto), '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1')
            WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{4}-[0-9]{2}-[0-9]{2}$')
                THEN TRIM(data_eleicao_texto)
            WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{2}-[0-9]{2}-[0-9]{4}$')
                THEN regexp_replace(TRIM(data_eleicao_texto), '^([0-9]{2})-([0-9]{2})-([0-9]{4})$', '$3-$2-$1')
            WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{8}$')
                THEN regexp_replace(TRIM(data_eleicao_texto), '^([0-9]{4})([0-9]{2})([0-9]{2})$', '$1-$2-$3')
        END AS data_eleicao_iso,
        CASE
            WHEN regexp_like(TRIM(data_nascimento_texto), '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
                THEN regexp_replace(TRIM(data_nascimento_texto), '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1')
            WHEN regexp_like(TRIM(data_nascimento_texto), '^[0-9]{4}-[0-9]{2}-[0-9]{2}$')
                THEN TRIM(data_nascimento_texto)
            WHEN regexp_like(TRIM(data_nascimento_texto), '^[0-9]{2}-[0-9]{2}-[0-9]{4}$')
                THEN regexp_replace(TRIM(data_nascimento_texto), '^([0-9]{2})-([0-9]{2})-([0-9]{4})$', '$3-$2-$1')
            WHEN regexp_like(TRIM(data_nascimento_texto), '^[0-9]{8}$')
                THEN regexp_replace(TRIM(data_nascimento_texto), '^([0-9]{4})([0-9]{2})([0-9]{2})$', '$1-$2-$3')
        END AS data_nascimento_iso
    FROM origem
    CROSS JOIN contagens_chave
), validas AS (
    SELECT *
    FROM normalizadas
    WHERE tdoa_assert_true_named(
          TRY_CAST(ano_texto AS SMALLINT) = {{ANO_RECURSO}}, 'ano_do_recurso')
      AND tdoa_assert_true_named(
          codigo_eleicao <> '' AND uf <> '' AND unidade_eleitoral <> ''
          AND codigo_cargo <> '' AND numero_candidato <> '' AND sequencia_candidato <> '',
          'campos_da_chave')
      AND tdoa_assert_true_named(chaves_duplicadas = 0, 'unicidade_da_chave')
)
SELECT
    arrow_try_cast('{{ANO_RECURSO}}', 'UInt16') AS ano_eleicao,
    "CD_ELEICAO" AS codigo_eleicao,
    "SG_UF" AS uf,
    "SG_UE" AS unidade_eleitoral,
    "CD_CARGO" AS codigo_cargo,
    "NR_CANDIDATO" AS numero_candidato,
    "SQ_CANDIDATO" AS sequencia_candidato,
    CASE WHEN EXTRACT(YEAR FROM arrow_try_cast(data_eleicao_iso, 'Date32')) BETWEEN 1800 AND {{ANO_RECURSO}} + 1
        THEN arrow_try_cast(data_eleicao_iso, 'Date32') END AS data_eleicao,
    CASE WHEN EXTRACT(YEAR FROM arrow_try_cast(data_nascimento_iso, 'Date32')) BETWEEN 1800 AND {{ANO_RECURSO}} + 1
        THEN arrow_try_cast(data_nascimento_iso, 'Date32') END AS data_nascimento,
    CASE WHEN NULLIF(TRIM(idade_texto), '') NOT IN ('-1', '#NULO', '#NE')
        AND regexp_like(NULLIF(TRIM(idade_texto), ''), '^[0-9]{1,3}$')
        THEN arrow_try_cast(NULLIF(TRIM(idade_texto), ''), 'UInt8') END AS idade_data_posse,
    _tdoa_json_original AS registro_original
FROM validas
