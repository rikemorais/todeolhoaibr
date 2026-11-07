-- Contrato para eventos de votação; eventos repetidos por ID são inválidos.
WITH contadas AS (
    SELECT *, COUNT(*) OVER (PARTITION BY id) AS __tdoa_contagem_chave
    FROM bronze
)
SELECT {{COLUNAS_BRONZE}}
FROM contadas AS origem
WHERE tdoa_assert_true(origem.id IS NOT NULL AND TRIM(origem.id) <> '')
  AND tdoa_assert_true(origem.__tdoa_contagem_chave = 1)
