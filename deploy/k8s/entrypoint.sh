#!/bin/bash
set -e

DATA_DIR="${HKASK_DATA_DIR:-/data}"
DB_PATH="${DATA_DIR}/kask.db"

echo "=== hKask pod starting ==="
echo "Pod ID: ${POD_ID:-unknown}"
echo "Data directory: $DATA_DIR"

mkdir -p "$DATA_DIR"

# The litestream sidecar container handles database restore and WAL replication.
# If /data/kask.db doesn't exist, the sidecar's `litestream replicate` command
# will restore from S3 before kask starts (litestream checks for existing db
# on startup and restores if missing).
#
# Schema initialization is lazy — UserStore::initialize_schema() runs on
# first access. No explicit migration step needed.

echo "Starting kask serve..."
exec /usr/local/bin/kask serve
