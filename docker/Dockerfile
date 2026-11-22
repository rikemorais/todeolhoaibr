# syntax=docker/dockerfile:1.7
# BuildKit mantém o cache do Cargo entre builds, inclusive quando um crate muda.
# O perfil release continua sendo o padrão; o override local seleciona dev.
FROM rust:1.100-slim-bookworm AS compilacao
RUN apt-get update && apt-get install -y --no-install-recommends libssl-dev pkg-config \
    && rm -rf /var/lib/apt/lists/*
ARG CARGO_BUILD_JOBS=2
ARG CARGO_PROFILE=release
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --profile ${CARGO_PROFILE} --locked --bin todeolho \
    && perfil_dir=${CARGO_PROFILE} \
    && if [ "${CARGO_PROFILE}" = dev ]; then perfil_dir=debug; fi \
    && cp "/app/target/${perfil_dir}/todeolho" /usr/local/bin/todeolho

# Imagem final: só o binário, sem compilador e sem usuário root.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=compilacao /usr/local/bin/todeolho /usr/local/bin/todeolho
ENTRYPOINT ["/usr/local/bin/todeolho"]
