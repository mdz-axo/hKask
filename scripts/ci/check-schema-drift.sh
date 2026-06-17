#!/usr/bin/env bash
# SCHEMA_SQL drift check — verifies embedded schema in hkask-test-harness
# matches canonical schema in hkask-storage by table/index name.
#
# The vec_embeddings table is intentionally excluded (requires sqlite-vec).
#
# Exit 0 = in sync. Exit 1 = drift detected.

set -euo pipefail

HARNESS="crates/hkask-test-harness/src/schema.rs"
STORAGE="crates/hkask-storage/src/sql/schema.sql"

[ -f "$HARNESS" ] || { echo "ERROR: $HARNESS not found"; exit 1; }
[ -f "$STORAGE" ] || { echo "ERROR: $STORAGE not found"; exit 1; }

# Extract names: CREATE TABLE/INDEX IF NOT EXISTS <name>
extract_names() {
    grep -oE 'CREATE (TABLE|INDEX) IF NOT EXISTS [a-zA-Z_]+' "$1" \
        | awk '{print $NF}' \
        | sort -u
}

harness_tables=$(extract_names "$HARNESS")
storage_tables=$(extract_names "$STORAGE" | grep -v 'vec_embeddings')

if [ "$harness_tables" = "$storage_tables" ]; then
    echo "PASS: Schema table/index names match ($(echo "$harness_tables" | wc -l) total)."
    exit 0
fi

echo "FAIL: Schema drift detected."
echo ""
echo "Extra in harness:"
comm -23 <(echo "$harness_tables") <(echo "$storage_tables") || true
echo ""
echo "Missing from harness:"
comm -13 <(echo "$harness_tables") <(echo "$storage_tables") || true
echo ""
echo "Action: update SCHEMA_SQL in $HARNESS."
exit 1
