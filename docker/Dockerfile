# Builds one image with the API server and the compiled frontend.
# Build from the repo root: docker build -f docker/Dockerfile -t rollcall .

# ---- Frontend ----
FROM node:22-bookworm-slim AS frontend
WORKDIR /src/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY frontend/ ./
RUN npm run build

# ---- Backend ----
FROM rust:1-bookworm AS backend
WORKDIR /src/backend
# sqlx checks queries against the committed .sqlx metadata instead of a live DB.
ENV SQLX_OFFLINE=true
COPY backend/ ./
RUN cargo build --release --locked && cp target/release/rollcall /usr/local/bin/rollcall

# ---- Runtime ----
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 rollcall
COPY --from=backend /usr/local/bin/rollcall /usr/local/bin/rollcall
COPY --from=frontend /src/frontend/build /app/static
ENV BIND_ADDR=0.0.0.0:8080 \
    STATIC_DIR=/app/static \
    RUST_LOG=rollcall=info,tower_http=info
USER rollcall
EXPOSE 8080
CMD ["rollcall"]
