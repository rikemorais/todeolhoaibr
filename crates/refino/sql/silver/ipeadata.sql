-- Contrato ipeadata.observacao.v1: uma linha por observação de série socioeconômica.
-- Preserva nível geográfico (NIVNOME) para distinguir Brasil, estados, regiões e municípios.
WITH origem AS (
    SELECT
        TRIM("SERCODIGO") AS serie,
        TRIM("TERCODIGO") AS territorio,
        TRIM("VALDATA") AS data_texto,
        TRIM("NIVNOME") AS nivel,
        TRIM("VALVALOR") AS valor_texto,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal,
        COUNT(*) OVER (
            PARTITION BY TRIM("SERCODIGO"), TRIM("TERCODIGO"), TRIM("VALDATA")
        ) AS ocorrencias
    FROM bronze
)
SELECT
    serie,
    territorio,
    CASE
        WHEN regexp_like(data_texto, '^[0-9]{4}-[0-9]{2}-[0-9]{2}T')
            THEN to_date(
                regexp_replace(data_texto, '^([0-9]{4}-[0-9]{2}-[0-9]{2})T.*$', '$1'),
                '%Y-%m-%d')
        WHEN regexp_like(data_texto, '^[0-9]{4}-[0-9]{2}-[0-9]{2}$')
            THEN to_date(data_texto, '%Y-%m-%d')
        WHEN regexp_like(data_texto, '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
            THEN to_date(data_texto, '%d/%m/%Y')
    END AS data,
    nivel,
    CASE
        WHEN valor_texto IS NOT NULL
            AND TRIM(valor_texto) <> ''
            AND regexp_like(TRIM(valor_texto), '^-?[0-9]{1,18}([.][0-9]{1,9})?$')
            THEN CAST(TRIM(valor_texto) AS DECIMAL(38, 9))
    END AS valor,
    valor_texto AS valor_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM origem
WHERE tdoa_assert_true_named(
    serie IS NOT NULL AND serie <> '', 'serie_obrigatoria')
  AND tdoa_assert_true_named(
    territorio IS NOT NULL AND territorio <> '', 'territorio_obrigatorio')
  AND tdoa_assert_true_named(ocorrencias = 1, 'chave_unica')
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
