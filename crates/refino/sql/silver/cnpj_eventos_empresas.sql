-- Contrato cnpj.eventos.empresas.v1.
-- Compara dois snapshots Bronze consecutivos da tabela de empresas e produz
-- eventos de abertura, fechamento e alteração campo a campo.
--
-- Tabelas registradas: `vigente` (snapshot novo) e `anterior` (snapshot antigo).
-- Placeholders {{MANIFESTO_VIGENTE}} e {{MANIFESTO_ANTERIOR}} são substituídos
-- em Rust antes da execução.

-- Aberturas: CNPJ presente no vigente, ausente no anterior.
SELECT
    'abertura'                           AS tipo_evento,
    'empresas'                           AS tabela,
    v.cnpj_basico,
    CAST(NULL AS VARCHAR)                AS campo_alterado,
    CAST(NULL AS VARCHAR)                AS valor_anterior,
    CAST(NULL AS VARCHAR)                AS valor_novo,
    '{{MANIFESTO_ANTERIOR}}'             AS manifesto_anterior,
    '{{MANIFESTO_VIGENTE}}'              AS manifesto_novo,
    v._tdoa_origem_manifesto,
    v._tdoa_origem_parte,
    v._tdoa_origem_ordinal
FROM vigente v
LEFT JOIN anterior a ON v.cnpj_basico = a.cnpj_basico
WHERE a.cnpj_basico IS NULL

UNION ALL

-- Fechamentos: CNPJ presente no anterior, ausente no vigente.
SELECT
    'fechamento'                         AS tipo_evento,
    'empresas'                           AS tabela,
    a.cnpj_basico,
    CAST(NULL AS VARCHAR)                AS campo_alterado,
    CAST(NULL AS VARCHAR)                AS valor_anterior,
    CAST(NULL AS VARCHAR)                AS valor_novo,
    '{{MANIFESTO_ANTERIOR}}'             AS manifesto_anterior,
    '{{MANIFESTO_VIGENTE}}'              AS manifesto_novo,
    a._tdoa_origem_manifesto,
    a._tdoa_origem_parte,
    a._tdoa_origem_ordinal
FROM anterior a
LEFT JOIN vigente v ON a.cnpj_basico = v.cnpj_basico
WHERE v.cnpj_basico IS NULL

UNION ALL

-- Alteração: razao_social
SELECT
    'alteracao'                          AS tipo_evento,
    'empresas'                           AS tabela,
    v.cnpj_basico,
    'razao_social'                       AS campo_alterado,
    a.razao_social                       AS valor_anterior,
    v.razao_social                       AS valor_novo,
    '{{MANIFESTO_ANTERIOR}}'             AS manifesto_anterior,
    '{{MANIFESTO_VIGENTE}}'              AS manifesto_novo,
    v._tdoa_origem_manifesto,
    v._tdoa_origem_parte,
    v._tdoa_origem_ordinal
FROM vigente v
INNER JOIN anterior a ON v.cnpj_basico = a.cnpj_basico
WHERE COALESCE(v.razao_social, '') != COALESCE(a.razao_social, '')

UNION ALL

-- Alteração: natureza_juridica
SELECT
    'alteracao'                          AS tipo_evento,
    'empresas'                           AS tabela,
    v.cnpj_basico,
    'natureza_juridica'                  AS campo_alterado,
    a.natureza_juridica                  AS valor_anterior,
    v.natureza_juridica                  AS valor_novo,
    '{{MANIFESTO_ANTERIOR}}'             AS manifesto_anterior,
    '{{MANIFESTO_VIGENTE}}'              AS manifesto_novo,
    v._tdoa_origem_manifesto,
    v._tdoa_origem_parte,
    v._tdoa_origem_ordinal
FROM vigente v
INNER JOIN anterior a ON v.cnpj_basico = a.cnpj_basico
WHERE COALESCE(v.natureza_juridica, '') != COALESCE(a.natureza_juridica, '')

UNION ALL

-- Alteração: qualificacao_responsavel
SELECT
    'alteracao'                          AS tipo_evento,
    'empresas'                           AS tabela,
    v.cnpj_basico,
    'qualificacao_responsavel'           AS campo_alterado,
    a.qualificacao_responsavel           AS valor_anterior,
    v.qualificacao_responsavel           AS valor_novo,
    '{{MANIFESTO_ANTERIOR}}'             AS manifesto_anterior,
    '{{MANIFESTO_VIGENTE}}'              AS manifesto_novo,
    v._tdoa_origem_manifesto,
    v._tdoa_origem_parte,
    v._tdoa_origem_ordinal
FROM vigente v
INNER JOIN anterior a ON v.cnpj_basico = a.cnpj_basico
WHERE COALESCE(v.qualificacao_responsavel, '') != COALESCE(a.qualificacao_responsavel, '')

UNION ALL

-- Alteração: capital_social
SELECT
    'alteracao'                          AS tipo_evento,
    'empresas'                           AS tabela,
    v.cnpj_basico,
    'capital_social'                     AS campo_alterado,
    a.capital_social                     AS valor_anterior,
    v.capital_social                     AS valor_novo,
    '{{MANIFESTO_ANTERIOR}}'             AS manifesto_anterior,
    '{{MANIFESTO_VIGENTE}}'              AS manifesto_novo,
    v._tdoa_origem_manifesto,
    v._tdoa_origem_parte,
    v._tdoa_origem_ordinal
FROM vigente v
INNER JOIN anterior a ON v.cnpj_basico = a.cnpj_basico
WHERE COALESCE(v.capital_social, '') != COALESCE(a.capital_social, '')

UNION ALL

-- Alteração: porte_empresa
SELECT
    'alteracao'                          AS tipo_evento,
    'empresas'                           AS tabela,
    v.cnpj_basico,
    'porte_empresa'                      AS campo_alterado,
    a.porte_empresa                      AS valor_anterior,
    v.porte_empresa                      AS valor_novo,
    '{{MANIFESTO_ANTERIOR}}'             AS manifesto_anterior,
    '{{MANIFESTO_VIGENTE}}'              AS manifesto_novo,
    v._tdoa_origem_manifesto,
    v._tdoa_origem_parte,
    v._tdoa_origem_ordinal
FROM vigente v
INNER JOIN anterior a ON v.cnpj_basico = a.cnpj_basico
WHERE COALESCE(v.porte_empresa, '') != COALESCE(a.porte_empresa, '')

UNION ALL

-- Alteração: ente_federativo
SELECT
    'alteracao'                          AS tipo_evento,
    'empresas'                           AS tabela,
    v.cnpj_basico,
    'ente_federativo'                    AS campo_alterado,
    a.ente_federativo                    AS valor_anterior,
    v.ente_federativo                    AS valor_novo,
    '{{MANIFESTO_ANTERIOR}}'             AS manifesto_anterior,
    '{{MANIFESTO_VIGENTE}}'              AS manifesto_novo,
    v._tdoa_origem_manifesto,
    v._tdoa_origem_parte,
    v._tdoa_origem_ordinal
FROM vigente v
INNER JOIN anterior a ON v.cnpj_basico = a.cnpj_basico
WHERE COALESCE(v.ente_federativo, '') != COALESCE(a.ente_federativo, '')
