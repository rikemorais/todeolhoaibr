-- Contrato senado.voto.nominal.v1: uma linha para cada voto nominal.
WITH expandidos AS (
    SELECT
        bronze."codigoSessaoVotacao" AS codigo_sessao_votacao,
        bronze."dataSessao" AS data_sessao,
        bronze._tdoa_origem_manifesto,
        bronze._tdoa_origem_parte,
        bronze._tdoa_origem_ordinal,
        unnest(tdoa_json_expand_votos(bronze."votos")) AS voto_envelopado
    FROM bronze
), originais AS (
    SELECT
        codigo_sessao_votacao,
        data_sessao,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal,
        voto_envelopado,
        tdoa_json_get(voto_envelopado, '_tdoa_original') AS voto_original,
        arrow_try_cast(
            tdoa_json_get(voto_envelopado, '_tdoa_ordinal'), 'UInt32'
        ) AS ordem_voto
    FROM expandidos
), identificacoes AS (
    SELECT
        *,
        COALESCE(
            tdoa_json_get(voto_original, 'identificacaoParlamentar'),
            tdoa_json_get(voto_original, 'IdentificacaoParlamentar'),
            voto_original
        ) AS identidade
    FROM originais
), campos AS (
    SELECT
        *,
        COALESCE(
            tdoa_json_get(identidade, 'codigoParlamentar'),
            tdoa_json_get(identidade, 'CodigoParlamentar')
        ) AS codigo_parlamentar,
        COALESCE(
            tdoa_json_get(identidade, 'nomeParlamentar'),
            tdoa_json_get(identidade, 'NomeParlamentar')
        ) AS nome_parlamentar,
        COALESCE(
            tdoa_json_get(identidade, 'siglaPartido'),
            tdoa_json_get(identidade, 'SiglaPartido'),
            tdoa_json_get(identidade, 'SiglaPartidoParlamentar')
        ) AS sigla_partido,
        COALESCE(
            tdoa_json_get(identidade, 'siglaUf'),
            tdoa_json_get(identidade, 'siglaUF'),
            tdoa_json_get(identidade, 'SiglaUF'),
            tdoa_json_get(identidade, 'UfParlamentar')
        ) AS uf,
        COALESCE(
            tdoa_json_get(voto_original, 'voto'),
            tdoa_json_get(voto_original, 'SiglaVoto'),
            tdoa_json_get(voto_original, 'Voto'),
            tdoa_json_get(voto_original, 'descricaoVoto'),
            tdoa_json_get(voto_original, 'DescricaoVoto')
        ) AS voto
    FROM identificacoes
)
SELECT
    CASE WHEN tdoa_assert_true(
        codigo_sessao_votacao IS NOT NULL AND TRIM(codigo_sessao_votacao) <> ''
        AND codigo_parlamentar IS NOT NULL AND TRIM(codigo_parlamentar) <> ''
        AND ordem_voto IS NOT NULL
    ) THEN codigo_sessao_votacao END AS codigo_sessao_votacao,
    data_sessao,
    ordem_voto,
    codigo_parlamentar,
    CASE WHEN nome_parlamentar IS NOT NULL AND TRIM(nome_parlamentar) <> ''
        THEN nome_parlamentar END AS nome_parlamentar,
    CASE WHEN sigla_partido IS NOT NULL AND TRIM(sigla_partido) <> ''
        THEN sigla_partido END AS sigla_partido,
    CASE WHEN uf IS NOT NULL AND TRIM(uf) <> '' THEN uf END AS uf,
    CASE WHEN voto IS NOT NULL AND TRIM(voto) <> '' THEN voto END AS voto,
    voto_original,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM campos
ORDER BY
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal,
    ordem_voto
