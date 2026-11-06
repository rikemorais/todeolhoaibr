#!/bin/sh
# Ponto de entrada do serviço de consulta (DuckDB). Monta o init SQL (extensões,
# limites, segredo S3 do versitygw e as views Bronze/Silver) e sobe o console web, a UI ou o SQL.
#
#   web  (padrão)  Sobe o console SQL web (nosso, self-hosted) em primeiro plano.
#   ui             Sobe a DuckDB Local UI (frontend vem de ui.duckdb.org; instável — ver #167).
#   sql            Abre o CLI do DuckDB já com as views carregadas (offline).
#   <cmd...>       Executa o comando como veio (escape hatch).
set -eu

INIT="/tmp/init.sql"
BANCO="${TODEOLHO_DUCKDB_BANCO:-/tmp/consulta.duckdb}"
UI_PORT_INTERNO="${TODEOLHO_UI_PORT_INTERNO:-4200}"
UI_PORT="${TODEOLHO_UI_PORT:-4213}"

case "${1:-web}" in
web)
    # O console constrói seu próprio prefixo (views por filesystem); não usa o init.
    export TODEOLHO_CONSULTA_PORTA_INTERNA="${UI_PORT}"
    exec python3 /app/servidor.py
    ;;
ui)
    # Gera o init SQL (extensões, limites, segredo S3 e views Bronze/Silver) num arquivo.
    sh /app/gerar_init.sh >"$INIT"
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
    sh /app/gerar_init.sh >"$INIT"
    exec duckdb "${BANCO}" -init "${INIT}"
    ;;
*)
    exec "$@"
    ;;
esac
