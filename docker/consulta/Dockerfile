# DuckDB como "Athena local" para consultar o medalhão (ADR 0018, issue #158).
# Imagem enxuta: só o CLI do DuckDB + socat (ponte da UI) + python3 (gera as views).
FROM debian:bookworm-slim

# DuckDB v2.0.0, linux-amd64 (glibc). O sha256 vem da release no GitHub.
ARG DUCKDB_VERSAO=v2.0.0
ARG DUCKDB_SHA256=c6507206baf7793c6013dbd82368469a45f20dc0e17a6f92f204602f652ed2bc

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl socat python3 \
    && rm -rf /var/lib/apt/lists/*

# Baixa o CLI, confere o hash e instala só o binário.
RUN curl -fsSL -o /tmp/duckdb.tar.gz \
    "https://github.com/duckdb/duckdb/releases/download/${DUCKDB_VERSAO}/duckdb-cli-linux-amd64.tar.gz" \
    && echo "${DUCKDB_SHA256}  /tmp/duckdb.tar.gz" | sha256sum -c - \
    && tar -xzf /tmp/duckdb.tar.gz -C /usr/local/bin duckdb \
    && rm /tmp/duckdb.tar.gz \
    && duckdb --version

# Usuário sem privilégios; o cache das extensões fica no HOME dele.
RUN useradd --create-home --uid 10001 consulta
USER consulta
ENV HOME=/home/consulta

# Pré-instala as extensões no build para não baixá-las em runtime. O frontend da
# UI, porém, ainda é servido de ui.duckdb.org quando a UI abre (as queries rodam
# 100% locais).
RUN duckdb -c "INSTALL httpfs; INSTALL parquet; INSTALL ui;"

COPY --chown=consulta:consulta docker/consulta/entrypoint.sh /app/entrypoint.sh
COPY --chown=consulta:consulta docker/consulta/gerar_views.py /app/gerar_views.py

EXPOSE 4213
# Invoca via sh para não depender do bit de execução (nem de BuildKit no COPY).
ENTRYPOINT ["sh", "/app/entrypoint.sh"]
CMD ["ui"]
