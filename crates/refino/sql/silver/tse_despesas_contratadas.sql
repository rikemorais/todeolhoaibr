-- Contrato tse.prestacao_contas.despesa_contratada.2024.v1.
-- SQ_DESPESA não é chave única: preserve variantes e identifique ocorrências pela linhagem.
WITH origem AS (
    SELECT
        TRIM("AA_ELEICAO") AS ano_texto,
        TRIM("CD_ELEICAO") AS codigo_eleicao,
        TRIM("SQ_PRESTADOR_CONTAS") AS sequencia_prestador,
        NULLIF(TRIM("SQ_DESPESA"), '') AS sequencia_despesa_texto,
        TRIM("DT_DESPESA") AS data_texto,
        NULLIF(TRIM("DS_DESPESA"), '') AS descricao_despesa,
        TRIM("VR_DESPESA_CONTRATADA") AS valor_texto,
        CASE
            WHEN regexp_like(TRIM("DT_DESPESA"), '^[0-9]{2}/[0-9]{2}/[0-9]{4}$')
                THEN regexp_replace(TRIM("DT_DESPESA"), '^([0-9]{2})/([0-9]{2})/([0-9]{4})$', '$3-$2-$1')
        END AS data_iso,
        regexp_replace(TRIM("VR_DESPESA_CONTRATADA"), ',', '.', 'g') AS valor_normalizado,
        _tdoa_json_original AS registro_original,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
    WHERE regexp_like(upper(_tdoa_origem_url), 'DESPESAS_CONTRATADAS_CANDIDATOS_2024_BRASIL([.]CSV|[.]TXT)?$')
), tipadas AS (
    SELECT *,
        arrow_try_cast(ano_texto, 'UInt16') AS ano_eleicao,
        CASE WHEN sequencia_despesa_texto NOT IN ('-1', '-3', '#NULO', '#NE')
            THEN arrow_try_cast(sequencia_despesa_texto, 'UInt32') END AS sequencia_despesa,
        arrow_try_cast(data_iso, 'Date32') AS data_despesa,
        CASE WHEN regexp_like(valor_texto, '^[0-9]+,[0-9]{2}$')
            THEN TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) END AS valor_contratado
    FROM origem
)
SELECT
    ano_eleicao,
    codigo_eleicao,
    sequencia_prestador,
    sequencia_despesa,
    data_despesa,
    descricao_despesa,
    valor_contratado,
    registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipadas
WHERE tdoa_assert_true_named(ano_eleicao = 2024, 'ano_do_recurso')
  AND tdoa_assert_true_named(codigo_eleicao NOT IN ('', '-1', '-3'), 'codigo_eleicao')
  AND tdoa_assert_true_named(sequencia_prestador NOT IN ('', '-1', '-3'), 'sequencia_prestador')
  AND tdoa_assert_true_named(
      sequencia_despesa_texto IS NULL
      OR sequencia_despesa_texto IN ('-1', '-3', '#NULO', '#NE')
      OR sequencia_despesa IS NOT NULL,
      'sequencia_despesa'
  )
  AND tdoa_assert_true_named(valor_contratado IS NOT NULL, 'valor_contratado')
