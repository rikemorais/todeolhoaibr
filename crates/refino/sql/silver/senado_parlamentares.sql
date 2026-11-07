-- Contrato senado.parlamentar.legislatura.v1.
WITH campos AS (
    SELECT
        tdoa_json_get("IdentificacaoParlamentar", 'CodigoParlamentar') AS codigo_parlamentar,
        arrow_try_cast('{{LEGISLATURA}}', 'UInt16') AS legislatura,
        "IdentificacaoParlamentar" AS identificacao_original,
        {{OPCIONAL:Mandato}},
        _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
    FROM bronze
)
SELECT
    CASE WHEN tdoa_assert_true(codigo_parlamentar IS NOT NULL AND codigo_parlamentar <> '')
        THEN codigo_parlamentar END AS codigo_parlamentar,
    legislatura,
    identificacao_original,
    "Mandato" AS mandato_original
FROM campos
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
