-- Contrato tce-ce.folhas-pagamentos.v1.
-- Totais das folhas municipais do TCE-CE/SIM.
-- O Bronze decodifica `elements` do JSON em colunas Utf8 achatadas.
-- Refinar com colunas explícitas após smoke test com dados reais.
SELECT *
FROM bronze
