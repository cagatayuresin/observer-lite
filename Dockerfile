# ── Frontend assets, baked into the Rust binary ──────────────────────────────
FROM node:22-alpine AS frontend
WORKDIR /src/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --ignore-scripts
COPY frontend/ ./
RUN npm run build

# ── Static musl binary ────────────────────────────────────────────────────────
FROM rust:1-alpine AS rust
RUN apk add --no-cache musl-dev gcc
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY VERSION ./VERSION
COPY --from=frontend /src/frontend/dist ./frontend/dist
RUN cargo build --release --locked

# ── Runtime: no Node, no Python ───────────────────────────────────────────────
FROM alpine:3.22
ARG APP_VERSION=0.3.0-alpha.1

LABEL org.opencontainers.image.title="Observer Lite" \
      org.opencontainers.image.description="Lightweight self-hosted uptime monitoring." \
      org.opencontainers.image.version="${APP_VERSION}" \
      org.opencontainers.image.source="https://github.com/cagatayuresin/observer-lite" \
      org.opencontainers.image.licenses="MIT"

RUN apk add --no-cache ca-certificates wget su-exec \
 && adduser -D -H -u 10001 observer

COPY --from=rust /src/target/release/observer-lite /usr/local/bin/observer-lite
COPY docker-entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENV PORT=3000 \
    DATABASE_PATH=/data/observer.db

VOLUME ["/data"]
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- "http://127.0.0.1:${PORT}/api/health" || exit 1

ENTRYPOINT ["/entrypoint.sh"]
