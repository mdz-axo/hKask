#!/usr/bin/env bash
# Public Seam Inventory Generator
#
# Scans all workspace crates, counts public items and their contract annotations,
# and produces a machine-readable JSON inventory consumed by the SeamWatcher
# (crates/hkask-cns/src/seam_watcher.rs) and embedded at compile time.
#
# Usage:
#   ./scripts/audit/public-seam-inventory.sh [--output <path>]
#
# Default output: docs/status/public-seam-inventory.json

set -euo pipefail

OUTPUT="${2:-docs/status/public-seam-inventory.json}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$WORKSPACE_ROOT"

timestamp() {
    date -u +"%Y-%m-%dT%H:%M:%SZ"
}

count_pub_items() {
    local file="$1"
    local n=0
    set +e
    n=$(grep -cE '^\s*pub(\s+\((crate|super|in\s+\w+)\))?\s+(fn|struct|enum|trait|type|const|unsafe\s+fn)\s+' "$file" 2>/dev/null)
    set -e
    if [ -z "$n" ]; then n=0; fi
    echo "$n"
}

count_contracts() {
    local file="$1"
    local n=0
    set +e
    n=$(grep -c '/// expect:' "$file" 2>/dev/null)
    set -e
    if [ -z "$n" ]; then n=0; fi
    echo "$n"
}

count_tests() {
    local file="$1"
    local n=0
    set +e
    n=$(grep -c '#\[test\]\|#\[tokio::test\]' "$file" 2>/dev/null)
    set -e
    if [ -z "$n" ]; then n=0; fi
    echo "$n"
}

echo "==> Public Seam Inventory Generator"
echo "    Workspace: $WORKSPACE_ROOT"
echo "    Output:    $OUTPUT"
echo ""

# Discover crates
set +e
CRATES_LINES=$(grep -E '^\s*"[^"]*"' Cargo.toml 2>/dev/null)
set -e

declare -a CRATES
while IFS= read -r line; do
    name=$(echo "$line" | sed 's/.*"\(.*\)".*/\1/')
    if [ -n "$name" ] && [ -d "$name" ] && [ -f "$name/Cargo.toml" ]; then
        CRATES+=("$name")
    fi
done <<< "$CRATES_LINES"

# MCP servers
declare -a MCP_CRATES
while IFS= read -r dir; do
    MCP_CRATES+=("$dir")
done < <(find mcp-servers -maxdepth 2 -name Cargo.toml -exec dirname {} \; 2>/dev/null | sort)

TOTAL_ITEMS=0
TOTAL_COVERED=0
TOTAL_REQ_TESTS=0
declare -a JSON_ENTRIES

for crate_path in "${CRATES[@]}" "${MCP_CRATES[@]}"; do
    crate_name=$(basename "$crate_path")
    src_dir="$crate_path/src"
    if [ ! -d "$src_dir" ]; then
        continue
    fi

    pub_items=0
    contracts=0
    tests=0

    while IFS= read -r -d '' file; do
        set +e
        n=$(count_pub_items "$file"); pub_items=$((pub_items + n))
        n=$(count_contracts "$file"); contracts=$((contracts + n))
        n=$(count_tests "$file"); tests=$((tests + n))
        set -e
    done < <(find "$src_dir" -name '*.rs' -print0 2>/dev/null)

    # Integration tests
    tests_dir="$crate_path/tests"
    if [ -d "$tests_dir" ]; then
        while IFS= read -r -d '' file; do
            set +e
            n=$(count_tests "$file"); tests=$((tests + n))
            set -e
        done < <(find "$tests_dir" -name '*.rs' -print0 2>/dev/null)
    fi

    uncovered=$((pub_items - contracts))
    if [ "$pub_items" -gt 0 ]; then
        coverage_pct=$(echo "scale=1; $contracts * 100.0 / $pub_items" | bc 2>/dev/null || echo "0.0")
    else
        coverage_pct="0.0"
    fi

    TOTAL_ITEMS=$((TOTAL_ITEMS + pub_items))
    TOTAL_COVERED=$((TOTAL_COVERED + contracts))
    TOTAL_REQ_TESTS=$((TOTAL_REQ_TESTS + tests))

    json=$(printf '    "%s": {\n      "crate_name": "%s",\n      "total_items": %d,\n      "covered_items": %d,\n      "uncovered_items": %d,\n      "coverage_pct": %s,\n      "req_tests": %d\n    }' \
        "$crate_name" "$crate_name" "$pub_items" "$contracts" "$uncovered" "$coverage_pct" "$tests")
    JSON_ENTRIES+=("$json")
done

TOTAL_UNCOVERED=$((TOTAL_ITEMS - TOTAL_COVERED))
if [ "$TOTAL_ITEMS" -gt 0 ]; then
    WORKSPACE_COVERAGE=$(echo "scale=1; $TOTAL_COVERED * 100.0 / $TOTAL_ITEMS" | bc 2>/dev/null || echo "0.0")
else
    WORKSPACE_COVERAGE="0.0"
fi

# Emit JSON
{
    echo '{'
    echo '  "generated": "'"$(timestamp)"'",'
    echo '  "totals": {'
    echo '    "crate_name": "workspace",'
    echo '    "total_items": '"$TOTAL_ITEMS"','
    echo '    "covered_items": '"$TOTAL_COVERED"','
    echo '    "uncovered_items": '"$TOTAL_UNCOVERED"','
    echo '    "coverage_pct": '"$WORKSPACE_COVERAGE"','
    echo '    "req_tests": '"$TOTAL_REQ_TESTS"'
    echo '  },'
    echo '  "crates": {'

    first=true
    for entry in "${JSON_ENTRIES[@]}"; do
        if [ "$first" = true ]; then
            first=false
        else
            echo ','
        fi
        printf '%s' "$entry"
    done

    echo ''
    echo '  }'
    echo '}'
} > "$OUTPUT"

echo ""
echo "==> Inventory written to $OUTPUT"
echo "    Total public items: $TOTAL_ITEMS"
echo "    Contracted items:   $TOTAL_COVERED"
echo "    Coverage:           ${WORKSPACE_COVERAGE}%"
echo "    Crate count:        ${#JSON_ENTRIES[@]}"
echo "    Test count:         $TOTAL_REQ_TESTS"
echo "    Done — $(timestamp)"
