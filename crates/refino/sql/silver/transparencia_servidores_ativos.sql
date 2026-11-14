-- Contrato transparencia.servidores-ativos.v1.
-- Servidores ativos do Portal da Transparência (SIAPE e Bacen).
-- O Bronze decodifica o ZIP/CSV com separador `;` em colunas Utf8.
-- Refinar com colunas explícitas após smoke test com dados reais.
SELECT *
FROM bronze
