# Builds one image with the API server and the compiled frontend.
# Build from the repo root: docker build -t rollcall .

# ---- Frontend ----
FROM node:26-trixie-slim AS frontend
WORKDIR /src/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY frontend/ ./
RUN npm run build

# ---- Backend ----
FROM rust:1-trixie AS backend
WORKDIR /src/backend
# sqlx checks queries against the committed .sqlx metadata instead of a live DB.
ENV SQLX_OFFLINE=true
COPY backend/ ./
RUN cargo build --release --locked && cp target/release/rollcall /usr/local/bin/rollcall

# ---- Runtime ----
FROM debian:trixie-slim
# The upgrade picks up Debian security fixes released since the base image was
# published, so each weekly rebuild ships them.
RUN apt-get update \
 && apt-get upgrade -y \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 rollcall \
 && mkdir -p /app/uploads \
 && chown 10001 /app/uploads
COPY --from=backend /usr/local/bin/rollcall /usr/local/bin/rollcall
COPY --from=frontend /src/frontend/build /app/static
ENV BIND_ADDR=0.0.0.0:8080 \
    STATIC_DIR=/app/static \
    UPLOAD_DIR=/app/uploads \
    RUST_LOG=rollcall=info,tower_http=info
# Your uploaded photos. Mount a volume here so they outlive the container.
VOLUME /app/uploads
USER 10001
EXPOSE 8080
CMD ["rollcall"]
