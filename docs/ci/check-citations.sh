#!/usr/bin/env bash
# Citation Audit — checks citation density across the documentation corpus.
#
# Per DOCUMENTATION_STANDARDS.md §5.3:
#   "Reviewers check by running `grep -c '\[\^' <document>.md` and
#    confirming ≥ 1 citation per `##`-level section."
#
# This script automates that check across all active docs (excluding archive).
#
# Usage:
#   ./docs/ci/check-citations.sh [--strict] [--verbose]
#
#   --strict   Exit with non-zero if any document fails the ≥1 citation/section rule.
#   --verbose  Show section-level detail for each document.
#
# Score interpretation:
#   PASS  — Every ## section has ≥1 [^...] footnote citation.
#   GAP   — One or more ## sections lack citations. The gap count is shown.
#   EXEMPT — Document has 0 total ## sections (typically frontmatter-only or indices).
#
# Output format (for CI consumption):
#   PASS|GAP|EXEMPT,N  <file>  (N = number of uncited sections)

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

TOTAL_DOCS=0
PASS_COUNT=0
GAP_COUNT=0
EXEMPT_COUNT=0
TOTAL_GAP_SECTIONS=0
HAS_FAILURES=false

echo "=== Citation Audit ==="
echo "Rule: Every ##-level section SHOULD have ≥1 [^...] footnote citation"
echo "Scope: All active markdown docs under docs/ (excluding archive/)"
echo "Reference: DOCUMENTATION_STANDARDS.md §5"
echo ""

audit_file() {
    local file="$1"
    local relative="${file#docs/}"
    TOTAL_DOCS=$((TOTAL_DOCS + 1))

    # Count ##-level sections
    local section_count
    section_count=$(grep -c '^## ' "$file" 2>/dev/null || echo 0)

    if [ "$section_count" -eq 0 ]; then
        echo "  EXEMPT,0  $relative  (no ## sections)"
        EXEMPT_COUNT=$((EXEMPT_COUNT + 1))
        return
    fi

    # Count total [^...] footnote citations in the file
    local citation_count
    citation_count=$(grep -c '\[\^' "$file" 2>/dev/null || echo 0)

    # Find ## sections that lack citations.
    # Strategy: extract each ## section, check if it contains [^...
    local uncited=0
    local uncited_sections=""

    # Split by ## headers using awk
    local current_section=""
    local section_name=""
    local in_section=false

    while IFS= read -r line; do
        if echo "$line" | grep -q '^## '; then
            # Check previous section
            if [ "$in_section" = true ] && [ -n "$current_section" ]; then
                if ! echo "$current_section" | grep -q '\[\^'; then
                    uncited=$((uncited + 1))
                    if [ "$VERBOSE" = true ]; then
                        uncited_sections="${uncited_sections}    - $section_name"$'\n'
                    fi
                fi
            fi
            section_name=$(echo "$line" | sed 's/^## //')
            current_section="$line"$'\n'
            in_section=true
        elif [ "$in_section" = true ]; then
            if echo "$line" | grep -q '^#\{1,3\} '; then
                # Hit a heading at another level — end current section
                if ! echo "$current_section" | grep -q '\[\^'; then
                    uncited=$((uncited + 1))
                    if [ "$VERBOSE" = true ]; then
                        uncited_sections="${uncited_sections}    - $section_name"$'\n'
                    fi
                fi
                section_name=$(echo "$line" | sed 's/^## //')
                current_section="$line"$'\n'
            else
                current_section="$current_section$line"$'\n'
            fi
        fi
    done < "$file"

    # Check last section
    if [ "$in_section" = true ] && [ -n "$current_section" ]; then
        if ! echo "$current_section" | grep -q '\[\^'; then
            uncited=$((uncited + 1))
            if [ "$VERBOSE" = true ]; then
                uncited_sections="${uncited_sections}    - $section_name"$'\n'
            fi
        fi
    fi

    TOTAL_GAP_SECTIONS=$((TOTAL_GAP_SECTIONS + uncited))

    if [ "$uncited" -eq 0 ]; then
        echo "  PASS,0    $relative  ($citation_count citations, $section_count sections)"
        PASS_COUNT=$((PASS_COUNT + 1))
    else
        echo "  GAP,$uncited   $relative  ($citation_count citations, $section_count sections, $uncited uncited)"
        if [ "$VERBOSE" = true ] && [ -n "$uncited_sections" ]; then
            printf '%s' "$uncited_sections"
        fi
        GAP_COUNT=$((GAP_COUNT + 1))
        HAS_FAILURES=true
    fi
}

# ── Audit all active docs ────────────────────────────────────────────────────

while IFS= read -r -d '' file; do
    audit_file "$file"
done < <(find docs -name '*.md' -not -path '*/archive/*' -not -name 'README.md' -print0 2>/dev/null | sort -z)

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo "=== Summary ==="
echo "  Total documents:    $TOTAL_DOCS"
echo "  PASS (all cited):   $PASS_COUNT"
echo "  GAP (uncited sections): $GAP_COUNT"
echo "  EXEMPT (no sections): $EXEMPT_COUNT"
echo "  Total uncited sections: $TOTAL_GAP_SECTIONS"
echo ""

if [ "$STRICT" = true ] && [ "$HAS_FAILURES" = true ]; then
    echo "FAIL: $GAP_COUNT document(s) have uncited sections."
    exit 1
elif [ "$HAS_FAILURES" = true ]; then
    echo "NOTE: $GAP_COUNT document(s) have uncited sections. Run with --strict to fail CI."
else
    echo "PASS: All documents meet citation density requirements."
fi
