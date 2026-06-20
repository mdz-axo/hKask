#!/bin/bash
set -e

DATA_DIR="${HKASK_DATA_DIR:-/data}"
DB_PATH="${DATA_DIR}/kask.db"

echo "=== hKask container starting ==="
echo "Data directory: $DATA_DIR"

# Ensure data directory exists
mkdir -p "$DATA_DIR"
mkdir -p /etc/conduit

# Render configuration templates from environment variables
echo "Rendering configuration templates..."
envsubst < /etc/litestream.yml.template > /etc/litestream.yml
envsubst < /etc/conduit/conduit.toml.template > /etc/conduit/conduit.toml

# Restore kask database from object storage if no local copy exists
if [ ! -f "$DB_PATH" ]; then
    echo "No local database found. Attempting restore from Litestream replica..."
    if litestream restore -if-replica-exists -config /etc/litestream.yml "$DB_PATH"; then
        echo "Database restored from object storage."
    else
        echo "No replica found. Starting with fresh database."
    fi
else
    echo "Local database exists. Skipping restore."
fi

# Run database migrations (idempotent)
echo "Running database migrations..."
kask migrate --data-dir "$DATA_DIR" || echo "Warning: migrate command failed (may not exist yet)"

# Start supervisord which manages:
#   - conduit:  Matrix homeserver (federation on :8448)
#   - litestream: WAL replication to object storage
#   - kask: main application (HTTP on :3000)
echo "Starting supervisord..."
exec /usr/bin/supervisord -c /etc/supervisor/supervisord.conf
