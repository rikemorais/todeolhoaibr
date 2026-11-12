-- Contrato ibge.renda_per_capita.observacao.v1.
-- Mapeia as colunas codificadas do SIDRA /values para nomes legíveis.
-- A primeira linha do JSON é um cabeçalho descritivo (NC = 'Nível Territorial (Código)');
-- sentinelas de valor ('..', '-', 'X') ficam como valor_original mas não viram decimal.
SELECT
    "D1C"                          AS municipio_id,
    "D1N"                          AS municipio_nome,
    "D2C"                          AS variavel_id,
    "D2N"                          AS variavel_nome,
    "MN"                           AS unidade,
    "D3C"                          AS ano_censo,
    "V"                            AS valor_original,
    TRY_CAST(
        CASE
            WHEN regexp_like("V", '^-?[0-9]{1,29}([.][0-9]{1,9})?$') THEN "V"
            ELSE NULL
        END
        AS DECIMAL(38, 9)
    ) AS valor_decimal,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM bronze
WHERE "NC" <> 'Nível Territorial (Código)'
  AND "V" NOT IN ('..', '-', 'X')
