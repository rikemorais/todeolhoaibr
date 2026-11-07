-- Contrato para orientações de bancada por votação.
WITH contadas AS (
    SELECT *, COUNT(*) OVER (PARTITION BY "idVotacao", "siglaBancada") AS __tdoa_contagem_chave
    FROM bronze
)
SELECT {{COLUNAS_BRONZE}}
FROM contadas AS origem
WHERE tdoa_assert_true(
    origem."idVotacao" IS NOT NULL AND TRIM(origem."idVotacao") <> ''
    AND origem."siglaBancada" IS NOT NULL AND TRIM(origem."siglaBancada") <> ''
)
  AND tdoa_assert_true(origem.__tdoa_contagem_chave = 1)
