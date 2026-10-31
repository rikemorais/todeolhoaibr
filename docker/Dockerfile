# syntax=docker/dockerfile:1

# Etapa de compilação. Os cache mounts reaproveitam dependências entre builds.
# Em máquinas com pouca memória, reduza os jobs: TODEOLHO_BUILD_JOBS=1 no .env.
FROM rust:1.99-slim-bookworm AS compilacao
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked --bin todeolho \
    && cp target/release/todeolho /usr/local/bin/todeolho

# Imagem final: só o binário, sem compilador e sem usuário root.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=compilacao /usr/local/bin/todeolho /usr/local/bin/todeolho
ENTRYPOINT ["/usr/local/bin/todeolho"]
