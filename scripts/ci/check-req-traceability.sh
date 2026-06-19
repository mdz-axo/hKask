#!/usr/bin/env bash
# REQ traceability check — strict per-test presence gate + quality linter.
#
# Enforces Testing Discipline T4:
#   Every test function carries a nearby `// REQ:` tag traceable to a requirement.
#
# Also rejects placeholder/non-contract anchors:
#   - `REQ: pre:`
#   - `REQ: autogen-*`
#
# Quality mode:
#   Flags REQ lines that look like prose summaries rather than stable IDs/principle anchors.
#   Set STRICT_REQ_QUALITY=1 to make quality violations fail the gate.
#
# Pure bash — no Python dependency.

set -euo pipefail

echo "=== REQ Traceability Check (strict) ==="
echo ""

STRICT_REQ_QUALITY="${STRICT_REQ_QUALITY:-1}"

# ── Temporary files ──────────────────────────────────────────────────────────

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

MISSING_FILE="$TMPDIR/missing"
PLACEHOLDER_FILE="$TMPDIR/placeholder"
QUALITY_FILE="$TMPDIR/quality"
CRATE_STATS="$TMPDIR/crate_stats"
:> "$MISSING_FILE"
:> "$PLACEHOLDER_FILE"
:> "$QUALITY_FILE"
:> "$CRATE_STATS"

# ── Helpers ───────────────────────────────────────────────────────────────────

# Check if a REQ tag has a stable ID (principle ref P1-P12, or token with
# separators/digits). Returns 0 if quality is OK, 1 if flagged.
req_quality_ok() {
    local req_line="$1"

    # Principle reference: P1 through P12
    if echo "$req_line" | grep -qE '\bP(1[0-2]|[1-9])\b'; then
        return 0
    fi

    # Extract token immediately after "REQ:" up to em dash, space, or end-of-comment
    local token
    token=$(echo "$req_line" | sed -n 's/.*REQ:\s*\([^—[:space:]/]\+\).*/\1/p')
    if [ -z "$token" ]; then
        return 1
    fi

    # Stable ID heuristics: has separators (- _ . :) or digits
    if echo "$token" | grep -qE '[-_.:]|[0-9]'; then
        return 0
    fi

    return 1
}

