#!/usr/bin/env bash
# Migrate /// REQ: markers to #[contract(id=..., principle=...)] attributes.
# Phase 1 of rSolidity migration (v0.28.0).
#
# Usage:
#   scripts/migrate-req-to-contract.sh [--dry-run] [file...]
#
# Without arguments, processes all .rs files in crates/ and mcp-servers/.

set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" = "--dry-run" ]]; then
    DRY_RUN=true
    shift
fi

FILES=("$@")
if [[ ${#FILES[@]} -eq 0 ]]; then
    mapfile -t FILES < <(find crates/ mcp-servers/ -name '*.rs' -not -path '*/target/*')
fi

TOTAL_MIGRATED=0

for file in "${FILES[@]}"; do
    [[ -f "$file" ]] || continue
    # Skip if no /// REQ: tags
    grep -q "/// REQ:" "$file" 2>/dev/null || continue

    # Process file: extract REQ IDs, delete /// REQ: lines,
    # insert #[contract(...)] before pub fn lines.
    awk '
    /\/\/\/ REQ: / {
        # Extract the full ID: everything after "/// REQ: " until end of line or trailing whitespace
        line = $0
        sub(/^[[:space:]]*\/\/\/ REQ: /, "", line)
        # Remove any trailing whitespace and comments after the ID
        sub(/[[:space:]]*$/, "", line)
        # Extract principle from P{N} prefix
        principle = ""
        if (match(line, /^P[0-9]+/)) {
            principle = substr(line, RSTART, RLENGTH)
        }
        if (principle != "") {
            contract_ids[++contract_count] = line
        }
        # Skip this line (remove /// REQ:)
        next
    }

    /pub (fn |async fn )/ {
        if (contract_count > 0) {
            id = contract_ids[contract_count]
            match(id, /^P[0-9]+/)
            principle = substr(id, RSTART, RLENGTH)
            printf "    #[contract(id = \"%s\", principle = \"%s\")]\n", id, principle
            contract_count--
        }
        print
        next
    }

    { print }
    ' "$file" > "${file}.tmp" && mv "${file}.tmp" "$file"

    # Count how many were migrated in this file
    local_count=$(grep -c '#\[contract(id = ' "$file" 2>/dev/null || true)
    local_new_count=$(grep -c '#\[contract(id = ' "$file" 2>/dev/null || true)
    TOTAL_MIGRATED=$((TOTAL_MIGRATED + local_new_count))
done

echo "Migrated $TOTAL_MIGRATED /// REQ: tags to #[contract(...)] attributes."
echo "Files still needing manual review:"
echo "  - Verify each #[contract] appears directly before its pub fn"
echo "  - Add 'use hkask_rsolidity as rs;' where #[contract] appears"
echo "  - Run: cargo build --workspace to verify"