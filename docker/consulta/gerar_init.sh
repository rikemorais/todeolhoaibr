#!/bin/sh
# Imprime o init SQL do DuckDB (extensões, limites, segredo S3 do versitygw e as views do
# bronze) em stdout. Usado pelo entrypoint e pelo "Recarregar" do console web.
set -eu

cat <<SQL
INSTALL httpfs; LOAD httpfs;
INSTALL parquet; LOAD parquet;
SET memory_limit = '${TODEOLHO_DUCKDB_MEMORIA:-384MB}';
SET threads = ${TODEOLHO_DUCKDB_THREADS:-2};
CREATE OR REPLACE SECRET versitygw (
  TYPE s3,
  KEY_ID '${TODEOLHO_S3_CHAVE_DE_ACESSO:-todeolho}',
  SECRET '${TODEOLHO_S3_CHAVE_SECRETA:-todeolho-local}',
  ENDPOINT '${TODEOLHO_S3_ENDPOINT_HOST:-armazenamento:7070}',
  URL_STYLE 'path',
  USE_SSL false,
  REGION '${TODEOLHO_S3_REGIAO:-sa-east-1}'
);
SQL

python3 /app/gerar_views.py
