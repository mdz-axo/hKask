#!/usr/bin/env bash
# Contract Quality Review — generates a reviewable manifest of all REQ-tagged
# contracts across the workspace. Flags potential quality issues:
#   - Vacuous contracts (pre: true, post: true or no pre/post)
#   - Orphan REQ tags (no pre/post conditions)
#   - Naming inconsistencies
#   - Duplicate REQ IDs across crates
#
# Usage: bash scripts/review-contracts.sh [--crate <name>] [--format json|text]
# Output: full inventory for collaborative review

set -euo pipefail

MODE="text"
TARGET=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --format) MODE="$2"; shift 2 ;;
        --crate)  TARGET="$2"; shift 2 ;;
        *) shift ;;
    esac
done

CRATE_DIRS="crates/*/src"
if [ -n "$TARGET" ]; then
    CRATE_DIRS="crates/$TARGET/src"
fi

# ── Extract contract information ──────────────────────────────

extract_contracts() {
    local dir="$1"
    local crate_name
    crate_name=$(echo "$dir" | sed 's|crates/\([^/]*\)/src|\1|')

    # Find all pub fn with REQ tags above them
    grep -rn "pub fn \|pub async fn " "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" | grep -v "/tests/" \
        | while IFS=: read -r file line sig; do
            local fn_name
            fn_name=$(echo "$sig" | sed 's/.*fn \([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/')

            # Look for REQ tag and pre/post in 20 lines above
            local req=""
            local pre=""
            local post=""
            local inv=""

            for offset in $(seq 1 20); do
                local check=$((line - offset))
                [ "$check" -lt 1 ] && break
                local ctx_line
                ctx_line=$(sed -n "${check}p" "$file" 2>/dev/null)

                # Extract REQ tag
                if echo "$ctx_line" | grep -q "REQ:" && [ -z "$req" ]; then
                    req=$(echo "$ctx_line" | sed 's/.*REQ: *//' | awk '{print $1}' | sed 's/[;,.)}\]].*//')
                fi

                # Extract pre/post/inv
                if echo "$ctx_line" | grep -q "pre:" && [ -z "$pre" ]; then
                    pre=$(echo "$ctx_line" | sed 's/.*pre: *//' | sed 's/^[[:space:]]*//')
                fi
                if echo "$ctx_line" | grep -q "post:" && [ -z "$post" ]; then
                    post=$(echo "$ctx_line" | sed 's/.*post: *//' | sed 's/^[[:space:]]*//')
                fi
                if echo "$ctx_line" | grep -q "inv:" && [ -z "$inv" ]; then
                    inv=$(echo "$ctx_line" | sed 's/.*inv: *//' | sed 's/^[[:space:]]*//')
                fi
            done

            if [ -n "$req" ]; then
                # Quality flags
                local flags=""
                # Vacuous contract
                if [ "$pre" = "true" ] && [ "$post" = "true" ]; then
                    flags="${flags}VACUOUS "
                fi
                # Missing pre/post
                [ -z "$pre" ] && flags="${flags}NO_PRE "
                [ -z "$post" ] && flags="${flags}NO_POST "
                # Short/placeholder pre/post
                [ "${#pre}" -lt 5 ] && [ -n "$pre" ] && flags="${flags}SHORT_PRE "
                [ "${#post}" -lt 5 ] && [ -n "$post" ] && flags="${flags}SHORT_POST "

                if [ "$MODE" = "json" ]; then
                    echo "{\"crate\":\"$crate_name\",\"function\":\"$fn_name\",\"file\":\"$file\",\"line\":$line,\"req\":\"$req\",\"pre\":\"$pre\",\"post\":\"$post\",\"inv\":\"$inv\",\"flags\":\"$flags\"}"
                fi
            fi
        done
}

# ── Report ────────────────────────────────────────────────────

