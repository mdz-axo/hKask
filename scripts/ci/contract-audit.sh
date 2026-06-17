#!/usr/bin/env bash
# Contract coverage audit — lists uncontracted pub fn for replicant-driven
# contract proposals (Phase B1 of contract-first migration plan).
#
# Usage:
#   scripts/contract-audit.sh              # audit all crates
#   scripts/contract-audit.sh hkask-cns   # audit a specific crate
#   scripts/contract-audit.sh --json      # JSON output for MCP tool wrapping
#   scripts/contract-audit.sh --summary   # summary only (counts per crate)
#
# Exit 0 always (trend monitor, not a hard gate — baseline is 0/1,727).
#
# Reference: docs/plans/contract-first-migration-plan-v0.27.0.md §5.1

set -euo pipefail

MODE="detailed"
TARGET=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --json)    MODE="json"; shift ;;
        --summary) MODE="summary"; shift ;;
        --csv)     MODE="csv"; shift ;;
        *)         TARGET="$1"; shift ;;
    esac
done

# ── Helpers ──────────────────────────────────────────────────────────────────

count_pub_fns() {
    local dir="$1"
    local count
    count=$(grep -rn -e "pub fn " -e "pub async fn " "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | wc -l)
    # strip leading whitespace from wc -l output
    count=$(echo "$count" | tr -d ' ')
    echo "${count:-0}"
}

count_contracted() {
    local dir="$1"
    local count
    # Count doc-comment contracts (/// REQ:) and line-comment REQ tags (// REQ:)
    # Each contract starts with a REQ: line; pre:/post: follow on subsequent lines.
    # Exclude test code and tests/ directories.
    count=$(grep -rn "///* REQ:" "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | wc -l)
    count=$(echo "$count" | tr -d ' ')
    echo "${count:-0}"
}

list_uncontracted() {
    local dir="$1"
    # Get all pub fn lines, filter out test-only, then filter out those with contracts
    grep -rn -e "pub fn " -e "pub async fn " "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | while IFS=: read -r file line rest; do
            # Check if this file+line has a /// REQ: contract within 5 lines above
            contracted=false
            for offset in 0 1 2 3 4 5; do
                check_line=$((line - offset))
                if [ "$check_line" -gt 0 ]; then
                    if sed -n "${check_line}p" "$file" 2>/dev/null | grep -q "///* REQ:"; then
                        contracted=true
                        break
                    fi
                fi
            done
            if [ "$contracted" = false ]; then
                echo "$file:$line:$rest"
            fi
        done || true
}

# ── Main ─────────────────────────────────────────────────────────────────────

if [ "$MODE" = "json" ]; then
    echo "{"
    echo "  \"baseline\": {"
    echo "    \"total_pub_fns\": $(count_pub_fns "crates/"),"
    echo "    \"total_contracted\": $(count_contracted "crates/")"
    echo "  },"
    echo "  \"crates\": ["
    first=true
    for crate_dir in crates/*/; do
        crate=$(basename "$crate_dir")
        src="${crate_dir}src"
        [ -d "$src" ] || continue
        pub_count=$(count_pub_fns "$src")
        contracted_count=$(count_contracted "$src")
        coverage_pct="0.0"
        if [ "$pub_count" -gt 0 ]; then
            coverage_pct=$(echo "scale=1; $contracted_count * 100 / $pub_count" | bc 2>/dev/null || echo "0.0")
        fi
        if [ "$first" = true ]; then first=false; else echo ","; fi
        echo "    {"
        echo "      \"crate\": \"$crate\","
        echo "      \"pub_fns\": $pub_count,"
        echo "      \"contracted\": $contracted_count,"
        echo "      \"coverage_pct\": $coverage_pct"
        echo -n "    }"
    done
    echo ""
    echo "  ]"
    echo "}"
    exit 0
fi

if [ "$MODE" = "csv" ]; then
    echo "crate,pub_fns,contracted,coverage_pct"
    for crate_dir in crates/*/; do
        crate=$(basename "$crate_dir")
        src="${crate_dir}src"
        [ -d "$src" ] || continue
        pub_count=$(count_pub_fns "$src")
        contracted_count=$(count_contracted "$src")
        coverage_pct="0.0"
        if [ "$pub_count" -gt 0 ]; then
            coverage_pct=$(echo "scale=1; $contracted_count * 100 / $pub_count" | bc 2>/dev/null || echo "0.0")
        fi
        echo "$crate,$pub_count,$contracted_count,$coverage_pct"
    done
    exit 0
fi

if [ "$MODE" = "summary" ]; then
    echo "=== Contract Coverage Summary ==="
    echo ""
    printf "%-30s %8s %10s %10s\n" "Crate" "Pub Fns" "Contracted" "Coverage %"
    printf "%-30s %8s %10s %10s\n" "------------------------------" "--------" "----------" "----------"
    total_pub=0
    total_con=0
    for crate_dir in crates/*/; do
        crate=$(basename "$crate_dir")
        src="${crate_dir}src"
        [ -d "$src" ] || continue
        pub_count=$(count_pub_fns "$src")
        contracted_count=$(count_contracted "$src")
        coverage_pct="0.0"
        if [ "$pub_count" -gt 0 ]; then
            coverage_pct=$(echo "scale=1; $contracted_count * 100 / $pub_count" | bc 2>/dev/null || echo "0.0")
        fi
        total_pub=$((total_pub + pub_count))
        total_con=$((total_con + contracted_count))
        printf "%-30s %8d %10d %9s%%\n" "$crate" "$pub_count" "$contracted_count" "$coverage_pct"
    done
    echo ""
    total_cov="0.0"
    if [ "$total_pub" -gt 0 ]; then
        total_cov=$(echo "scale=1; $total_con * 100 / $total_pub" | bc 2>/dev/null || echo "0.0")
    fi
    printf "%-30s %8d %10d %9s%%\n" "TOTAL" "$total_pub" "$total_con" "$total_cov"
    echo ""
    echo "PASS: Contract coverage audit complete (trend monitor, not a hard gate)."
    exit 0
fi

# ── Detailed mode (default) ──────────────────────────────────────────────────

if [ -n "$TARGET" ]; then
    # Single crate mode
    src="crates/${TARGET}/src"
    if [ ! -d "$src" ]; then
        echo "ERROR: crate '${TARGET}' not found at ${src}"
        exit 1
    fi
    pub_count=$(count_pub_fns "$src")
    contracted_count=$(count_contracted "$src")
    coverage_pct="0.0"
    if [ "$pub_count" -gt 0 ]; then
        coverage_pct=$(echo "scale=1; $contracted_count * 100 / $pub_count" | bc 2>/dev/null || echo "0.0")
    fi

    echo "=== Contract Audit: ${TARGET} ==="
    echo ""
    echo "Public functions: $pub_count"
    echo "Contracted:       $contracted_count"
    echo "Coverage:         ${coverage_pct}%"
    echo ""

    if [ "$pub_count" -gt 0 ] && [ "$contracted_count" -lt "$pub_count" ]; then
        echo "── Uncontracted public functions ──"
        echo ""
        list_uncontracted "$src" | while IFS=: read -r file line sig; do
            printf "  %-50s L%-4d %s\n" "$file" "$line" "$sig"
        done
        echo ""
    fi

    uncontracted_count=$((pub_count - contracted_count))
    echo "Uncontracted: $uncontracted_count — candidates for replicant contract proposals."
else
    # All crates mode
    echo "=== Contract Coverage Audit — All Crates ==="
    echo ""
    total_pub=0
    total_con=0
    for crate_dir in crates/*/; do
        crate=$(basename "$crate_dir")
        src="${crate_dir}src"
        [ -d "$src" ] || continue
        pub_count=$(count_pub_fns "$src")
        contracted_count=$(count_contracted "$src")
        total_pub=$((total_pub + pub_count))
        total_con=$((total_con + contracted_count))

        if [ "$pub_count" -gt 0 ] && [ "$contracted_count" -eq 0 ]; then
            echo "  🔴 $crate: $pub_count pub fns, 0 contracted"
        elif [ "$pub_count" -gt 0 ]; then
            coverage_pct=$(echo "scale=1; $contracted_count * 100 / $pub_count" | bc 2>/dev/null || echo "0.0")
            echo "  🟡 $crate: $pub_count pub fns, $contracted_count contracted (${coverage_pct}%)"
        else
            echo "  ⚪ $crate: no pub fns"
        fi
    done
    echo ""
    total_cov="0.0"
    if [ "$total_pub" -gt 0 ]; then
        total_cov=$(echo "scale=1; $total_con * 100 / $total_pub" | bc 2>/dev/null || echo "0.0")
    fi
    echo "Total: $total_pub pub fns, $total_con contracted (${total_cov}%)"
    echo ""
    echo "PASS: Contract coverage audit complete (trend monitor, not a hard gate)."
fi

exit 0
