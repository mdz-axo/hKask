#!/usr/bin/env bash
# Test inventory — produces structured JSON of the hKask test corpus.
#
# Outputs a JSON document listing every test function with:
#   - crate, file, line, test name
#   - REQ tag presence (and tag content)
#   - Test type (unit, integration, proptest, fuzz)
#   - Contract presence (// REQ: pre: on the function under test)
#   - Discipline violations (no REQ tag, example-based instead of property)
#
# Usage:
#   scripts/test-inventory.sh              # full JSON
#   scripts/test-inventory.sh --summary    # per-crate summary only
#   scripts/test-inventory.sh --violations # discipline violations only
#   scripts/test-inventory.sh --crate X    # single crate
#
# Exit 0 always (informational tool, not a quality gate).
#
# Reference: docs/architecture/core/TESTING_DISCIPLINE.md §4, §8
#            test-harness-maturation-plan-v0.27.0.md §10

set -euo pipefail

MODE="full"
TARGET=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --summary)   MODE="summary"; shift ;;
        --violations) MODE="violations"; shift ;;
        --crate)     MODE="crate"; TARGET="$2"; shift 2 ;;
        *)           echo "Unknown flag: $1"; exit 1 ;;
    esac
done

# ── Helpers ──────────────────────────────────────────────────────────────────

# Extract test functions with metadata from a file.
# Output: crate:file:line:test_name:req_tag:test_type:has_contract:is_example
# test_type is one of: unit, integration, proptest, fuzz, doc
classify_tests_in_file() {
    local file="$1"
    local crate="$2"
    local test_type="$3"  # unit or integration

    local content
    content=$(cat "$file" 2>/dev/null) || return

    # Collect all lines so we can check nearby context for REQ tags
    local -a lines
    mapfile -t lines <<< "$content"
    local total="${#lines[@]}"

    local i=0
    while [[ $i -lt $total ]]; do
        local line="${lines[$i]}"
        # Match #[test], #[tokio::test], #[rstest], proptest! macro
        if echo "$line" | grep -qE '#\[(test|tokio::test|rstest)\]|^\s*proptest!'; then
            local test_name=""
            local is_proptest=false
            local req_tag=""
            local is_ignored=false

            # Check if it's proptest macro block
            if echo "$line" | grep -qE '^\s*proptest!'; then
                is_proptest=true
            fi

            # Check for #[ignore]
            if echo "$line" | grep -q '#\[ignore\]'; then
                is_ignored=true
            fi

            # Look ahead for the function name (fn test_xxx)
            local j=$((i + 1))
            while [[ $j -lt $total && $j -lt $((i + 5)) ]]; do
                local ahead="${lines[$j]}"
                if echo "$ahead" | grep -qE '^\s*(pub\s+)?(async\s+)?fn\s+(\w+)'; then
                    test_name=$(echo "$ahead" | sed -E 's/^\s*(pub\s+)?(async\s+)?fn\s+(\w+).*/\3/')
                    break
                elif echo "$ahead" | grep -qE '#\[test\]'; then
                    # Proptest! block with #[test] inside
                    test_name="proptest_block"
                    break
                fi
                j=$((j + 1))
            done

            # Look for REQ tag within 5 lines above the test
            local k=$((i - 1))
            while [[ $k -ge 0 && $k -ge $((i - 6)) ]]; do
                local above="${lines[$k]}"
                if echo "$above" | grep -q '// REQ:'; then
                    req_tag=$(echo "$above" | sed -n 's/.*\/\/ REQ:\s*\([^—]*\).*/\1/p' | xargs)
                    break
                fi
                k=$((k - 1))
            done

            # Determine final test type
            local final_type="$test_type"
            if $is_proptest; then
                final_type="proptest"
            fi
            if $is_ignored; then
                final_type="${final_type}_ignored"
            fi

            # Determine if example-based (heuristic: tests with assert_eq! and no proptest)
            local is_example_based="false"
            if ! $is_proptest; then
                # Look ahead for assert_eq! pattern in the test body
                local body_start=$j
                local body_end=$((body_start + 30))
                [[ $body_end -gt $total ]] && body_end=$total
                local snippet=""
                for ((b=body_start; b<body_end; b++)); do
                    snippet+="${lines[$b]}"$'\n'
                done
                if echo "$snippet" | grep -q 'assert_eq!'; then
                    is_example_based="true"
                fi
            fi

            # Check if there's a contract (// REQ: pre:) on the function under test
            # by searching the same file for a function with matching name and REQ contract
            local has_contract="false"
            if [[ -n "$test_name" && "$test_name" != "proptest_block" ]]; then
                # Search for pub fn with same name and check for REQ within 5 lines above it
                if grep -n "pub.*fn $test_name\b" "$file" 2>/dev/null | head -1 | grep -q .; then
                    local fn_line
                    fn_line=$(grep -n "pub.*fn $test_name\b" "$file" 2>/dev/null | head -1 | cut -d: -f1)
                    if [[ -n "$fn_line" ]]; then
                        local ck=$((fn_line - 1))
                        while [[ $ck -ge 0 && $ck -ge $((fn_line - 6)) ]]; do
                            if sed -n "${ck}p" "$file" 2>/dev/null | grep -q '///* REQ:'; then
                                has_contract="true"
                                break
                            fi
                            ck=$((ck - 1))
                        done
                    fi
                fi
            fi

            if [[ -n "$test_name" ]]; then
                echo "${crate}:${file}:${i}:${test_name}:${req_tag}:${final_type}:${has_contract}:${is_example_based}"
            fi
        fi
        i=$((i + 1))
    done
}

