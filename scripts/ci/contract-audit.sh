#!/usr/bin/env bash
# Contract coverage audit — lists uncontracted pub fn for replicant-driven
# contract proposals (Phase B1 of contract-first migration plan).
#
# Usage:
#   scripts/contract-audit.sh                  # audit all crates
#   scripts/contract-audit.sh hkask-cns       # audit a specific crate
#   scripts/contract-audit.sh --json          # JSON output for MCP tool wrapping
#   scripts/contract-audit.sh --summary       # summary only (counts per crate)
#   scripts/contract-audit.sh --expect        # audit expect: field presence (v0.28.0)
#   scripts/contract-audit.sh --principles    # audit goal-principle [P{N}] anchoring
#   scripts/contract-audit.sh --constraining  # audit constraining-principle annotations
#   scripts/contract-audit.sh --contract-quality # aggregate 4-layer quality score
#   scripts/contract-audit.sh --full          # run all modes (coverage + expect + principles + constraining + quality)
#
# Exit 0 always (trend monitor, not a hard gate — baseline is 0/1,727).
#
# Reference: docs/plans/contract-first-migration-plan-v0.27.0.md §5.1
# Extended v0.28.0 checks: docs/architecture/core/TESTING_DISCIPLINE.md §9.1

set -euo pipefail

MODE="detailed"
TARGET=""
FULL_MODE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --json)         MODE="json"; shift ;;
        --summary)      MODE="summary"; shift ;;
        --csv)          MODE="csv"; shift ;;
        --expect)       MODE="expect"; shift ;;
        --principles)   MODE="principles"; shift ;;
        --constraining) MODE="constraining"; shift ;;
        --contract-quality) MODE="contract-quality"; shift ;;
        --full)         FULL_MODE=true; shift ;;
        *)              TARGET="$1"; shift ;;
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

# ── v0.28.0 Extended Helpers ─────────────────────────────────────────────────

# Count contracts with expect: field
count_expect() {
    local dir="$1"
    local count
    count=$(grep -rn "///* REQ:.*expect:" "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | wc -l)
    count=$(echo "$count" | tr -d ' ')
    echo "${count:-0}"
}

# Count contracts with [P{N}] goal-principle tag on expect: lines
count_goal_principle() {
    local dir="$1"
    local count
    count=$(grep -rn "///* REQ:.*expect:.*\[P[0-9]*\]" "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | wc -l)
    count=$(echo "$count" | tr -d ' ')
    echo "${count:-0}"
}

# Count constraining-principle annotations ([P{N}] Constraining: lines)
count_constraining() {
    local dir="$1"
    local count
    count=$(grep -rn "///* \[P[0-9]*\].*Constraining:" "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | wc -l)
    count=$(echo "$count" | tr -d ' ')
    echo "${count:-0}"
}

# List contracts missing expect: field
list_missing_expect() {
    local dir="$1"
    # Get all contracted pub fns, then check for expect: within 10 lines above
    grep -rn -e "pub fn " -e "pub async fn " "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | while IFS=: read -r file line rest; do
            has_contract=false
            has_expect=false
            for offset in 0 1 2 3 4 5 6 7 8 9 10; do
                check_line=$((line - offset))
                if [ "$check_line" -gt 0 ]; then
                    local_ctx=$(sed -n "${check_line}p" "$file" 2>/dev/null || true)
                    if echo "$local_ctx" | grep -q "///* REQ:"; then
                        has_contract=true
                    fi
                    if echo "$local_ctx" | grep -q "expect:"; then
                        has_expect=true
                    fi
                fi
            done
            if [ "$has_contract" = true ] && [ "$has_expect" = false ]; then
                echo "MISSING_EXPECTATION:$file:$line:$rest"
            fi
        done || true
}

# List contracts with invalid or missing [P{N}] goal-principle tags
list_principle_gaps() {
    local dir="$1"
    grep -rn -e "pub fn " -e "pub async fn " "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | while IFS=: read -r file line rest; do
            has_contract=false
            has_principle=false
            for offset in 0 1 2 3 4 5 6 7 8 9 10; do
                check_line=$((line - offset))
                if [ "$check_line" -gt 0 ]; then
                    local_ctx=$(sed -n "${check_line}p" "$file" 2>/dev/null || true)
                    if echo "$local_ctx" | grep -q "///* REQ:"; then
                        has_contract=true
                    fi
                    if echo "$local_ctx" | grep -qE "\[P[0-9]+\]"; then
                        has_principle=true
                    fi
                fi
            done
            if [ "$has_contract" = true ] && [ "$has_principle" = false ]; then
                echo "MISSING_GOAL_PRINCIPLE:$file:$line:$rest"
            fi
        done || true
    # Check for invalid principle numbers (not P1-P12) in contracts
    grep -rnE "///* REQ:.*\[P([0]|[1][3-9]|[2-9][0-9])\]" "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | while IFS=: read -r file line rest; do
            echo "INVALID_PRINCIPLE:$file:$line:$rest"
        done || true
}

# List contracts with zero constraining principles
list_unconstrained() {
    local dir="$1"
    grep -rn -e "pub fn " -e "pub async fn " "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | while IFS=: read -r file line rest; do
            has_contract=false
            has_constraining=false
            for offset in 0 1 2 3 4 5 6 7 8 9 10; do
                check_line=$((line - offset))
                if [ "$check_line" -gt 0 ]; then
                    local_ctx=$(sed -n "${check_line}p" "$file" 2>/dev/null || true)
                    if echo "$local_ctx" | grep -q "///* REQ:"; then
                        has_contract=true
                    fi
                    if echo "$local_ctx" | grep -qE "Constraining:"; then
                        has_constraining=true
                    fi
                fi
            done
            if [ "$has_contract" = true ] && [ "$has_constraining" = false ]; then
                echo "UNCONSTRAINED:$file:$line:$rest"
            fi
        done || true
}

# Check for missing Magna Carta constraints (P1-P4) in contracts dealing with user data/consent/capabilities
list_missing_magna_carta() {
    local dir="$1"
    grep -rn -e "pub fn " -e "pub async fn " "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | while IFS=: read -r file line rest; do
            has_contract=false
            has_p1=false; has_p2=false; has_p4=false
            for offset in 0 1 2 3 4 5 6 7 8 9 10; do
                check_line=$((line - offset))
                if [ "$check_line" -gt 0 ]; then
                    local_ctx=$(sed -n "${check_line}p" "$file" 2>/dev/null || true)
                    if echo "$local_ctx" | grep -q "///* REQ:"; then
                        has_contract=true
                    fi
                    if echo "$local_ctx" | grep -qE "\[P1\].*Constraining:"; then
                        has_p1=true
                    fi
                    if echo "$local_ctx" | grep -qE "\[P2\].*Constraining:"; then
                        has_p2=true
                    fi
                    if echo "$local_ctx" | grep -qE "\[P4\].*Constraining:"; then
                        has_p4=true
                    fi
                fi
            done
            if [ "$has_contract" = true ]; then
                missing=""
                [ "$has_p1" = false ] && missing="$missing P1"
                [ "$has_p2" = false ] && missing="$missing P2"
                [ "$has_p4" = false ] && missing="$missing P4"
                if [ -n "$missing" ]; then
                    echo "MISSING_MAGNA_CARTA_CONSTRAINT:$file:$line:$rest (missing:$missing)"
                fi
            fi
        done || true
}

# ── Main ─────────────────────────────────────────────────────────────────────

# Route to appropriate handler
if [ "$FULL_MODE" = true ]; then
    run_full_mode "${TARGET:-ALL}"
    exit 0
fi

case "$MODE" in
    expect)
        TARGET="${TARGET:-ALL}"
        if [ "$TARGET" = "ALL" ]; then
            for crate_dir in crates/*/; do
                c=$(basename "$crate_dir")
                [ -d "crates/${c}/src" ] || continue
                run_expect_mode "$c"
            done
        else
            run_expect_mode "$TARGET"
        fi
        exit 0
        ;;
    principles)
        TARGET="${TARGET:-ALL}"
        if [ "$TARGET" = "ALL" ]; then
            for crate_dir in crates/*/; do
                c=$(basename "$crate_dir")
                [ -d "crates/${c}/src" ] || continue
                run_principles_mode "$c"
            done
        else
            run_principles_mode "$TARGET"
        fi
        exit 0
        ;;
    constraining)
        TARGET="${TARGET:-ALL}"
        if [ "$TARGET" = "ALL" ]; then
            for crate_dir in crates/*/; do
                c=$(basename "$crate_dir")
                [ -d "crates/${c}/src" ] || continue
                run_constraining_mode "$c"
            done
        else
            run_constraining_mode "$TARGET"
        fi
        exit 0
        ;;
    contract-quality)
        TARGET="${TARGET:-ALL}"
        if [ "$TARGET" = "ALL" ]; then
            for crate_dir in crates/*/; do
                c=$(basename "$crate_dir")
                [ -d "crates/${c}/src" ] || continue
                run_contract_quality_mode "$c"
            done
        else
            run_contract_quality_mode "$TARGET"
        fi
        exit 0
        ;;
esac

# ── Coverage mode (existing detailed/summary/json/csv) ───────────────────────

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

# ── Detailed mode (default) and extended mode definitions ────────────────────

run_expect_mode() {
    crate="$1"
    src="crates/${crate}/src"
    [ -d "$src" ] || return

    pub_count=$(count_pub_fns "$src")
    contracted_count=$(count_contracted "$src")
    expect_count=$(count_expect "$src")

    echo "=== Expect Audit: ${crate} ==="
    echo ""
    echo "Public functions:   $pub_count"
    echo "Contracted:         $contracted_count"
    echo "With expect: field: $expect_count"
    echo "Missing expect:     $((contracted_count - expect_count))"
    echo ""

    missing=$(list_missing_expect "$src")
    if [ -n "$missing" ]; then
        echo "── Contracts Missing expect: Field ──"
        echo "$missing" | while IFS=: read -r tag file line sig; do
            printf "  %-50s L%-4d %s\n" "$file" "$line" "$sig"
        done
        echo ""
    fi

    expectation_completeness_pct="0.0"
    if [ "$contracted_count" -gt 0 ]; then
        expectation_completeness_pct=$(echo "scale=1; $expect_count * 100 / $contracted_count" | bc 2>/dev/null || echo "0.0")
    fi
    echo "Expectation completeness: ${expectation_completeness_pct}%"
    echo ""
}

run_principles_mode() {
    crate="$1"
    src="crates/${crate}/src"
    [ -d "$src" ] || return

    contracted_count=$(count_contracted "$src")
    goal_count=$(count_goal_principle "$src")

    echo "=== Principle Audit: ${crate} ==="
    echo ""
    echo "Contracted:              $contracted_count"
    echo "With [P{N}] goal-principle: $goal_count"
    echo "Missing goal-principle:   $((contracted_count - goal_count))"
    echo ""

    principle_gaps=$(list_principle_gaps "$src")
    if [ -n "$principle_gaps" ]; then
        echo "── Principle Gaps ──"
        echo "$principle_gaps" | while IFS=: read -r tag file line rest; do
            printf "  %-20s %-50s L%-4d %s\n" "$tag" "$file" "$line" "$rest"
        done
        echo ""
    fi
}

run_constraining_mode() {
    crate="$1"
    src="crates/${crate}/src"
    [ -d "$src" ] || return

    contracted_count=$(count_contracted "$src")
    constraining_count=$(count_constraining "$src")

    echo "=== Constraining Principle Audit: ${crate} ==="
    echo ""
    echo "Contracted:                   $contracted_count"
    echo "Constraining annotations:      $constraining_count"
    echo ""

    unconstrained=$(list_unconstrained "$src")
    if [ -n "$unconstrained" ]; then
        unconstrained_count=$(echo "$unconstrained" | wc -l)
        echo "── Unconstrained Contracts ──"
        echo "$unconstrained" | while IFS=: read -r tag file line sig; do
            printf "  %-50s L%-4d %s\n" "$file" "$line" "$sig"
        done
        echo ""
        echo "Unconstrained: $unconstrained_count contracts with zero [P{N}] Constraining annotations"
        echo ""
    fi

    magna_carta_gaps=$(list_missing_magna_carta "$src")
    if [ -n "$magna_carta_gaps" ]; then
        magna_count=$(echo "$magna_carta_gaps" | wc -l)
        echo "── Missing Magna Carta (P1-P4) Constraints ──"
        echo "$magna_carta_gaps" | while IFS=: read -r tag file line rest; do
            printf "  %-50s L%-4d %s\n" "$file" "$line" "$rest"
        done
        echo ""
        echo "Magna Carta gaps: $magna_count contracts missing one or more P1-P4 constraints"
        echo ""
    fi
}

# ── Full mode (all checks) ───────────────────────────────────────────────────

run_full_mode() {
    crate="$1"
    [ -z "$crate" ] && crate="ALL"
    echo "=== Full Contract Audit (v0.28.0): ${crate} ==="
    echo ""

    if [ "$crate" = "ALL" ]; then
        for crate_dir in crates/*/; do
            c=$(basename "$crate_dir")
            [ -d "crates/${c}/src" ] || continue
            run_expect_mode "$c"
            run_principles_mode "$c"
            run_constraining_mode "$c"
        done
        # Also run coverage summary
        echo "=== Contract Coverage Summary ==="
        echo ""
        printf "%-30s %8s %10s %10s\n" "Crate" "Pub Fns" "Contracted" "Coverage %"
        printf "%-30s %8s %10s %10s\n" "------------------------------" "--------" "----------" "----------"
        total_pub=0
        total_con=0
        for crate_dir in crates/*/; do
            c=$(basename "$crate_dir")
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
            printf "%-30s %8d %10d %9s%%\n" "$c" "$pub_count" "$contracted_count" "$coverage_pct"
        done
        echo ""
        total_cov="0.0"
        if [ "$total_pub" -gt 0 ]; then
            total_cov=$(echo "scale=1; $total_con * 100 / $total_pub" | bc 2>/dev/null || echo "0.0")
        fi
        printf "%-30s %8d %10d %9s%%\n" "TOTAL" "$total_pub" "$total_con" "$total_cov"
        echo ""
        return
    fi

    run_expect_mode "$crate"
    run_principles_mode "$crate"
    run_constraining_mode "$crate"

    # Detailed coverage for single crate
    src="crates/${crate}/src"
    pub_count=$(count_pub_fns "$src")
    contracted_count=$(count_contracted "$src")
    coverage_pct="0.0"
    if [ "$pub_count" -gt 0 ]; then
        coverage_pct=$(echo "scale=1; $contracted_count * 100 / $pub_count" | bc 2>/dev/null || echo "0.0")
    fi

    echo "=== Contract Audit: ${crate} ==="
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
}

# ── Detailed mode (default coverage audit) ───────────────────────────────────

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
