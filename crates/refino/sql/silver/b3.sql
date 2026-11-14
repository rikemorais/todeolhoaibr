-- Contrato b3.cotahist.v1.
-- Cotações históricas diárias da B3. O Bronze já extrai os 26 campos do
-- fixed-width e converte decimais implícitos.
SELECT
    to_date(data_pregao, '%Y%m%d') AS data_pregao,
    NULLIF(TRIM(codneg), '') AS codigo_negociacao,
    CAST(NULLIF(TRIM(codbdi), '') AS INT) AS codigo_bdi,
    CAST(NULLIF(TRIM(tpmerc), '') AS INT) AS tipo_mercado,
    NULLIF(TRIM(nomres), '') AS nome_resumido,
    NULLIF(TRIM(especi), '') AS especificacao,
    NULLIF(TRIM(modref), '') AS moeda_referencia,
    TRY_CAST(preabe AS DECIMAL(18, 2)) AS preco_abertura,
    TRY_CAST(premax AS DECIMAL(18, 2)) AS preco_maximo,
    TRY_CAST(premin AS DECIMAL(18, 2)) AS preco_minimo,
    TRY_CAST(premed AS DECIMAL(18, 2)) AS preco_medio,
    TRY_CAST(preult AS DECIMAL(18, 2)) AS preco_ultimo,
    TRY_CAST(preofc AS DECIMAL(18, 2)) AS preco_melhor_oferta_compra,
    TRY_CAST(preofv AS DECIMAL(18, 2)) AS preco_melhor_oferta_venda,
    CAST(NULLIF(TRIM(totneg), '') AS BIGINT) AS total_negocios,
    CAST(NULLIF(TRIM(quatot), '') AS BIGINT) AS quantidade_titulos,
    TRY_CAST(voltot AS DECIMAL(18, 2)) AS volume_total,
    TRY_CAST(preexe AS DECIMAL(18, 2)) AS preco_exercicio,
    NULLIF(TRIM(indopc), '') AS indicador_opcao,
    NULLIF(TRIM(datven), '') AS data_vencimento,
    CAST(NULLIF(TRIM(fatcot), '') AS INT) AS fator_cotacao,
    TRY_CAST(ptoexe AS DECIMAL(18, 6)) AS preco_exercicio_pontos,
    NULLIF(TRIM(codisi), '') AS codigo_isin,
    CAST(NULLIF(TRIM(dismes), '') AS INT) AS distribuicao,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM bronze