# Scan a crate for all tests (inline + integration)
scan_crate() {
    local crate_dir="$1"
    local crate_name="$2"

    # Unit tests (inline #[cfg(test)] modules)
    find "$crate_dir/src" -name "*.rs" 2>/dev/null | while read -r file; do
        if grep -q '#\[test\]\|#\[tokio::test\]\|proptest!' "$file" 2>/dev/null; then
            classify_tests_in_file "$file" "$crate_name" "unit"
        fi
    done

    # Integration tests (tests/ directory)
    find "$crate_dir/tests" -name "*.rs" 2>/dev/null | while read -r file; do
        if grep -q '#\[test\]\|#\[tokio::test\]\|proptest!' "$file" 2>/dev/null; then
            classify_tests_in_file "$file" "$crate_name" "integration"
        fi
    done
}

# ── Main ─────────────────────────────────────────────────────────────────────

# Collect all test records
TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT
RECORDS="$TMPDIR/records"
:> "$RECORDS"

if [[ -n "$TARGET" ]]; then
    crate_dir="crates/${TARGET}"
    if [[ -d "$crate_dir" ]]; then
        scan_crate "$crate_dir" "$TARGET" >> "$RECORDS" 2>/dev/null || true
    fi
    # Also check mcp-servers
    mcp_dir="mcp-servers/${TARGET}"
    if [[ -d "$mcp_dir" ]]; then
        scan_crate "$mcp_dir" "$TARGET" >> "$RECORDS" 2>/dev/null || true
    fi
else
    for crate_dir in crates/*/; do
        crate=$(basename "$crate_dir")
        scan_crate "$crate_dir" "$crate" >> "$RECORDS" 2>/dev/null || true
    done
    for mcp_dir in mcp-servers/*/; do
        crate=$(basename "$mcp_dir")
        scan_crate "$mcp_dir" "$crate" >> "$RECORDS" 2>/dev/null || true
    done
fi

sort -u "$RECORDS" -o "$RECORDS"

# ── Compute summary per crate ────────────────────────────────────────────────

compute_crate_summary() {
    local crate="$1"
    local records_file="$2"

    local total=0
    local with_req=0
    local without_req=0
    local proptest_count=0
    local unit_count=0
    local integration_count=0
    local with_contract=0
    local example_based=0
    local ignored_count=0

    while IFS=: read -r crate_rec file line name req type has_contract is_example; do
        if [[ "$crate_rec" != "$crate" ]]; then continue; fi
        total=$((total + 1))
        if [[ -n "$req" ]]; then with_req=$((with_req + 1)); else without_req=$((without_req + 1)); fi
        if [[ "$type" == *"proptest"* ]]; then proptest_count=$((proptest_count + 1)); fi
        if [[ "$type" == *"ignored"* ]]; then ignored_count=$((ignored_count + 1)); fi
        if [[ "$type" == "unit"* ]]; then unit_count=$((unit_count + 1)); fi
        if [[ "$type" == "integration"* ]]; then integration_count=$((integration_count + 1)); fi
        if [[ "$has_contract" == "true" ]]; then with_contract=$((with_contract + 1)); fi
        if [[ "$is_example" == "true" ]]; then example_based=$((example_based + 1)); fi
    done < <(grep "^[^:]*:[^:]*:[^:]*:[^:]*:[^:]*:[^:]*:" "$records_file" | grep ":${crate}:" 2>/dev/null || true)

    local req_pct=0
    if [[ $total -gt 0 ]]; then
        req_pct=$(echo "scale=1; $with_req * 100 / $total" | bc 2>/dev/null || echo "0.0")
    fi

    local violations=0
    if [[ $without_req -gt 0 ]]; then violations=$((violations + without_req)); fi
    if [[ $example_based -gt 0 ]]; then violations=$((violations + example_based)); fi

    echo "${total}:${with_req}:${without_req}:${proptest_count}:${unit_count}:${integration_count}:${with_contract}:${example_based}:${ignored_count}:${req_pct}:${violations}"
}

# ── Output ───────────────────────────────────────────────────────────────────

