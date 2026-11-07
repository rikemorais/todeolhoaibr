-- Contrato de sessões do Senado: preservar cada linha e validar a chave do conjunto.
WITH origem AS (
    SELECT {{COLUNAS_BRONZE}},
        _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal,
        COUNT(*) OVER (PARTITION BY "codigoSessaoVotacao") AS ocorrencias_sessao
    FROM bronze AS origem
)
SELECT {{COLUNAS_BRONZE}}
FROM origem
WHERE tdoa_assert_true(
    "codigoSessaoVotacao" IS NOT NULL
    AND TRIM("codigoSessaoVotacao") <> ''
    AND ocorrencias_sessao = 1
)
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
