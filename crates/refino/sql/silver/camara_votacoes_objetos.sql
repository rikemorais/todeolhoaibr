-- Contrato para objetos associados à votação; preserva todas as colunas Bronze.
WITH contadas AS (
    SELECT *, COUNT(*) OVER (PARTITION BY "idVotacao", proposicao_id) AS __tdoa_contagem_chave
    FROM bronze
)
SELECT {{COLUNAS_BRONZE}},
    origem._tdoa_origem_manifesto,
    origem._tdoa_origem_parte,
    origem._tdoa_origem_ordinal
FROM contadas AS origem
WHERE tdoa_assert_true(
    origem."idVotacao" IS NOT NULL AND TRIM(origem."idVotacao") <> ''
    AND origem.proposicao_id IS NOT NULL AND TRIM(origem.proposicao_id) <> ''
)
  AND tdoa_assert_true(origem.__tdoa_contagem_chave = 1)
