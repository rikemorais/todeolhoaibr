-- Contrato emendas-cgu.autoria-localidade.v1.
-- A localidade da CGU é mantida como texto; não há chave IBGE documentada.
WITH entrada AS (
    SELECT
        {{EXPR_OPCIONAL:codigoEmenda}} AS codigo_emenda,
        {{EXPR_OPCIONAL:ano}} AS ano_origem,
        {{EXPR_OPCIONAL:tipoEmenda}} AS tipo_emenda,
        {{EXPR_OPCIONAL:autor}} AS codigo_autor,
        {{EXPR_OPCIONAL:nomeAutor}} AS nome_autor,
        {{EXPR_OPCIONAL:numeroEmenda}} AS numero_emenda,
        {{EXPR_OPCIONAL:localidadeDoGasto}} AS localidade_do_gasto,
        {{EXPR_OPCIONAL:valorEmpenhado}} AS valor_empenhado_origem,
        {{EXPR_OPCIONAL:valorLiquidado}} AS valor_liquidado_origem,
        {{EXPR_OPCIONAL:valorPago}} AS valor_pago_origem,
        {{EXPR_OPCIONAL:valorRestosPagarInscritos}} AS restos_pagar_inscritos_origem,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
), tipada AS (
    SELECT
        NULLIF(TRIM(codigo_emenda), '') AS codigo_emenda,
        TRY_CAST(NULLIF(TRIM(ano_origem), '') AS SMALLINT) AS ano,
        NULLIF(TRIM(tipo_emenda), '') AS tipo_emenda,
        NULLIF(TRIM(codigo_autor), '') AS codigo_autor,
        NULLIF(TRIM(nome_autor), '') AS nome_autor,
        NULLIF(TRIM(numero_emenda), '') AS numero_emenda,
        NULLIF(TRIM(localidade_do_gasto), '') AS localidade_do_gasto,
        TRY_CAST(REPLACE(NULLIF(TRIM(valor_empenhado_origem), ''), ',', '.') AS DECIMAL(20, 2)) AS valor_empenhado,
        TRY_CAST(REPLACE(NULLIF(TRIM(valor_liquidado_origem), ''), ',', '.') AS DECIMAL(20, 2)) AS valor_liquidado,
        TRY_CAST(REPLACE(NULLIF(TRIM(valor_pago_origem), ''), ',', '.') AS DECIMAL(20, 2)) AS valor_pago,
        TRY_CAST(REPLACE(NULLIF(TRIM(restos_pagar_inscritos_origem), ''), ',', '.') AS DECIMAL(20, 2)) AS restos_pagar_inscritos,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal,
        ano_origem
    FROM entrada
)
SELECT
    codigo_emenda,
    ano,
    tipo_emenda,
    codigo_autor,
    nome_autor,
    numero_emenda,
    localidade_do_gasto,
    valor_empenhado,
    valor_liquidado,
    valor_pago,
    restos_pagar_inscritos,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipada
WHERE tdoa_assert_true_named(
        codigo_emenda IS NOT NULL AND ano IS NOT NULL AND codigo_autor IS NOT NULL,
        'identificadores_de_emenda_ano_e_autor_obrigatorios')
  AND tdoa_emendas_corresponde_filtro(ano_origem, CAST(NULL AS VARCHAR))
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
