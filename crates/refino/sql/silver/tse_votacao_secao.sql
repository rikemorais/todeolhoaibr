-- Contrato tse.votacao.secao.v1: preservar o grão e validar cada recurso ano × UF.
WITH contagens_chave AS (
    SELECT COUNT(*) AS chaves_duplicadas
    FROM (
        SELECT chave
        FROM (
            SELECT CONCAT(
                CAST(LENGTH(codigo_eleicao) AS VARCHAR), ':', codigo_eleicao,
                CAST(LENGTH(uf) AS VARCHAR), ':', uf,
                CAST(LENGTH(turno) AS VARCHAR), ':', turno,
                CAST(LENGTH(codigo_municipio) AS VARCHAR), ':', codigo_municipio,
                CAST(LENGTH(zona) AS VARCHAR), ':', zona,
                CAST(LENGTH(secao) AS VARCHAR), ':', secao,
                CAST(LENGTH(codigo_cargo) AS VARCHAR), ':', codigo_cargo,
                CAST(LENGTH(numero_votavel) AS VARCHAR), ':', numero_votavel
            ) AS chave
            FROM (
                SELECT
                    COALESCE(TRIM("CD_ELEICAO"), '') AS codigo_eleicao,
                    COALESCE(TRIM("SG_UF"), '') AS uf,
                    COALESCE(CAST(TRY_CAST(TRIM("NR_TURNO") AS TINYINT) AS VARCHAR), '') AS turno,
                    COALESCE(TRIM("CD_MUNICIPIO"), '') AS codigo_municipio,
                    COALESCE(TRIM("NR_ZONA"), '') AS zona,
                    COALESCE(TRIM("NR_SECAO"), '') AS secao,
                    COALESCE(TRIM("CD_CARGO"), '') AS codigo_cargo,
                    COALESCE(TRIM("NR_VOTAVEL"), '') AS numero_votavel
                FROM bronze
            ) AS componentes
        ) AS chaves
        GROUP BY chave
        HAVING COUNT(*) > 1
    ) AS duplicadas
), origem AS (
    SELECT
        *,
        TRIM("ANO_ELEICAO") AS ano_texto,
        TRIM("CD_ELEICAO") AS codigo_eleicao,
        TRIM("NR_TURNO") AS turno_texto,
        TRIM("SG_UF") AS uf,
        TRIM("CD_MUNICIPIO") AS codigo_municipio,
        TRIM("NR_ZONA") AS zona,
        TRIM("NR_SECAO") AS secao,
        TRIM("CD_CARGO") AS codigo_cargo,
        TRIM("NR_VOTAVEL") AS numero_votavel,
        TRIM("QT_VOTOS") AS quantidade_texto,
        {{EXPR_OPCIONAL:DT_ELEICAO}} AS data_eleicao_texto
    FROM bronze
    CROSS JOIN contagens_chave
), preparados AS (
    SELECT *,
        CASE
            WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
            THEN regexp_replace(TRIM(data_eleicao_texto), '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1')
            WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{4}-[0-9]{2}-[0-9]{2}$')
            THEN TRIM(data_eleicao_texto)
            WHEN regexp_like(TRIM(data_eleicao_texto), '^[0-9]{8}$')
            THEN regexp_replace(TRIM(data_eleicao_texto), '^([0-9]{4})([0-9]{2})([0-9]{2})$', '$1-$2-$3')
        END AS data_eleicao_iso
    FROM origem
), validados AS (
    SELECT *
    FROM preparados
    WHERE tdoa_assert_true(TRY_CAST(ano_texto AS SMALLINT) = {{ANO_RECURSO}})
      AND tdoa_assert_true('{{UF_RECURSO}}' = 'BR' OR uf = '{{UF_RECURSO}}')
      AND tdoa_assert_true(codigo_eleicao <> '' AND uf <> '' AND codigo_municipio <> '' AND zona <> ''
          AND secao <> '' AND codigo_cargo <> '' AND numero_votavel <> '')
      AND tdoa_assert_true(regexp_like(turno_texto, '^[0-9]+$') AND TRY_CAST(turno_texto AS TINYINT) > 0)
      AND tdoa_assert_true(regexp_like(quantidade_texto, '^[0-9]+$')
          AND arrow_try_cast(quantidade_texto, 'UInt32') IS NOT NULL)
      AND tdoa_assert_true(chaves_duplicadas = 0)
      AND tdoa_assert_true(data_eleicao_texto IS NULL OR TRIM(data_eleicao_texto) = ''
          OR arrow_try_cast(data_eleicao_iso, 'Date32') IS NOT NULL)
)
SELECT
    arrow_try_cast('{{ANO_RECURSO}}', 'UInt16') AS ano_eleicao,
    "CD_ELEICAO" AS codigo_eleicao,
    arrow_try_cast(turno_texto, 'UInt8') AS turno,
    "SG_UF" AS uf,
    "SG_UE" AS unidade_eleitoral,
    "CD_MUNICIPIO" AS codigo_municipio,
    "NR_ZONA" AS zona,
    "NR_SECAO" AS secao,
    "CD_CARGO" AS codigo_cargo,
    "NR_VOTAVEL" AS numero_votavel,
    arrow_try_cast(quantidade_texto, 'UInt32') AS quantidade_votos,
    arrow_try_cast(data_eleicao_iso, 'Date32') AS data_eleicao,
    {{EXPR_OPCIONAL:NM_TIPO_ELEICAO}} AS tipo_eleicao,
    {{EXPR_OPCIONAL:DS_ELEICAO}} AS nome_eleicao,
    {{EXPR_OPCIONAL:TP_ABRANGENCIA}} AS abrangencia,
    {{EXPR_OPCIONAL:NM_UE}} AS nome_unidade_eleitoral,
    {{EXPR_OPCIONAL:NM_MUNICIPIO}} AS nome_municipio,
    {{EXPR_OPCIONAL:DS_CARGO}} AS descricao_cargo,
    {{EXPR_OPCIONAL:NM_VOTAVEL}} AS nome_votavel,
    {{EXPR_OPCIONAL:NR_LOCAL_VOTACAO}} AS numero_local_votacao,
    {{EXPR_OPCIONAL:SQ_CANDIDATO}} AS sequencia_candidato,
    {{EXPR_OPCIONAL:NM_LOCAL_VOTACAO}} AS nome_local_votacao,
    {{EXPR_OPCIONAL:DS_LOCAL_VOTACAO_ENDERECO}} AS endereco_local_votacao,
    _tdoa_json_original AS registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM validados
