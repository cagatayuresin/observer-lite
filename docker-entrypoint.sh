#!/bin/sh
set -e

if [ -z "${SECRET_KEY:-}" ]; then
    echo "[observer-lite] SECRET_KEY is required. Set it to a long random value." >&2
    exit 1
fi

data_dir=$(dirname "${DATABASE_PATH:-/data/observer.db}")
mkdir -p "$data_dir"

echo "[observer-lite] Starting on port ${PORT:-3000}"
if [ "$(id -u)" = "0" ]; then
    chown observer:observer "$data_dir"
    exec su-exec observer /usr/local/bin/observer-lite
fi

exec /usr/local/bin/observer-lite
