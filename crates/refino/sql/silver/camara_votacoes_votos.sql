-- Contrato para votos individuais; linhas sem chave completa são preservadas e
-- não participam da validação de unicidade, conforme o contrato vigente.
WITH contadas AS (
    SELECT *, COUNT(*) OVER (PARTITION BY "idVotacao", deputado_id) AS __tdoa_contagem_chave
    FROM bronze
)
SELECT {{COLUNAS_BRONZE}}
FROM contadas AS origem
WHERE tdoa_assert_true(
    origem."idVotacao" IS NULL
    OR TRIM(origem."idVotacao") = ''
    OR origem.deputado_id IS NULL
    OR TRIM(origem.deputado_id) = ''
    OR origem.__tdoa_contagem_chave = 1
)
