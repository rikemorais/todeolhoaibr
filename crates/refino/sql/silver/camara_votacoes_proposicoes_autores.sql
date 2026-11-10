-- Contrato para autores e ordem de assinatura por proposição.
WITH contadas AS (
    SELECT *, COUNT(*) OVER (PARTITION BY "idProposicao", "ordemAssinatura") AS __tdoa_contagem_chave
    FROM bronze
)
SELECT {{COLUNAS_BRONZE}},
    origem._tdoa_origem_manifesto,
    origem._tdoa_origem_parte,
    origem._tdoa_origem_ordinal
FROM contadas AS origem
WHERE tdoa_assert_true(
    origem."idProposicao" IS NOT NULL AND TRIM(origem."idProposicao") <> ''
    AND origem."ordemAssinatura" IS NOT NULL AND TRIM(origem."ordemAssinatura") <> ''
)
  AND tdoa_assert_true(origem.__tdoa_contagem_chave = 1)
