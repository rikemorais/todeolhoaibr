-- Contrato camara.despesa.ceap.v2. ideDocumento não é chave única.
WITH validas AS (
    SELECT *
    FROM bronze
    WHERE tdoa_assert_true(
        "datEmissao" IS NULL OR TRIM("datEmissao") = ''
        OR TRY_CAST(TRIM("datEmissao") AS TIMESTAMP) IS NOT NULL
    )
      AND tdoa_assert_true(
        "datPagamentoRestituicao" IS NULL OR TRIM("datPagamentoRestituicao") = ''
        OR TRY_CAST(TRIM("datPagamentoRestituicao") AS TIMESTAMP) IS NOT NULL
    )
      AND tdoa_assert_true(
        "vlrDocumento" IS NULL OR TRIM("vlrDocumento") = ''
        OR regexp_like(TRIM("vlrDocumento"), '^-?[0-9]{1,12}([.][0-9]{1,6})?$')
    )
      AND tdoa_assert_true(
        "vlrGlosa" IS NULL OR TRIM("vlrGlosa") = ''
        OR regexp_like(TRIM("vlrGlosa"), '^-?[0-9]{1,12}([.][0-9]{1,6})?$')
    )
      AND tdoa_assert_true(
        "vlrLiquido" IS NULL OR TRIM("vlrLiquido") = ''
        OR regexp_like(TRIM("vlrLiquido"), '^-?[0-9]{1,12}([.][0-9]{1,6})?$')
    )
      AND tdoa_assert_true(
        "vlrRestituicao" IS NULL OR TRIM("vlrRestituicao") = ''
        OR regexp_like(TRIM("vlrRestituicao"), '^-?[0-9]{1,12}([.][0-9]{1,6})?$')
    )
)
SELECT
    arrow_try_cast('{{ANO_RECURSO}}', 'UInt16') AS ano,
    "txNomeParlamentar" AS nome_parlamentar,
    "nuDeputadoId" AS id_deputado,
    "ideDocumento" AS id_documento,
    "txtCNPJCPF" AS documento_fornecedor,
    CAST(TRY_CAST(NULLIF(TRIM("datEmissao"), '') AS TIMESTAMP) AS DATE) AS data_emissao,
    CAST(TRY_CAST(NULLIF(TRIM("datPagamentoRestituicao"), '') AS TIMESTAMP) AS DATE)
        AS data_pagamento_restituicao,
    TRY_CAST(NULLIF(TRIM("vlrDocumento"), '') AS DECIMAL(18, 6)) AS valor_documento,
    TRY_CAST(NULLIF(TRIM("vlrGlosa"), '') AS DECIMAL(18, 6)) AS valor_glosa,
    TRY_CAST(NULLIF(TRIM("vlrLiquido"), '') AS DECIMAL(18, 6)) AS valor_liquido,
    TRY_CAST(NULLIF(TRIM("vlrRestituicao"), '') AS DECIMAL(18, 6)) AS valor_restituicao,
    _tdoa_json_original AS registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM validas
