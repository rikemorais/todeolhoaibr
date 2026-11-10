-- Contrato senado.despesa.ceaps.v1.
WITH origem AS (
    SELECT
        TRIM("ANO") AS ano_texto,
        NULLIF(TRIM("MES"), '') AS mes_texto,
        NULLIF(TRIM("DATA"), '') AS data_texto,
        NULLIF(TRIM("COD_DOCUMENTO"), '') AS codigo_limpo,
        NULLIF(TRIM("VALOR_REEMBOLSADO"), '') AS valor_texto,
        *,
        CASE
            WHEN regexp_like(TRIM("DATA"), '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
            THEN TRY_CAST(regexp_replace(
                TRIM("DATA"), '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1'
            ) AS DATE)
        END AS data_tipada
    FROM bronze
), classificados AS (
    SELECT
        *,
        COUNT(*) OVER (PARTITION BY codigo_limpo) AS ocorrencias_codigo,
        CASE
            WHEN data_texto IS NOT NULL AND data_tipada IS NULL AND codigo_limpo IS NULL
            THEN TRUE ELSE FALSE
        END AS linha_desalinhada,
        regexp_replace(
            regexp_replace(TRIM("VALOR_REEMBOLSADO"), '[\r\n]', '', 'g'),
            ',', '.', 'g'
        ) AS valor_normalizado
    FROM origem
), validadas AS (
    SELECT *
    FROM classificados
    WHERE tdoa_assert_true(TRY_CAST(ano_texto AS SMALLINT) = {{ANO_RECURSO}})
      AND tdoa_assert_true(NULLIF(TRIM("SENADOR"), '') IS NOT NULL)
      AND tdoa_assert_true(mes_texto IS NULL OR (
          TRY_CAST(mes_texto AS TINYINT) BETWEEN 1 AND 12
      ))
      AND tdoa_assert_true(codigo_limpo IS NULL OR ocorrencias_codigo = 1)
      AND tdoa_assert_true(
          valor_texto IS NULL OR linha_desalinhada OR (
              regexp_like(valor_normalizado, '^-?([0-9]+([.][0-9]{1,2})?|[.][0-9]{1,2})$')
              AND TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) IS NOT NULL
          )
      )
)
SELECT
    arrow_try_cast('{{ANO_RECURSO}}', 'UInt16') AS ano,
    arrow_try_cast(mes_texto, 'UInt8') AS mes,
    "SENADOR" AS senador,
    "TIPO_DESPESA" AS tipo_despesa,
    "CNPJ_CPF" AS cnpj_cpf,
    "FORNECEDOR" AS fornecedor,
    "DOCUMENTO" AS documento,
    CASE
        WHEN EXTRACT(YEAR FROM data_tipada) BETWEEN 1900 AND {{ANO_RECURSO}} + 1
        THEN data_tipada
    END AS data,
    "DETALHAMENTO" AS detalhamento,
    CASE WHEN linha_desalinhada THEN NULL
        ELSE TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) END AS valor_reembolsado,
    "COD_DOCUMENTO" AS cod_documento,
    _tdoa_json_original AS registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM validadas
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
