# ============================================
# Dockerfile para Gestor Financiero - Fly.io
# ============================================
# El binario se compila LOCALMENTE antes del deploy
#   cargo build --release && strip target/release/gestor-financiero-server
# ============================================

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        && rm -rf /var/lib/apt/lists/*

COPY target/release/gestor-financiero-server /usr/local/bin/
COPY static/index.html /app/static/index.html

EXPOSE 3000

CMD ["/usr/local/bin/gestor-financiero-server"]
