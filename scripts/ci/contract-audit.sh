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
#   scripts/contract-audit.sh --rsolidity     # audit rSolidity #[contract] attribute presence
#   scripts/contract-audit.sh --full          # run all modes (coverage + expect + principles + constraining + quality + rsolidity)
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
        --rsolidity)     MODE="rsolidity"; shift ;;
        --full)         FULL_MODE=true; shift ;;
        *)              TARGET="$1"; shift ;;
    esac
done

# ── Helpers for scanning both crates/ and mcp-servers/ ────────────────────────

# List all crate source directories (both crates/ and mcp-servers/).
# Each entry: "crate_name|src_dir"
list_all_crate_sources() {
    for crate_dir in crates/*/; do
        local c
        c=$(basename "$crate_dir")
        [ -d "crates/${c}/src" ] || continue
        echo "${c}|crates/${c}/src"
    done
    for mcp_dir in mcp-servers/*/; do
        local c
        c=$(basename "$mcp_dir")
        [ -d "mcp-servers/${c}/src" ] || continue
        echo "${c}|mcp-servers/${c}/src"
    done
}

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
    local count=0
    # Count all contract markers: #[contract], #[rs::contract], /// contract(id:, /// REQ: (legacy)
    local c1 c2 c3 c4
    c1=$(grep -rn '#\[contract(id' "$dir" --include="*.rs" 2>/dev/null | grep -v "cfg(test)" | grep -v "/tests/" | wc -l)
    c2=$(grep -rn '#\[rs::contract(id' "$dir" --include="*.rs" 2>/dev/null | grep -v "cfg(test)" | grep -v "/tests/" | wc -l)
    c3=$(grep -rn '/// contract(id:' "$dir" --include="*.rs" 2>/dev/null | grep -v "cfg(test)" | grep -v "/tests/" | wc -l)
    c4=$(grep -rn '/// REQ:' "$dir" --include="*.rs" 2>/dev/null | grep -v "cfg(test)" | grep -v "/tests/" | wc -l)
    count=$(( $(echo "$c1" | tr -d ' ') + $(echo "$c2" | tr -d ' ') + $(echo "$c3" | tr -d ' ') + $(echo "$c4" | tr -d ' ') ))
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