if [[ "$MODE" == "summary" ]]; then
    echo "{"
    echo "  \"generated_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "  \"crates\": {"

    first=true
    # Collect unique crate names
    crates=$(cut -d: -f5 "$RECORDS" | sort -u)
    for crate in $crates; do
        if [[ -z "$crate" ]]; then continue; fi
        if grep -q "^[^:]*:[^:]*:[^:]*:[^:]*:${crate}:" "$RECORDS" 2>/dev/null; then
            summary=$(compute_crate_summary "$crate" "$RECORDS")
            IFS=: read -r total with_req without_req ptest unit integ contract example ignored pct v <<< "$summary"

            if $first; then first=false; else echo ","; fi
            echo -n "    \"$crate\": {"
            echo -n "\"total_tests\": $total, "
            echo -n "\"with_req\": $with_req, "
            echo -n "\"without_req\": $without_req, "
            echo -n "\"proptest\": $ptest, "
            echo -n "\"unit\": $unit, "
            echo -n "\"integration\": $integ, "
            echo -n "\"with_contract\": $contract, "
            echo -n "\"example_based\": $example, "
            echo -n "\"ignored\": $ignored, "
            echo -n "\"req_coverage_pct\": $pct, "
            echo -n "\"discipline_violations\": $v"
            echo -n "}"
        fi
    done

    echo ""
    echo "  }"
    echo "}"
    exit 0
fi

if [[ "$MODE" == "violations" ]]; then
    echo "{"
    echo "  \"generated_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
    echo "  \"violations\": ["

    first=true
    while IFS=: read -r file line name req type has_contract is_example crate; do
        if [[ -z "$name" ]]; then continue; fi

        # Violation: no REQ tag
        if [[ -z "$req" ]]; then
            if $first; then first=false; else echo ","; fi
            echo "    {\"crate\": \"$crate\", \"file\": \"$file\", \"line\": $((line + 1)), \"test\": \"$name\", \"violation\": \"missing_req_tag\", \"detail\": \"Test has no // REQ: tag. Add // REQ: <spec_id> — description above the test.\"}"
        fi

        # Violation: example-based (could be property-based)
        if [[ "$is_example" == "true" ]]; then
            if $first; then first=false; else echo ","; fi
            echo "    {\"crate\": \"$crate\", \"file\": \"$file\", \"line\": $((line + 1)), \"test\": \"$name\", \"violation\": \"example_based\", \"detail\": \"Test uses assert_eq! with literal values. Consider converting to proptest for property-based verification.\"}"
        fi

        # Violation: contract missing on function under test
        if [[ "$has_contract" == "false" && -n "$req" ]]; then
            if $first; then first=false; else echo ","; fi
            echo "    {\"crate\": \"$crate\", \"file\": \"$file\", \"line\": $((line + 1)), \"test\": \"$name\", \"violation\": \"uncontracted_function\", \"detail\": \"Test references spec via REQ tag but the function under test has no // REQ: pre: contract.\"}"
        fi
    done < <(while IFS=: read -r file line name req type has_contract is_example; do
        crate=$(echo "$file" | cut -d/ -f2)
        echo "$file:$line:$name:$req:$type:$has_contract:$is_example:$crate"
    done < "$RECORDS")

    echo ""
    echo "  ]"
    echo "}"
    exit 0
fi

# ── Full mode (default) ──────────────────────────────────────────────────────

echo "{"
echo "  \"generated_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
echo "  \"summary\": {"
echo "    \"total_tests\": $(wc -l < "$RECORDS" | tr -d ' '),"
echo "    \"crates_with_tests\": $(cut -d: -f5 "$RECORDS" | sort -u | wc -l | tr -d ' ')"
echo "  },"

# Per-crate summaries
echo "  \"crates\": {"
first=true
crates=$(cut -d: -f5 "$RECORDS" | sort -u)
for crate in $crates; do
    if [[ -z "$crate" ]]; then continue; fi
    summary=$(compute_crate_summary "$crate" "$RECORDS")
    IFS=: read -r total with_req without_req ptest unit integ contract example ignored pct v <<< "$summary"
    if $first; then first=false; else echo ","; fi
    echo -n "    \"$crate\": {"
    echo -n "\"total\": $total, \"with_req\": $with_req, \"without_req\": $without_req, \"proptest\": $ptest, \"req_pct\": $pct, \"violations\": $v"
    echo -n "}"
done
echo ""
echo "  },"

# Per-test list
echo "  \"tests\": ["
first=true
while IFS=: read -r file line name req type has_contract is_example; do
    if [[ -z "$name" ]]; then continue; fi
    if $first; then first=false; else echo ","; fi
    # Escape JSON strings
    file_escaped=$(echo "$file" | sed 's/"/\\"/g')
    name_escaped=$(echo "$name" | sed 's/"/\\"/g')
    req_escaped=$(echo "$req" | sed 's/"/\\"/g')
    echo -n "    {\"file\": \"$file_escaped\", \"line\": $((line + 1)), \"name\": \"$name_escaped\", \"req\": \"$req_escaped\", \"type\": \"$type\", \"has_contract\": $has_contract, \"is_example_based\": $is_example}"
done < "$RECORDS"
echo ""
echo "  ]"
echo "}"

exit 0
