-- Contrato para votos individuais; linhas sem chave completa são preservadas e
-- não participam da validação de unicidade, conforme o contrato vigente.
WITH contadas AS (
    SELECT *, COUNT(*) OVER (PARTITION BY "idVotacao", deputado_id) AS __tdoa_contagem_chave
    FROM bronze
)
SELECT {{COLUNAS_BRONZE}},
    origem._tdoa_origem_manifesto,
    origem._tdoa_origem_parte,
    origem._tdoa_origem_ordinal
FROM contadas AS origem
WHERE tdoa_assert_true(
    origem."idVotacao" IS NULL
    OR TRIM(origem."idVotacao") = ''
    OR origem.deputado_id IS NULL
    OR TRIM(origem.deputado_id) = ''
    OR origem.__tdoa_contagem_chave = 1
)
