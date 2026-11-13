-- Contrato tse.prestacao_contas.receita_candidato.2014.v1.
-- Preserva cada ocorrência do consolidado BRASIL com identidade técnica Bronze.
WITH origem AS (
    SELECT
        TRIM("Cód. Eleição") AS codigo_eleicao,
        TRIM("Sequencial Candidato") AS sequencia_candidato,
        TRIM("CPF do candidato") AS cpf_candidato,
        NULLIF(TRIM("Data da receita"), '') AS data_texto,
        regexp_replace(
            NULLIF(TRIM("Data da receita"), ''),
            '^([0-9]{2})/([0-9]{2})/([0-9]{4})[0-9]{2}:[0-9]{2}:[0-9]{2}$',
            '$3-$2-$1'
        ) AS data_iso,
        regexp_replace(TRIM("Valor receita"), ',', '.', 'g') AS valor_normalizado,
        NULLIF(TRIM("Tipo receita"), '') AS tipo_receita,
        NULLIF(TRIM("Descrição da receita"), '') AS descricao_receita,
        _tdoa_json_original AS registro_original,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
    WHERE regexp_like(upper(_tdoa_origem_url), 'RECEITAS_CANDIDATOS_2014_BRASIL([.]CSV|[.]TXT)?$')
), tipadas AS (
    SELECT *,
        arrow_try_cast(data_iso, 'Date32') AS data_receita,
        TRY_CAST(valor_normalizado AS DECIMAL(18, 2)) AS valor_receita
    FROM origem
)
SELECT
    arrow_try_cast('2014', 'UInt16') AS ano_eleicao,
    codigo_eleicao,
    sequencia_candidato,
    cpf_candidato,
    data_receita,
    valor_receita,
    tipo_receita,
    descricao_receita,
    registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipadas
WHERE tdoa_assert_true_named(codigo_eleicao NOT IN ('', '#NULO', '#NE', '-1', '-3'), 'codigo_eleicao')
  AND tdoa_assert_true_named(sequencia_candidato NOT IN ('', '#NULO', '#NE', '-1', '-3'), 'sequencia_candidato')
  AND tdoa_assert_true_named(cpf_candidato NOT IN ('', '#NULO', '#NE', '-1', '-3'), 'cpf_candidato')
  AND tdoa_assert_true_named(data_texto IS NULL OR data_receita IS NOT NULL, 'data_receita')
  AND tdoa_assert_true_named(valor_receita IS NOT NULL, 'valor_receita')
