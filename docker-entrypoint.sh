#!/bin/sh
set -e

# Ensure data directory
mkdir -p "$(dirname "$DATABASE_PATH")"

if [ -z "${SECRET_KEY:-}" ]; then
    echo "[observer-lite] SECRET_KEY is required. Set it to a long random value." >&2
    exit 1
fi

# Run database migrations
echo "[observer-lite] Running migrations…"
alembic upgrade head

# Start server
echo "[observer-lite] Starting on port ${PORT:-3000}…"
exec uvicorn app.main:app \
    --host 0.0.0.0 \
    --port "${PORT:-3000}" \
    --workers 1 \
    --log-level info
