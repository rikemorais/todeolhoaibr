-- Contrato emendas-siga-brasil.favorecido-pagamento.v2.
-- Preserva a localidade e acrescenta IBGE por componente exato e UF; a origem não fornece OB.
-- CPF/CNPJ permanece fora da Silver para limitar a exposição de dados pessoais.
WITH entrada AS (
    SELECT
        NULLIF(TRIM("Favorecido do Pagamento - Município/UF"), '') AS localidade_origem,
        NULLIF(TRIM("Favorecido do Pagamento (Natureza Subgrupo)"), '') AS natureza_origem,
        NULLIF(TRIM("Favorecido do Pagamento (Tipo)"), '') AS tipo_favorecido_origem,
        UPPER(NULLIF(TRIM("Favorecido do Pagamento (UF)"), '')) AS uf_origem,
        tdoa_municipio_ibge_siga_codigo(
            "Favorecido do Pagamento (UF)",
            "Favorecido do Pagamento - Município/UF"
        ) AS codigo_municipio_ibge,
        tdoa_municipio_ibge_siga_nome(
            "Favorecido do Pagamento (UF)",
            "Favorecido do Pagamento - Município/UF"
        ) AS nome_municipio_ibge,
        tdoa_municipio_ibge_siga_metodo(
            "Favorecido do Pagamento (UF)",
            "Favorecido do Pagamento - Município/UF"
        ) AS metodo_mapeamento_municipio,
        NULLIF(TRIM("Autor"), '') AS autor_origem,
        NULLIF(TRIM("Emenda (Número/Ano)"), '') AS emenda_origem,
        NULLIF(TRIM("Pago + RP (Favorecido lista OB)"), '') AS valor_origem,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM bronze
), tipada AS (
    SELECT
        localidade_origem AS localidade_favorecido,
        natureza_origem AS natureza_subgrupo_favorecido,
        tipo_favorecido_origem AS tipo_favorecido,
        uf_origem AS uf,
        codigo_municipio_ibge,
        nome_municipio_ibge,
        metodo_mapeamento_municipio,
        autor_origem AS autor,
        regexp_replace(emenda_origem, '-[0-9]{4}$', '') AS numero_emenda_origem,
        TRY_CAST(regexp_replace(emenda_origem, '^.*-([0-9]{4})$', '$1') AS SMALLINT) AS ano_emenda,
        TRY_CAST(valor_origem AS DECIMAL(20, 2)) AS valor_pago_mais_resto_a_pagar,
        emenda_origem,
        uf_origem,
        _tdoa_origem_manifesto,
        _tdoa_origem_parte,
        _tdoa_origem_ordinal
    FROM entrada
)
SELECT
    localidade_favorecido,
    natureza_subgrupo_favorecido,
    tipo_favorecido,
    uf,
    codigo_municipio_ibge,
    nome_municipio_ibge,
    metodo_mapeamento_municipio,
    autor,
    numero_emenda_origem AS numero_emenda,
    ano_emenda,
    valor_pago_mais_resto_a_pagar,
    _tdoa_origem_manifesto,
    _tdoa_origem_parte,
    _tdoa_origem_ordinal
FROM tipada
WHERE tdoa_assert_true_named(
        localidade_favorecido IS NOT NULL AND uf IS NOT NULL
            AND autor IS NOT NULL AND ano_emenda IS NOT NULL
            AND numero_emenda_origem IS NOT NULL,
        'emenda_e_localidade_obrigatorias')
  AND tdoa_emendas_corresponde_filtro(CAST(ano_emenda AS VARCHAR), uf_origem)
ORDER BY _tdoa_origem_manifesto, _tdoa_origem_parte, _tdoa_origem_ordinal
