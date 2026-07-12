#!/usr/bin/env bash
# Citation Audit — checks citation density across the documentation corpus.
#
# Per DOCUMENTATION_STANDARDS.md section 5.3:
#   Every ##-level section SHOULD contain >=1 [^...] footnote citation.
#
# Usage:
#   ./docs/ci/check-citations.sh [--strict] [--verbose]
#
#   --strict   Exit non-zero if any document has uncited sections.
#   --verbose  Show which specific sections lack citations.

set -euo pipefail

STRICT=false
VERBOSE=false
for arg in "$@"; do
    case "$arg" in
        --strict) STRICT=true ;;
        --verbose) VERBOSE=true ;;
        *) echo "Unknown flag: $arg"; exit 1 ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$WORKSPACE_ROOT"

TOTAL_DOCS=0; PASS=0; GAP=0; EXEMPT=0; TOTAL_UNCITED=0; HAS_FAILURES=false

echo "=== Citation Audit ==="
echo "Rule: Every ## section SHOULD have >=1 [^...] footnote citation"
echo "Scope: docs/ (excluding archive/)"
echo ""

audit_file() {
    local file="$1" rel="${file#docs/}"
    TOTAL_DOCS=$((TOTAL_DOCS + 1))

    # Extract ## headings and their line numbers
    local headings
    headings=$(grep -n '^## ' "$file" 2>/dev/null || true)
    if [ -z "$headings" ]; then
        echo "  EXEMPT,0  $rel"
        EXEMPT=$((EXEMPT + 1))
        return
    fi

    local section_count
    section_count=$(echo "$headings" | wc -l)

    local citation_count
    citation_count=$(grep -c '\[\^' "$file" 2>/dev/null || echo 0)

    # Build array of heading line numbers
    local -a heading_lines=()
    local -a heading_names=()
    while IFS=: read -r lineno name; do
        heading_lines+=("$lineno")
        heading_names+=("$name")
    done <<< "$headings"

    local last_line
    last_line=$(wc -l < "$file")
    local uncited=0
    local uncited_names=""

    for i in "${!heading_lines[@]}"; do
        local start=${heading_lines[$i]}
        local end
        if [ "$i" -lt $((${#heading_lines[@]} - 1)) ]; then
            end=$((${heading_lines[$i+1]} - 1))
        else
            end=$last_line
        fi

        # Check whether this section contains a [^...] citation
        if ! sed -n "${start},${end}p" "$file" | grep -q '\[\^'; then
            uncited=$((uncited + 1))
            if [ "$VERBOSE" = true ]; then
                local sname
                sname=$(echo "${heading_names[$i]}" | sed 's/^## //')
                uncited_names="${uncited_names}    - $sname"$'\n'
            fi
        fi
    done

    TOTAL_UNCITED=$((TOTAL_UNCITED + uncited))

    if [ "$uncited" -eq 0 ]; then
        echo "  PASS,0    $rel  ($citation_count citations, $section_count sections)"
        PASS=$((PASS + 1))
    else
        echo "  GAP,$uncited   $rel  ($citation_count citations, $section_count sections, $uncited uncited)"
        if [ "$VERBOSE" = true ] && [ -n "$uncited_names" ]; then
            printf '%s' "$uncited_names"
        fi
        GAP=$((GAP + 1))
        HAS_FAILURES=true
    fi
}

while IFS= read -r -d '' file; do
    audit_file "$file"
done < <(find docs -name '*.md' -not -path '*/archive/*' -print0 2>/dev/null | sort -z)

echo ""
echo "=== Summary ==="
echo "  Total documents:    $TOTAL_DOCS"
echo "  PASS (all cited):   $PASS"
echo "  GAP (uncited):      $GAP"
echo "  EXEMPT (no sections): $EXEMPT"
echo "  Total uncited:      $TOTAL_UNCITED"
echo ""

if [ "$STRICT" = true ] && [ "$HAS_FAILURES" = true ]; then
    echo "FAIL: $GAP document(s) have uncited sections."
    exit 1
elif [ "$HAS_FAILURES" = true ]; then
    echo "NOTE: $GAP document(s) have uncited sections. Use --strict to fail CI."
else
    echo "PASS: All documents meet citation density requirements."
fi
