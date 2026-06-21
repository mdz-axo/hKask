#!/bin/bash
set -e

DATA_DIR="${HKASK_DATA_DIR:-/data}"
DB_PATH="${DATA_DIR}/kask.db"

echo "=== hKask pod starting ==="
echo "Pod ID: ${POD_ID:-unknown}"
echo "Data directory: $DATA_DIR"

mkdir -p "$DATA_DIR"

# Render configs from environment variables
envsubst < /etc/litestream.yml.template > /etc/litestream.yml
envsubst < /etc/conduit/conduit.toml.template > /etc/conduit/conduit.toml

# Restore kask database from Litestream if no local copy
if [ ! -f "$DB_PATH" ]; then
    echo "No local database. Attempting restore from Litestream replica..."
    if litestream restore -if-replica-exists -config /etc/litestream.yml "$DB_PATH"; then
        echo "Database restored from object storage."
    else
        echo "No replica found. Starting with fresh database."
    fi
fi

echo "Running database migrations..."
kask migrate --data-dir "$DATA_DIR" || echo "Warning: migrate command failed"

echo "Starting supervisord..."
exec /usr/bin/supervisord -c /etc/supervisor/supervisord.conf
