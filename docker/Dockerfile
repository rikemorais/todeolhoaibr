# Etapa de compilação. Multi-stage clássico, sem recursos exclusivos do BuildKit,
# para buildar em qualquer Docker, inclusive sem o plugin buildx. O cache de
# camadas reaproveita a compilação enquanto Cargo.toml, Cargo.lock e crates não
# mudam. Em máquinas com pouca memória, reduza os jobs: TODEOLHO_BUILD_JOBS=1 no .env.
FROM rust:1.99-slim-bookworm AS compilacao
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --locked --bin todeolho \
    && cp target/release/todeolho /usr/local/bin/todeolho

# Imagem final: só o binário, sem compilador e sem usuário root.
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=compilacao /usr/local/bin/todeolho /usr/local/bin/todeolho
ENTRYPOINT ["/usr/local/bin/todeolho"]