# Determine crate name from file path.
# "crates/hkask-foo/src/bar.rs" → "hkask-foo"
# "mcp-servers/hkask-mcp-foo/src/main.rs" → "mcp-servers/hkask-mcp-foo"
crate_from_path() {
    local path="$1"
    if [[ "$path" == crates/* ]]; then
        echo "$path" | cut -d/ -f2
    elif [[ "$path" == mcp-servers/* ]]; then
        echo "$path" | cut -d/ -f1-2
    else
        echo "unknown"
    fi
}

# ── Scan ──────────────────────────────────────────────────────────────────────

total_tests=0
total_with_req=0
total_missing=0
total_placeholder=0
total_quality=0

# Per-crate counters using associative array
declare -A crate_tests
declare -A crate_with_req
declare -A crate_missing
declare -A crate_quality

for root in crates mcp-servers; do
    [ -d "$root" ] || continue

    while IFS= read -r -d '' file; do
        # Find all #[test] and #[tokio::test] line numbers
        test_lines=$(grep -nE '^\s*#\[(tokio::)?test(\s*\(.*\))?\]' "$file" 2>/dev/null || true)
        [ -z "$test_lines" ] && continue

        crate=$(crate_from_path "$file")

        while IFS= read -r tline; do
            line_num=$(echo "$tline" | cut -d: -f1)
            total_tests=$((total_tests + 1))
            crate_tests["$crate"]=$((${crate_tests["$crate"]:-0} + 1))

            # Check the 6 lines before the test attribute for a REQ: tag
            start=$((line_num > 6 ? line_num - 6 : 1))
            end=$((line_num - 1))
            prior_lines=$(sed -n "${start},${end}p" "$file" 2>/dev/null)

            req_match=$(echo "$prior_lines" | grep -E 'REQ:|expect:|contract:' | tail -1 || true)
            if [ -n "$req_match" ] && ! echo "$req_match" | grep -q 'expect:/post:'; then
                total_with_req=$((total_with_req + 1))
                crate_with_req["$crate"]=$((${crate_with_req["$crate"]:-0} + 1))

                # Check for placeholder REQ tags
                if echo "$req_match" | grep -qE 'REQ:\s*(pre:|autogen-)'; then
                    total_placeholder=$((total_placeholder + 1))
                    echo "$file:$line_num :: $req_match" >> "$PLACEHOLDER_FILE"
                fi

                # Quality check
                if ! req_quality_ok "$req_match"; then
                    total_quality=$((total_quality + 1))
                    crate_quality["$crate"]=$((${crate_quality["$crate"]:-0} + 1))
                    echo "$file:$line_num :: $req_match" >> "$QUALITY_FILE"
                fi
            else
                total_missing=$((total_missing + 1))
                crate_missing["$crate"]=$((${crate_missing["$crate"]:-0} + 1))
                echo "$file:$line_num" >> "$MISSING_FILE"
            fi
        done <<< "$test_lines"
    done < <(find "$root" -name '*.rs' -print0 2>/dev/null)
done

# ── Report ────────────────────────────────────────────────────────────────────

echo "crate,tests,with_req,missing,quality_flags,coverage"

# Collect all crate names
declare -A all_crates
for crate in "${!crate_tests[@]}"; do all_crates["$crate"]=1; done
for crate in "${!crate_with_req[@]}"; do all_crates["$crate"]=1; done
for crate in "${!crate_missing[@]}"; do all_crates["$crate"]=1; done
for crate in "${!crate_quality[@]}"; do all_crates["$crate"]=1; done

for crate in $(printf '%s\n' "${!all_crates[@]}" | sort); do
    t=${crate_tests["$crate"]:-0}
    w=${crate_with_req["$crate"]:-0}
    m=${crate_missing["$crate"]:-0}
    q=${crate_quality["$crate"]:-0}
    if [ "$t" -gt 0 ]; then
        pct=$(awk "BEGIN {printf \"%.1f\", ($w * 100.0 / $t)}")
    else
        pct="0.0"
    fi
    echo "$crate,$t,$w,$m,$q,${pct}%"
done

echo ""
echo "Total tests: $total_tests"
echo "Missing REQ tags: $total_missing"
echo "Placeholder REQ tags: $total_placeholder"
echo "REQ quality flags: $total_quality"

# ── Errors ────────────────────────────────────────────────────────────────────

FAILED=0

if [ "$total_missing" -gt 0 ]; then
    echo ""
    echo "ERROR: tests missing nearby REQ tag:"
    head -100 "$MISSING_FILE" | while IFS= read -r line; do
        echo "  - $line"
    done
    FAILED=1
fi

if [ "$total_placeholder" -gt 0 ]; then
    echo ""
    echo "ERROR: placeholder REQ tags found (replace with real requirement IDs):"
    head -100 "$PLACEHOLDER_FILE" | while IFS= read -r line; do
        echo "  - $line"
    done
    FAILED=1
fi

if [ "$total_quality" -gt 0 ]; then
    level="WARN"
    if [ "$STRICT_REQ_QUALITY" = "1" ]; then
        level="ERROR"
        FAILED=1
    fi
    echo ""
    echo "$level: REQ tags lacking stable ID/principle anchor:"
    head -120 "$QUALITY_FILE" | while IFS= read -r line; do
        echo "  - $line"
    done
fi

# ── Result ────────────────────────────────────────────────────────────────────

echo ""
if [ "$FAILED" -eq 0 ]; then
    echo "PASS: every test has a nearby non-placeholder REQ anchor."
    if [ "$total_quality" -gt 0 ] && [ "$STRICT_REQ_QUALITY" != "1" ]; then
        echo "PASS (with warnings): enable STRICT_REQ_QUALITY=1 to enforce quality flags as hard failures."
    fi
    exit 0
else
    exit 1
fi
