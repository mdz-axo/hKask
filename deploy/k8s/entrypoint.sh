#!/bin/bash
set -e

DATA_DIR="${HKASK_DATA_DIR:-/data}"
DB_PATH="${DATA_DIR}/kask.db"

echo "=== hKask pod starting ==="
echo "Pod ID: ${POD_ID:-unknown}"
echo "Data directory: $DATA_DIR"

mkdir -p "$DATA_DIR"

# Restore kask database from Litestream if no local copy
if [ ! -f "$DB_PATH" ]; then
    echo "No local database. Attempting restore from Litestream replica..."
    if litestream restore -if-replica-exists -config /etc/litestream.yml "$DB_PATH"; then
        echo "Database restored from object storage."
    else
        echo "No replica found. Starting with fresh database."
    fi
fi

# Schema initialization is lazy — UserStore::initialize_schema() runs on first access.
# No explicit migration step needed.

echo "Starting kask serve..."
exec /usr/local/bin/kask serve
