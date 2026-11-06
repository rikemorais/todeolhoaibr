-- Contrato ibge.agregado.observacao.v1.
-- Mantém dimensões e valor recebido; valores fora do decimal simples ficam nulos.
SELECT
    variavel_id,
    variavel,
    unidade,
    classificacoes,
    nivel_id,
    nivel_nome,
    localidade_id,
    localidade_nome,
    periodo,
    valor AS valor_original,
    TRY_CAST(
        CASE
            WHEN regexp_like(valor, '^-?[0-9]{1,29}([.][0-9]{1,9})?$') THEN valor
            ELSE NULL
        END
        AS DECIMAL(38, 9)
    ) AS valor_decimal
FROM bronze