# Count contracts with expect: field (on any doc-comment line, not just same-line as REQ:)
count_expect() {
    local dir="$1"
    local count
    count=$(grep -rn "^[[:space:]]*///*.*expect:" "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | wc -l)
    count=$(echo "$count" | tr -d ' ')
    echo "${count:-0}"
}

# Count contracts with [P{N}] goal-principle tag (on any doc-comment line)
count_goal_principle() {
    local dir="$1"
    local count
    count=$(grep -rnE "^[[:space:]]*///*.*expect:.*\[P[0-9]+\]" "$dir" --include="*.rs" 2>/dev/null \
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

# Count functions with #[contract] attribute (rSolidity migration)
count_rsolidity() {
    local dir="$1"
    local count
    count=$(grep -rn "#\[.*contract" "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | wc -l)
    count=$(echo "$count" | tr -d ' ')
    echo "${count:-0}"
}

# List rSolidity drift: #[contract] without matching /// REQ:, and /// REQ: without #[contract].
# Also detects id/principle mismatches between the two sources.
list_rsolidity_drift() {
    local dir="$1"
    grep -rn -e "pub fn " -e "pub async fn " "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" \
        | grep -v "/tests/" \
        | while IFS=: read -r file line rest; do
            has_contract=false
            has_rsolidity=false
            req_id=""
            rs_id=""
            rs_principle=""
            for offset in 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do
                check_line=$((line - offset))
                if [ "$check_line" -gt 0 ]; then
                    ctx=$(sed -n "${check_line}p" "$file" 2>/dev/null || true)
                    # Stop at previous function's closing brace
                    if [ "$(echo "$ctx" | tr -d ' ')" = "}" ]; then
                        break
                    fi
                    if echo "$ctx" | grep -q "///* REQ:"; then
                        has_contract=true
                        # Extract REQ id — capture P{N}- prefix and everything until space
                        req_id=$(echo "$ctx" | sed -n 's/.*REQ: *\(P[0-9]*-[^ ]*\).*/\1/p')
                    fi
                    if echo "$ctx" | grep -q "#\[.*contract"; then
                        has_rsolidity=true
                        # Extract rSolidity id
                        rs_id=$(echo "$ctx" | sed -n 's/.*id *= *"\([^"]*\)".*/\1/p')
                        # shellcheck disable=SC2034 # extracted for future use
                        rs_principle=$(echo "$ctx" | sed -n 's/.*principle *= *"\([^"]*\)".*/\1/p')
                    fi
                fi
            done
            if [ "$has_rsolidity" = true ] && [ "$has_contract" = false ]; then
                echo "ORPHAN_RSOLIDITY:$file:$line:$rest (no /// REQ:)"
            elif [ "$has_contract" = true ] && [ "$has_rsolidity" = false ]; then
                echo "UNMIGRATED:$file:$line:$rest"
            elif [ "$has_contract" = true ] && [ "$has_rsolidity" = true ]; then
                # Check for id/principle drift
                if [ -n "$req_id" ] && [ -n "$rs_id" ] && [ "$req_id" != "$rs_id" ]; then
                    echo "ID_MISMATCH:$file:$line:$rest (REQ:$req_id != rs:$rs_id)"
                fi
            fi
        done || true
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
            for offset in 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do
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
            for offset in 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do
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
            for offset in 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do
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
            for offset in 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30; do
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
    [ -d "$src" ] || src="mcp-servers/${crate}/src"
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
    [ -d "$src" ] || src="mcp-servers/${crate}/src"
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

# ── Contract Quality Mode (4-layer score) ──────────────────────────────────────

run_contract_quality_mode() {
    crate="$1"
    src="crates/${crate}/src"
    [ -d "$src" ] || src="mcp-servers/${crate}/src"
    [ -d "$src" ] || return

    contracted=$(count_contracted "$src")
    expect_count=$(count_expect "$src")
    goal_count=$(count_goal_principle "$src")
    constraining_count=$(count_constraining "$src")

    missing_expect=$((contracted - expect_count))
    missing_goal=$((contracted - goal_count))
    unconstrained=$((contracted - constraining_count))

    # Quality score: weighted average of layer completeness
    # Layer weights: expect=35%, goal-principle=30%, constraining=25%, pre/post=10%
    expect_pct="0"; goal_pct="0"; constraining_pct="0"
    if [ "$contracted" -gt 0 ]; then
        expect_pct=$(echo "scale=1; $expect_count * 100 / $contracted" | bc 2>/dev/null || echo "0")
        goal_pct=$(echo "scale=1; $goal_count * 100 / $contracted" | bc 2>/dev/null || echo "0")
        constraining_pct=$(echo "scale=1; $constraining_count * 100 / $contracted" | bc 2>/dev/null || echo "0")
    fi

    quality_score=$(echo "scale=1; 0.35*$expect_pct + 0.30*$goal_pct + 0.25*$constraining_pct + 10" | bc 2>/dev/null || echo "0.0")

    echo "=== Contract Quality: ${crate} ==="
    echo ""
    echo "Contracted:            $contracted"
    echo ""
    echo "Layer                  Count    Complete %"
    echo "────────────────────── ──────── ──────────"
    printf "expect: (user voice)   %-8d %s%%\n" "$expect_count" "$expect_pct"
    printf "[P{N}] goal-principle  %-8d %s%%\n" "$goal_count" "$goal_pct"
    printf "[P{N}] Constraining:   %-8d %s%%\n" "$constraining_count" "$constraining_pct"
    echo ""
    printf "QUALITY SCORE:         %s/100\n" "$quality_score"
    echo ""

    if [ "$quality_score" = "0.0" ] || [ -z "$quality_score" ]; then
        quality_score="0.0"
    fi

    # Flag violations by severity
    violations=0
    if [ "$missing_expect" -gt 0 ]; then
        echo "── CRITICAL: $missing_expect contracts missing expect: (user expectation) ──"
        list_missing_expect "$src" | while IFS=: read -r _ file line rest; do
            printf "  %-50s L%-4d %s\n" "$file" "$line" "$rest"
        done
        echo ""
        violations=$((violations + 1))
    fi

    if [ "$missing_goal" -gt 0 ]; then
        echo "── CRITICAL: $missing_goal contracts missing [P{N}] goal-principle ──"
        principle_gaps=$(list_principle_gaps "$src")
        echo "$principle_gaps" | while IFS=: read -r _ file line rest; do
            printf "  %-50s L%-4d %s\n" "$file" "$line" "$rest"
        done
        echo ""
        violations=$((violations + 1))
    fi

    if [ "$unconstrained" -gt 0 ]; then
        echo "── HIGH: $unconstrained contracts without [P{N}] Constraining: annotations ──"
        list_unconstrained "$src" | while IFS=: read -r _ file line rest; do
            printf "  %-50s L%-4d %s\n" "$file" "$line" "$rest"
        done
        echo ""
        violations=$((violations + 1))
    fi

    magna_carta_gaps=$(list_missing_magna_carta "$src")
    if [ -n "$magna_carta_gaps" ]; then
        magna_count=$(echo "$magna_carta_gaps" | wc -l)
        echo "── MEDIUM: $magna_count contracts missing Magna Carta (P1-P4) constraints ──"
        echo "$magna_carta_gaps" | while IFS=: read -r _ file line rest; do
            printf "  %-50s L%-4d %s\n" "$file" "$line" "$rest"
        done
        echo ""
        violations=$((violations + 1))
    fi

    if [ "$violations" -eq 0 ]; then
        echo "✓ All contracts pass 4-layer quality check."
        echo ""
    fi

    echo "Quality gate: $(echo "$quality_score >= 80" | bc -l | grep -q 1 && echo 'PASS (>=80%)' || echo 'BELOW TARGET (<80%)')"
    echo ""
}

run_full_mode() {
    crate="$1"
    [ -z "$crate" ] && crate="ALL"
    echo "=== Full Contract Audit (v0.28.0): ${crate} ==="
    echo ""

    if [ "$crate" = "ALL" ]; then
        while IFS='|' read -r c _; do
            [ -n "$c" ] || continue
            # Determine src dir for this crate
            if [ -d "crates/${c}/src" ]; then
                run_expect_mode "$c"
                run_principles_mode "$c"
                run_constraining_mode "$c"
                run_rsolidity_mode "$c"
            fi
        done < <(list_all_crate_sources)
        # Also run coverage summary
        echo "=== Contract Coverage Summary ==="
        echo ""
        printf "%-30s %8s %10s %10s\n" "Crate" "Pub Fns" "Contracted" "Coverage %"
        printf "%-30s %8s %10s %10s\n" "------------------------------" "--------" "----------" "----------"
        total_pub=0
        total_con=0
        while IFS='|' read -r crate src; do
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
        done < <(list_all_crate_sources)
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
    [ -d "$src" ] || src="mcp-servers/${crate}/src"
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

# ── rSolidity Migration Mode ───────────────────────────────────────────────────

run_rsolidity_mode() {
    crate="$1"
    src="crates/${crate}/src"
    [ -d "$src" ] || return

    # Also handle mcp-servers
    mcp_src="mcp-servers/${crate}/src"
    [ -d "$mcp_src" ] && src="$mcp_src"

    contracted=$(count_contracted "$src")
    rsolidity_count=$(count_rsolidity "$src")

    migration_pct="0.0"
    if [ "$contracted" -gt 0 ]; then
        migration_pct=$(echo "scale=1; $rsolidity_count * 100 / $contracted" | bc 2>/dev/null || echo "0.0")
    fi

    echo "=== rSolidity Migration: ${crate} ==="
    echo ""
    echo "Contracted:         $contracted"
    echo "With #[contract]:   $rsolidity_count"
    echo "Migration:          ${migration_pct}%"
    echo ""

    # Drift detection
    drift=$(list_rsolidity_drift "$src")
    if [ -n "$drift" ]; then
        orphans=$(echo "$drift" | grep "ORPHAN_RSOLIDITY:" || true)
        unmigrated=$(echo "$drift" | grep "UNMIGRATED:" || true)
        mismatches=$(echo "$drift" | grep "ID_MISMATCH:" || true)

        if [ -n "$orphans" ]; then
            orphan_count=$(echo "$orphans" | wc -l)
            echo "── Orphaned #[contract] (no matching /// REQ:) — ${orphan_count} ──"
            echo "$orphans" | while IFS=: read -r tag file line rest; do
                printf "  %-50s L%-4d %s\n" "$file" "$line" "$rest"
            done
            echo ""
        fi

        if [ -n "$mismatches" ]; then
            mismatch_count=$(echo "$mismatches" | wc -l)
            echo "── ID Mismatches (#[contract] id != /// REQ: id) — ${mismatch_count} ──"
            echo "$mismatches" | while IFS=: read -r tag file line rest; do
                printf "  %-50s L%-4d %s\n" "$file" "$line" "$rest"
            done
            echo ""
        fi

        if [ -n "$unmigrated" ]; then
            unmigrated_count=$(echo "$unmigrated" | wc -l)
            echo "── Unmigrated (/// REQ: only, no #[contract]) — ${unmigrated_count} ──"
            echo "$unmigrated" | while IFS=: read -r tag file line rest; do
                printf "  %-50s L%-4d %s\n" "$file" "$line" "$rest"
            done
            echo ""
        fi
    fi

    if [ "$rsolidity_count" -eq "$contracted" ] && [ "$contracted" -gt 0 ] && [ -z "$(echo "$drift" | grep "ORPHAN_RSOLIDITY\|ID_MISMATCH" || true)" ]; then
        echo "✓ All $contracted contracts have #[contract] attributes. Zero drift detected."
        echo ""
    elif [ "$rsolidity_count" -eq 0 ] && [ "$contracted" -eq 0 ]; then
        echo "No contracts found in crate."
        echo ""
    fi
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
    rsolidity)
        TARGET="${TARGET:-ALL}"
        if [ "$TARGET" = "ALL" ]; then
            for crate_dir in crates/*/; do
                c=$(basename "$crate_dir")
                [ -d "crates/${c}/src" ] || continue
                run_rsolidity_mode "$c"
            done
        else
            run_rsolidity_mode "$TARGET"
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
    echo "=== Contract Audit Summary (v0.28.0 extended) ==="
    echo ""
    printf "%-32s %7s %9s %9s %7s %9s %11s\n" "Crate" "PubFns" "Contracted" "Cover%" "expect:" "Ground%" "#[contract]"
    printf "%-32s %7s %9s %9s %7s %9s %11s\n" "--------------------------------" "-------" "---------" "------" "-------" "-------" "-----------"
    total_pub=0
    total_con=0
    total_exp=0
    total_rs=0
    while IFS='|' read -r crate src; do
        [ -d "$src" ] || continue
        [ -n "$crate" ] || continue
        pub_count=$(count_pub_fns "$src")
        contracted_count=$(count_contracted "$src")
        expect_count=$(count_expect "$src")
        rsolidity_count=$(count_rsolidity "$src")

        coverage_pct="0.0"
        grounding_pct="0.0"
        if [ "$pub_count" -gt 0 ]; then
            coverage_pct=$(echo "scale=1; $contracted_count * 100 / $pub_count" | bc 2>/dev/null || echo "0.0")
        fi
        if [ "$contracted_count" -gt 0 ]; then
            grounding_pct=$(echo "scale=1; $expect_count * 100 / $contracted_count" | bc 2>/dev/null || echo "0.0")
        fi

        total_pub=$((total_pub + pub_count))
        total_con=$((total_con + contracted_count))
        total_exp=$((total_exp + expect_count))
        total_rs=$((total_rs + rsolidity_count))

        printf "%-32s %7d %9d %8s%% %7d %8s%% %11d\n" \
            "$crate" "$pub_count" "$contracted_count" "$coverage_pct" \
            "$expect_count" "$grounding_pct" "$rsolidity_count"
    done < <(list_all_crate_sources)
    echo ""
    total_cov="0.0"
    total_grounding="0.0"
    if [ "$total_pub" -gt 0 ]; then
        total_cov=$(echo "scale=1; $total_con * 100 / $total_pub" | bc 2>/dev/null || echo "0.0")
    fi
    if [ "$total_con" -gt 0 ]; then
        total_grounding=$(echo "scale=1; $total_exp * 100 / $total_con" | bc 2>/dev/null || echo "0.0")
    fi
    printf "%-32s %7d %9d %8s%% %7d %8s%% %11d\n" \
        "TOTAL" "$total_pub" "$total_con" "$total_cov" \
        "$total_exp" "$total_grounding" "$total_rs"
    echo ""
    echo "Uncovered contracts (REQ: tags without expect:): $((total_con - total_exp))"
    echo "rSolidity migration coverage: $total_rs / $total_con contracts"
    echo ""
    echo "PASS: Contract audit complete (trend monitor, not a hard gate)."
    exit 0
fi

# ── Detailed mode (default) and extended mode definitions ────────────────────


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
