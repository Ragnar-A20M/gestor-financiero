# ============================================
# Dockerfile para Gestor Financiero - Fly.io
# ============================================

# Etapa 1: Compilación
FROM rust:1.85-slim-bookworm AS builder

# Dependencias del sistema para compilar sqlx con native-tls
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Compilar en release para optimizar
RUN cargo build --release && \
    strip target/release/gestor-financiero-server

# Etapa 2: Imagen final mínima
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gestor-financiero-server /usr/local/bin/
COPY --from=builder /app/static/index.html /app/static/index.html

EXPOSE 3000

CMD ["/usr/local/bin/gestor-financiero-server"]
