-- Contrato remuneracao-estadual-mg.folha.v1.
-- Passthrough das colunas Bronze do CSV de remuneração do Executivo de MG.
-- Os cabeçalhos dependem do leiaute publicado pelo CKAN (pós-2021 difere do
-- anterior). Refinar com colunas explícitas após smoke test com dados reais.
SELECT *
FROM bronze
