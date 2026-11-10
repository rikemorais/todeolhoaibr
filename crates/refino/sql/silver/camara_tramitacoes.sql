-- Contrato camara.tramitacao.evento.v1. Repetições de eventos são preservadas.
WITH tipadas AS (
    SELECT
        "uriProposicao",
        arrow_try_cast(NULLIF(TRIM(sequencia), ''), 'UInt32') AS sequencia,
        arrow_try_cast(NULLIF(TRIM("dataHora"), ''), 'Timestamp(µs)') AS data_hora,
        {{OPCIONAL:idTramitacao}},
        {{OPCIONAL:codSituacao}},
        {{OPCIONAL:numero}},
        {{OPCIONAL:siglaOrgao}},
        {{OPCIONAL:uriOrgao}},
        {{OPCIONAL:regime}},
        {{OPCIONAL:descricaoTramitacao}},
        {{OPCIONAL:despacho}},
        _tdoa_json_original AS registro_original,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
    WHERE tdoa_assert_true("uriProposicao" IS NOT NULL AND TRIM("uriProposicao") <> '')
      AND tdoa_assert_true(
        sequencia IS NULL OR TRIM(sequencia) = ''
        OR regexp_like(TRIM(sequencia), '^[0-9]{1,10}$')
    )
      AND tdoa_assert_true(
        "dataHora" IS NULL OR TRIM("dataHora") = ''
        OR TRY_CAST(TRIM("dataHora") AS TIMESTAMP) IS NOT NULL
    )
)
SELECT
    arrow_try_cast('{{ANO_RECURSO}}', 'UInt16') AS ano,
    TRIM("uriProposicao") AS uri_proposicao,
    sequencia,
    data_hora,
    "idTramitacao" AS id_tramitacao,
    "codSituacao" AS codigo_situacao,
    numero,
    "siglaOrgao" AS sigla_orgao,
    "uriOrgao" AS uri_orgao,
    regime,
    "descricaoTramitacao" AS descricao_tramitacao,
    despacho,
    registro_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipadas
