-- Contrato camara.deputado.exercicio.v1.
-- Converte linhas sem identidade completa em nulos nos campos obrigatórios para
-- que o publicador rejeite o lote; COUNT também valida duplicidade do ID.
WITH classificados AS (
    SELECT
        *,
        COUNT(*) OVER (PARTITION BY id) AS ocorrencias_id
    FROM bronze
)
SELECT
    CASE WHEN ocorrencias_id = 1 AND regexp_like(id, '^[0-9]+$') THEN id END AS id,
    NULLIF(TRIM(uri), '') AS uri,
    NULLIF(TRIM(nome), '') AS nome,
    "siglaPartido" AS sigla_partido,
    "uriPartido" AS uri_partido,
    NULLIF(TRIM("siglaUf"), '') AS sigla_uf,
    NULLIF(TRIM("idLegislatura"), '') AS id_legislatura,
    "urlFoto" AS url_foto,
    email,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM classificados
