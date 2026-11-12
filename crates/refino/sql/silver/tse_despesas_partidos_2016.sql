-- Contrato tse.prestacao_contas.despesa_partido.2016.v1.
-- Layout histórico de 2016: cabeçalhos acentuados, valor sem centavos ou com vírgula,
-- data colada ao horário (dd/MM/yyyyHH:mm:ss) e sem coluna de ano.
-- Diferente de despesas_candidatos: o prestador é o diretório partidário (CNPJ + sequencial).
WITH origem AS (
    SELECT
        TRIM("Cód. Eleição") AS codigo_eleicao,
        TRIM("Sequencial do Prestador de conta") AS sequencia_prestador,
        TRIM("CNPJ Prestador Conta") AS cnpj_prestador,
        NULLIF(TRIM("Data da despesa"), '') AS data_texto,
        TRIM("Valor despesa") AS valor_texto,
        NULLIF(TRIM("Tipo despesa"), '') AS tipo_despesa,
        NULLIF(TRIM("Descrição da despesa"), '') AS descricao_despesa,
        CASE
            WHEN regexp_like(TRIM("Data da despesa"), '^[0-9]{2}/[0-9]{2}/[0-9]{4}')
                THEN regexp_replace(
                    substr(TRIM("Data da despesa"), 1, 10),
                    '^([0-9]{2})/([0-9]{2})/([0-9]{4})$',
                    '$3-$2-$1'
                )
        END AS data_iso,
        regexp_replace(TRIM("Valor despesa"), ',', '.', 'g') AS valor_normalizado,
        _tdoa_json_original AS registro_original,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
    WHERE regexp_like(upper(_tdoa_origem_url), 'DESPESAS_PARTIDOS_PRESTACAO_CONTAS_FINAL_2016_BRASIL([.]CSV|[.]TXT)?$')
), tipadas AS (
    SELECT *,
        arrow_try_cast(data_iso, 'Date32') AS data_despesa,
        TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) AS valor_despesa
    FROM origem
)
SELECT
    CAST(2016 AS SMALLINT UNSIGNED) AS ano_eleicao,
    codigo_eleicao,
    sequencia_prestador,
    cnpj_prestador,
    data_despesa,
    valor_despesa,
    tipo_despesa,
    descricao_despesa,
    registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipadas
WHERE tdoa_assert_true_named(codigo_eleicao NOT IN ('', '-1', '-3'), 'codigo_eleicao')
  AND tdoa_assert_true_named(sequencia_prestador NOT IN ('', '-1', '-3'), 'sequencia_prestador')
  AND tdoa_assert_true_named(cnpj_prestador NOT IN ('', '-1', '-3'), 'cnpj_prestador')
  AND tdoa_assert_true_named(
      data_texto IS NULL
      OR data_despesa IS NOT NULL,
      'data_despesa'
  )
  AND tdoa_assert_true_named(valor_despesa IS NOT NULL, 'valor_despesa')
