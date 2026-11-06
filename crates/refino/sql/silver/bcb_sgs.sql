-- Contrato bcb.sgs.observacao.v1.
-- Datas e decimais inválidos falham na consulta; nenhum registro é descartado.
SELECT
    to_date(data, '%d/%m/%Y') AS data,
    CAST(
        CASE
            WHEN regexp_like(valor, '^-?[0-9]{1,12}([.][0-9]{1,6})?$') THEN valor
            ELSE '__valor_bcb_invalido__'
        END
        AS DECIMAL(18, 6)
    ) AS valor
FROM bronze
