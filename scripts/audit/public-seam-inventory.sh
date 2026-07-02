#!/usr/bin/env bash
# Public Seam Inventory Generator
#
# Scans all workspace crates, counts public items and their contract
# annotations, and produces a machine-readable JSON inventory consumed
# by the SeamWatcher (crates/hkask-cns/src/seam_watcher.rs).
#
# Usage: ./scripts/audit/public-seam-inventory.sh

set -euo pipefail

OUTPUT="docs/status/public-seam-inventory.json"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$WORKSPACE_ROOT"

timestamp() { date -u +"%Y-%m-%dT%H:%M:%SZ"; }

count_pub_items() {
    local f="$1" n=0
    set +e; n=$(grep -cE '^\s*pub(\s+\((crate|super|in\s+\w+)\))?\s+(fn|struct|enum|trait|type|const|unsafe\s+fn)\s+' "$f" 2>/dev/null); set -e
    echo "${n:-0}"
}

count_contracts() {
    local f="$1" n=0
    set +e; n=$(grep -c '/// expect:' "$f" 2>/dev/null); set -e
    echo "${n:-0}"
}

count_tests() {
    local f="$1" n=0
    set +e; n=$(grep -c '\[test\]' "$f" 2>/dev/null); set -e
    echo "${n:-0}"
}

echo "==> Public Seam Inventory Generator"
echo "    Workspace: $WORKSPACE_ROOT"
echo ""

# Discover crates
CRATES=$(grep -E '^\s*"[^"]*"' Cargo.toml 2>/dev/null | sed 's/.*"\(.*\)".*/\1/' | sort -u)

TOTAL_ITEMS=0
TOTAL_COVERED=0
TOTAL_TESTS=0
JSON_PARTS=""

for crate_path in $CRATES; do
    if [ ! -d "$crate_path" ] || [ ! -f "$crate_path/Cargo.toml" ]; then
        continue
    fi
    crate_name=$(basename "$crate_path")
    src_dir="$crate_path/src"
    if [ ! -d "$src_dir" ]; then
        # MCP servers sometimes have flat structures
        if [ -f "$crate_path/main.rs" ]; then
            src_dir="$crate_path"
        else
            continue
        fi
    fi

    pub_items=0; contracts=0; tests=0
    while IFS= read -r -d '' f; do
        n=$(count_pub_items "$f"); pub_items=$((pub_items + n))
        n=$(count_contracts "$f"); contracts=$((contracts + n))
        n=$(count_tests "$f"); tests=$((tests + n))
    done < <(find "$src_dir" -name '*.rs' -print0 2>/dev/null)

    # integration tests
    if [ -d "$crate_path/tests" ]; then
        while IFS= read -r -d '' f; do
            n=$(count_tests "$f"); tests=$((tests + n))
        done < <(find "$crate_path/tests" -name '*.rs' -print0 2>/dev/null)
    fi

    uncovered=$((pub_items - contracts))
    cov="0.0"
    if [ "$pub_items" -gt 0 ]; then
        cov=$(echo "scale=1; $contracts * 100.0 / $pub_items" | bc 2>/dev/null || echo "0.0")
    fi

    TOTAL_ITEMS=$((TOTAL_ITEMS + pub_items))
    TOTAL_COVERED=$((TOTAL_COVERED + contracts))
    TOTAL_TESTS=$((TOTAL_TESTS + tests))

    entry="\"$crate_name\": {\"crate_name\":\"$crate_name\",\"total_items\":$pub_items,\"covered_items\":$contracts,\"uncovered_items\":$uncovered,\"coverage_pct\":$cov,\"req_tests\":$tests}"
    if [ -z "$JSON_PARTS" ]; then
        JSON_PARTS="$entry"
    else
        JSON_PARTS="$JSON_PARTS,$entry"
    fi
done

# Also scan MCP servers
for crate_path in $(find mcp-servers -maxdepth 2 -name Cargo.toml -exec dirname {} \; 2>/dev/null | sort); do
    crate_name=$(basename "$crate_path")
    src_dir="$crate_path/src"
    if [ ! -d "$src_dir" ]; then
        if [ -f "$crate_path/main.rs" ]; then
            src_dir="$crate_path"
        else
            continue
        fi
    fi

    pub_items=0; contracts=0; tests=0
    while IFS= read -r -d '' f; do
        n=$(count_pub_items "$f"); pub_items=$((pub_items + n))
        n=$(count_contracts "$f"); contracts=$((contracts + n))
        n=$(count_tests "$f"); tests=$((tests + n))
    done < <(find "$src_dir" -name '*.rs' -print0 2>/dev/null)

    if [ -d "$crate_path/tests" ]; then
        while IFS= read -r -d '' f; do
            n=$(count_tests "$f"); tests=$((tests + n))
        done < <(find "$crate_path/tests" -name '*.rs' -print0 2>/dev/null)
    fi

    uncovered=$((pub_items - contracts))
    cov="0.0"
    if [ "$pub_items" -gt 0 ]; then
        cov=$(echo "scale=1; $contracts * 100.0 / $pub_items" | bc 2>/dev/null || echo "0.0")
    fi

    TOTAL_ITEMS=$((TOTAL_ITEMS + pub_items))
    TOTAL_COVERED=$((TOTAL_COVERED + contracts))
    TOTAL_TESTS=$((TOTAL_TESTS + tests))

    entry="\"$crate_name\": {\"crate_name\":\"$crate_name\",\"total_items\":$pub_items,\"covered_items\":$contracts,\"uncovered_items\":$uncovered,\"coverage_pct\":$cov,\"req_tests\":$tests}"
    if [ -z "$JSON_PARTS" ]; then
        JSON_PARTS="$entry"
    else
        JSON_PARTS="$JSON_PARTS,$entry"
    fi
done

TOTAL_UNCOVERED=$((TOTAL_ITEMS - TOTAL_COVERED))
WS_COV="0.0"
if [ "$TOTAL_ITEMS" -gt 0 ]; then
    WS_COV=$(echo "scale=1; $TOTAL_COVERED * 100.0 / $TOTAL_ITEMS" | bc 2>/dev/null || echo "0.0")
fi

TS=$(timestamp)

cat > "$OUTPUT" << HERE
{
  "generated": "$TS",
  "totals": {
    "crate_name": "workspace",
    "total_items": $TOTAL_ITEMS,
    "covered_items": $TOTAL_COVERED,
    "uncovered_items": $TOTAL_UNCOVERED,
    "coverage_pct": $WS_COV,
    "req_tests": $TOTAL_TESTS
  },
  "crates": {
HERE

printf '%s' "$JSON_PARTS" >> "$OUTPUT"

cat >> "$OUTPUT" << 'HERE'

  }
}
HERE

echo ""
echo "==> Inventory written to $OUTPUT"
echo "    Total public items: $TOTAL_ITEMS"
echo "    Contracted items:   $TOTAL_COVERED"
echo "    Coverage:           ${WS_COV}%"
echo "    Test count:         $TOTAL_TESTS"
echo "    Done — $(timestamp)"
