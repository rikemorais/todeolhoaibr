#!/bin/sh
# Ponto de entrada do serviço de consulta (DuckDB). Monta o init SQL (extensões,
# limites, segredo S3 do versitygw e as views do bronze) e sobe a UI ou o SQL.
#
#   ui   (padrão)  Sobe a DuckDB Local UI; fica em primeiro plano.
#   sql            Abre o CLI do DuckDB já com as views carregadas (offline).
#   <cmd...>       Executa o comando como veio (escape hatch).
set -eu

INIT="/tmp/init.sql"
BANCO="${TODEOLHO_DUCKDB_BANCO:-/tmp/consulta.duckdb}"
UI_PORT_INTERNO="${TODEOLHO_UI_PORT_INTERNO:-4200}"
UI_PORT="${TODEOLHO_UI_PORT:-4213}"

# A UI faz bind em 127.0.0.1 dentro do container; servimos o init num arquivo.
{
    echo "INSTALL httpfs; LOAD httpfs;"
    echo "INSTALL parquet; LOAD parquet;"
    echo "SET memory_limit = '${TODEOLHO_DUCKDB_MEMORIA:-384MB}';"
    echo "SET threads = ${TODEOLHO_DUCKDB_THREADS:-2};"
    # Segredo opcional: permite consultar via s3://bronze/... além do filesystem.
    cat <<SQL
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
} >"$INIT"

case "${1:-ui}" in
ui)
    echo "SET ui_local_port = ${UI_PORT_INTERNO};" >>"$INIT"
    echo "CALL start_ui_server();" >>"$INIT"
    # A UI faz bind em localhost -> ::1 (loopback IPv6); a ponte a expõe na porta
    # publicada do container (IPv4), encaminhando para o loopback IPv6.
    socat "TCP-LISTEN:${UI_PORT},fork,reuseaddr" "TCP6:localhost:${UI_PORT_INTERNO}" &
    echo "DuckDB UI em http://localhost:${UI_PORT} (o frontend vem de ui.duckdb.org; precisa de internet)."
    # O CLI precisa continuar vivo para o servidor da UI seguir no ar; stdin nunca fecha.
    exec sh -c "tail -f /dev/null | duckdb '${BANCO}' -init '${INIT}'"
    ;;
sql)
    exec duckdb "${BANCO}" -init "${INIT}"
    ;;
*)
    exec "$@"
    ;;
esac
