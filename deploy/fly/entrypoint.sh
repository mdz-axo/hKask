#!/bin/bash
set -e

DATA_DIR="${HKASK_DATA_DIR:-/data}"
DB_PATH="${DATA_DIR}/kask.db"

echo "=== hKask container starting ==="
echo "Data directory: $DATA_DIR"

# Ensure data directory exists
mkdir -p "$DATA_DIR"

# Render Litestream configuration from environment variables
echo "Rendering Litestream configuration..."
envsubst < /etc/litestream.yml.template > /etc/litestream.yml

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

# Start Litestream replication with kask as child process.
# Litestream monitors the WAL and streams changed pages to object storage.
# If kask exits, Litestream flushes remaining WAL and exits.
echo "Starting kask with Litestream replication..."
exec litestream replicate -config /etc/litestream.yml -exec "kask serve --data-dir $DATA_DIR"