if [ "$MODE" = "json" ]; then
    echo "["
    first=true
    for dir in $CRATE_DIRS; do
        [ -d "$dir" ] || continue
        results=$(extract_contracts "$dir")
        if [ -n "$results" ]; then
            while IFS= read -r line; do
                [ -z "$line" ] && continue
                if [ "$first" = true ]; then first=false; else echo ","; fi
                echo -n "  $line"
            done <<< "$results"
        fi
    done
    echo ""
    echo "]"
    exit 0
fi

# Text mode — reviewable manifest
echo "# Contract Quality Review"
echo "## Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

total=0
vacuous=0
no_pre=0
no_post=0
duplicate_ids=""

declare -A seen_req_ids

for dir in $CRATE_DIRS; do
    [ -d "$dir" ] || continue
    crate_name=$(echo "$dir" | sed 's|crates/\([^/]*\)/src|\1|')

    contracts=$(grep -rn "pub fn \|pub async fn " "$dir" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" | grep -v "/tests/")

    crate_total=0
    crate_issues=0

    while IFS=: read -r file line sig; do
        fn_name=$(echo "$sig" | sed 's/.*fn \([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/')

        req=""
        pre=""
        post=""

        for offset in $(seq 1 20); do
            check=$((line - offset))
            [ "$check" -lt 1 ] && break
            ctx_line=$(sed -n "${check}p" "$file" 2>/dev/null)

            if echo "$ctx_line" | grep -q "REQ:" && [ -z "$req" ]; then
                req=$(echo "$ctx_line" | sed 's/.*REQ: *//' | awk '{print $1}' | sed 's/[;,.)\]}].*//')
            fi
            if echo "$ctx_line" | grep -q "pre:" && [ -z "$pre" ]; then
                pre=$(echo "$ctx_line" | sed 's/.*pre: *//')
            fi
            if echo "$ctx_line" | grep -q "post:" && [ -z "$post" ]; then
                post=$(echo "$ctx_line" | sed 's/.*post: *//')
            fi
        done

        if [ -n "$req" ]; then
            crate_total=$((crate_total + 1))
            total=$((total + 1))

            # Check for duplicates
            if [ -n "${seen_req_ids[$req]:-}" ]; then
                duplicate_ids="$duplicate_ids\n  DUPLICATE: $req (${seen_req_ids[$req]} and $crate_name::$fn_name)"
            fi
            seen_req_ids[$req]="$crate_name::$fn_name"

            # Quality checks
            issues=""
            if [ "$pre" = "true" ] && [ "$post" = "true" ]; then
                issues="VACUOUS"
                vacuous=$((vacuous + 1))
            fi
            [ -z "$pre" ] && issues="$issues NO_PRE" && no_pre=$((no_pre + 1))
            [ -z "$post" ] && issues="$issues NO_POST" && no_post=$((no_post + 1))

            if [ -n "$issues" ]; then
                crate_issues=$((crate_issues + 1))
            fi
        fi
    done <<< "$contracts"

    echo "## $crate_name ($crate_total contracted, $crate_issues issues)"
    echo ""
done

echo ""
echo "---"
echo ""
echo "## Summary"
echo ""
echo "- Total contracted functions: $total"
echo "- Vacuous contracts (pre:true, post:true): $vacuous"
echo "- Missing preconditions: $no_pre"
echo "- Missing postconditions: $no_post"

if [ -n "$duplicate_ids" ]; then
    echo ""
    echo "## Duplicate REQ IDs"
    echo -e "$duplicate_ids"
fi

echo ""
echo "## Review Instructions"
echo ""
echo "1. For each VACUOUS contract: strengthen pre/post to capture actual behavior"
echo "2. For each NO_PRE/NO_POST: add the missing condition"
echo "3. For each DUPLICATE: ensure unique IDs or consolidate if same contract"
echo "4. Use 'kask contract discover' to find uncontracted functions"
echo "5. Use '/improv plussing review <function>' for collaborative refinement"
