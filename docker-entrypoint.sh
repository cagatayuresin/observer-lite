#!/bin/sh
set -e

# Ensure data directory
mkdir -p "$(dirname "$DATABASE_PATH")"

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
