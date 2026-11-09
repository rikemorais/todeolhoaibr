-- Contrato camara.legislatura.periodo.v1.
WITH tipadas AS (
    SELECT
        arrow_try_cast(id, 'UInt16') AS id,
        TRY_CAST("dataInicio" AS DATE) AS data_inicio,
        TRY_CAST("dataFim" AS DATE) AS data_fim,
        arrow_try_cast("dataEleicao", 'UInt16') AS ano_eleicao,
        uri,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
), classificados AS (
    SELECT *, COUNT(*) OVER (PARTITION BY id) AS ocorrencias_id
    FROM tipadas
)
SELECT
    CASE WHEN ocorrencias_id = 1 THEN id END AS id,
    CASE WHEN data_fim >= data_inicio THEN data_inicio END AS data_inicio,
    CASE WHEN data_fim >= data_inicio THEN data_fim END AS data_fim,
    ano_eleicao,
    uri,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM classificados
