# ── Stage 1: Build frontend ──────────────────────────────────────────────────
FROM node:20-alpine AS frontend-builder
WORKDIR /build/frontend
COPY frontend/package*.json ./
RUN npm ci --ignore-scripts --silent
COPY frontend/ ./
RUN npm run build

# ── Stage 2: Install Python deps ─────────────────────────────────────────────
FROM python:3.12-slim AS backend-deps
COPY --from=ghcr.io/astral-sh/uv:0.12.19 /uv /usr/local/bin/uv
WORKDIR /build
COPY backend/pyproject.toml backend/uv.lock ./
RUN uv export --frozen --no-dev --no-emit-project --output-file /tmp/requirements.txt \
 && uv pip install --python /usr/local/bin/python --prefix=/install --only-binary :all: --require-hashes -r /tmp/requirements.txt

# ── Stage 3: Runtime image ────────────────────────────────────────────────────
FROM python:3.12-slim
ARG APP_VERSION=0.0.0

LABEL org.opencontainers.image.title="Observer Lite" \
      org.opencontainers.image.description="Lightweight self-hosted uptime monitoring with email and Telegram alerts." \
      org.opencontainers.image.version="${APP_VERSION}" \
      org.opencontainers.image.source="https://github.com/cagatayuresin/observer-lite" \
      org.opencontainers.image.licenses="MIT"

# iputils-ping for PING monitor type
RUN apt-get update && \
    apt-get install -y --no-install-recommends iputils-ping && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Python packages
COPY --from=backend-deps /install /usr/local

# Backend source
COPY VERSION ./VERSION
COPY backend/app ./app
COPY backend/alembic ./alembic
COPY backend/alembic.ini ./

# Built frontend assets (served as static files)
COPY --from=frontend-builder /build/frontend/dist ./static

# Data directory for SQLite
RUN mkdir -p /data && chmod 755 /data

ENV PORT=3000
ENV DATABASE_PATH=/data/observer.db
ENV DATABASE_URL=sqlite+aiosqlite:////data/observer.db
ENV PYTHONUNBUFFERED=1
ENV PYTHONDONTWRITEBYTECODE=1

VOLUME ["/data"]
EXPOSE 3000

COPY docker-entrypoint.sh ./
RUN chmod +x docker-entrypoint.sh

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD python -c "import os, urllib.request; urllib.request.urlopen(f'http://localhost:{os.environ.get(\"PORT\", \"3000\")}/api/docs')" || exit 1

ENTRYPOINT ["./docker-entrypoint.sh"]
