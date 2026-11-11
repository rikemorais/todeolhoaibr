-- Contrato tse.prestacao_contas.receita_doador_originario.2022.v1.
-- As chaves de negócio testadas colidem; preserve ocorrências e variantes pela linhagem.
WITH origem AS (
    SELECT
        TRIM("AA_ELEICAO") AS ano_texto,
        TRIM("CD_ELEICAO") AS codigo_eleicao,
        TRIM("SQ_PRESTADOR_CONTAS") AS sequencia_prestador,
        CASE WHEN TRIM("NR_CPF_CNPJ_DOADOR_ORIGINARIO") IN ('', '-1', '-3', '#NULO', '#NE') THEN NULL
            ELSE TRIM("NR_CPF_CNPJ_DOADOR_ORIGINARIO") END AS documento_doador,
        CASE WHEN TRIM("NM_DOADOR_ORIGINARIO") IN ('', '-1', '-3', '#NULO', '#NE') THEN NULL
            ELSE TRIM("NM_DOADOR_ORIGINARIO") END AS nome_doador,
        CASE WHEN TRIM("NM_DOADOR_ORIGINARIO_RFB") IN ('', '-1', '-3', '#NULO', '#NE') THEN NULL
            ELSE TRIM("NM_DOADOR_ORIGINARIO_RFB") END AS nome_doador_rfb,
        CASE WHEN TRIM("TP_DOADOR_ORIGINARIO") IN ('', '-1', '-3', '#NULO', '#NE') THEN NULL
            ELSE TRIM("TP_DOADOR_ORIGINARIO") END AS tipo_doador,
        CASE WHEN TRIM("CD_CNAE_DOADOR_ORIGINARIO") IN ('', '-1', '-3', '#NULO', '#NE') THEN NULL
            ELSE TRIM("CD_CNAE_DOADOR_ORIGINARIO") END AS codigo_cnae,
        CASE WHEN TRIM("DS_CNAE_DOADOR_ORIGINARIO") IN ('', '-1', '-3', '#NULO', '#NE') THEN NULL
            ELSE TRIM("DS_CNAE_DOADOR_ORIGINARIO") END AS descricao_cnae,
        NULLIF(TRIM("SQ_RECEITA"), '') AS sequencia_receita_texto,
        TRIM("DT_RECEITA") AS data_texto,
        CASE WHEN TRIM("DS_RECEITA") IN ('', '-1', '-3', '#NULO', '#NE') THEN NULL
            ELSE TRIM("DS_RECEITA") END AS descricao_receita,
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
    WHERE regexp_like(upper(_tdoa_origem_url), 'RECEITAS_CANDIDATOS_DOADOR_ORIGINARIO_2022_BRASIL([.]CSV|[.]TXT)?$')
), tipadas AS (
    SELECT *,
        arrow_try_cast(ano_texto, 'UInt16') AS ano_eleicao,
        CASE WHEN sequencia_receita_texto NOT IN ('-1', '-3', '#NULO', '#NE')
            THEN arrow_try_cast(sequencia_receita_texto, 'UInt32') END AS sequencia_receita,
        arrow_try_cast(data_iso, 'Date32') AS data_receita,
        CASE WHEN regexp_like(valor_texto, '^[0-9]+,[0-9]{2}$')
            THEN TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) END AS valor_receita
    FROM origem
)
SELECT
    ano_eleicao,
    codigo_eleicao,
    sequencia_prestador,
    documento_doador,
    nome_doador,
    nome_doador_rfb,
    tipo_doador,
    codigo_cnae,
    descricao_cnae,
    sequencia_receita,
    data_receita,
    descricao_receita,
    valor_receita,
    registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipadas
WHERE tdoa_assert_true_named(ano_eleicao = 2022, 'ano_do_recurso')
  AND tdoa_assert_true_named(codigo_eleicao NOT IN ('', '-1', '-3'), 'codigo_eleicao')
  AND tdoa_assert_true_named(sequencia_prestador NOT IN ('', '-1', '-3'), 'sequencia_prestador')
  AND tdoa_assert_true_named(
      sequencia_receita_texto IN ('-1', '-3', '#NULO', '#NE')
      OR sequencia_receita IS NOT NULL,
      'sequencia_receita'
  )
  AND tdoa_assert_true_named(
      data_texto IN ('', '-1', '-3', '#NULO', '#NE') OR data_receita IS NOT NULL,
      'data_receita'
  )
  AND tdoa_assert_true_named(valor_receita IS NOT NULL, 'valor_receita')
