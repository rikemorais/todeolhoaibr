-- Contrato tse.prestacao_contas.receita_candidato.2024.v1.
-- SQ_RECEITA não é uma chave única; a linhagem Bronze identifica cada ocorrência publicada.
WITH origem AS (
    SELECT
        TRIM("AA_ELEICAO") AS ano_texto,
        TRIM("CD_ELEICAO") AS codigo_eleicao_chave,
        TRIM("SQ_PRESTADOR_CONTAS") AS prestador_chave,
        NULLIF(TRIM("SQ_RECEITA"), '') AS sequencia_receita_texto,
        TRIM("DT_RECEITA") AS data_texto,
        TRIM("VR_RECEITA") AS valor_texto,
        CASE
            WHEN regexp_like(TRIM("DT_RECEITA"), '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
                THEN regexp_replace(TRIM("DT_RECEITA"), '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1')
            WHEN regexp_like(TRIM("DT_RECEITA"), '^[0-9]{4}-[0-9]{2}-[0-9]{2}$')
                THEN TRIM("DT_RECEITA")
        END AS data_iso,
        regexp_replace(TRIM("VR_RECEITA"), ',', '.', 'g') AS valor_normalizado,
        _tdoa_json_original AS registro_original,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
    WHERE regexp_like(upper(_tdoa_origem_url), 'RECEITAS_CANDIDATOS_2024_BRASIL([.]CSV|[.]TXT)?$')
), tipadas AS (
    SELECT *,
        arrow_try_cast(ano_texto, 'UInt16') AS ano_eleicao,
        CASE WHEN sequencia_receita_texto NOT IN ('-1', '-3')
            THEN arrow_try_cast(sequencia_receita_texto, 'UInt32') END AS sequencia_receita,
        arrow_try_cast(data_iso, 'Date32') AS data_receita,
        CASE WHEN regexp_like(valor_texto, '^[0-9]+,[0-9]{2}$')
            THEN TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) END AS valor_receita
    FROM origem
)
SELECT
    ano_eleicao,
    codigo_eleicao_chave AS codigo_eleicao,
    prestador_chave AS sequencia_prestador,
    sequencia_receita,
    data_receita,
    valor_receita,
    registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipadas
WHERE tdoa_assert_true_named(ano_eleicao = 2024, 'ano_do_recurso')
  AND tdoa_assert_true_named(codigo_eleicao_chave NOT IN ('', '-1', '-3'), 'codigo_eleicao')
  AND tdoa_assert_true_named(prestador_chave NOT IN ('', '-1', '-3'), 'sequencia_prestador')
  AND tdoa_assert_true_named(sequencia_receita_texto IN ('-1', '-3') OR sequencia_receita IS NOT NULL, 'sequencia_receita')
  AND tdoa_assert_true_named(data_receita IS NOT NULL, 'data_receita')
  AND tdoa_assert_true_named(valor_receita IS NOT NULL, 'valor_receita')
