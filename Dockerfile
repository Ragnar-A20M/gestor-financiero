# ============================================
# Dockerfile multistage para Gestor Financiero
# Compila dentro del contenedor en Fly.io
# ============================================

# ---- Stage 1: Compilación ----
FROM rust:bookworm AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY static/ static/

RUN cargo build --release && \
    strip target/release/gestor-financiero-server

# ---- Stage 2: Runtime ----
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gestor-financiero-server /usr/local/bin/

EXPOSE 3000

CMD ["/usr/local/bin/gestor-financiero-server"]