#!/usr/bin/env bash
# docs/ci/check-links.sh — Validate internal cross-references in hKask documentation
# Per DOCUMENTATION_STANDARDS.md §10: Zero broken links (excluding intentional placeholders)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$DOCS_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0
WARNINGS=0
CHECKED=0

# Intentional placeholders (links that are known to point to future deliverables)
INTENTIONAL_PLACEHOLDERS=(
    "docs/specifications/cns-protocol-reference.md"  # Deferred
    ".github/scripts/check_links.sh"                 # External CI
)

echo "=== hKask Documentation Link Checker ==="
echo "Scanning: $DOCS_DIR"
echo ""

# Find all markdown files (excluding archive/)
while IFS= read -r -d '' file; do
    # Skip archive files
    if [[ "$file" == *"/docs/archive/"* ]]; then
        continue
    fi

    # Extract relative path for reporting
    rel_file="${file#$PROJECT_ROOT/}"

    # Extract markdown links: [text](path)
    while IFS= read -r link_line; do
        # Extract the path portion of [text](path) or [text](path#anchor)
        target=$(echo "$link_line" | grep -oP '\]\([^)]+\)' | sed 's/^\](//;s/)$//' | cut -d'#' -f1)

        if [[ -z "$target" ]]; then
            continue
        fi

        # Skip external URLs
        if [[ "$target" =~ ^https?:// ]]; then
            continue
        fi
        # Skip email links
        if [[ "$target" =~ ^mailto: ]]; then
            continue
        fi
        # Skip empty
        if [[ "$target" == "" ]]; then
            continue
        fi

        CHECKED=$((CHECKED + 1))

        # Resolve the target path relative to the file's directory
        file_dir="$(dirname "$file")"
        if [[ -f "$file_dir/$target" ]] || [[ -d "$file_dir/$target" ]]; then
            continue  # OK
        fi

        # Also try relative to DOCS_DIR
        rel_target="$target"
        if [[ "$rel_target" == /* ]]; then
            # Absolute within project — strip leading / and check
            rel_target="${rel_target#/}"
        fi

        # Combine: if target starts with ../, resolve from file_dir first
        # Otherwise try from DOCS_DIR
        resolved_from_docs="$DOCS_DIR/$rel_target"

        if [[ -f "$resolved_from_docs" ]] || [[ -d "$resolved_from_docs" ]]; then
            continue  # OK
        fi

        # Check if this is an intentional placeholder
        is_placeholder=false
        for placeholder in "${INTENTIONAL_PLACEHOLDERS[@]}"; do
            if [[ "$target" == "$placeholder" ]] || [[ "$rel_target" == "$placeholder" ]]; then
                is_placeholder=true
                break
            fi
        done

        if [[ "$is_placeholder" == true ]]; then
            echo -e "${YELLOW}[PLACEHOLDER]${NC} $rel_file → $target"
            WARNINGS=$((WARNINGS + 1))
        else
            echo -e "${RED}[BROKEN]${NC} $rel_file → $target"
            ERRORS=$((ERRORS + 1))
        fi
    done < <(grep -noP '\[[^\]]+\]\([^)]+\)' "$file" 2>/dev/null || true)

done < <(find "$DOCS_DIR" -name "*.md" -not -path "*/archive/*" -print0)

echo ""
echo "=== Results ==="
echo "Links checked: $CHECKED"
echo -e "Broken: ${RED}$ERRORS${NC}"
echo -e "Placeholders: ${YELLOW}$WARNINGS${NC}"

if [[ $ERRORS -gt 0 ]]; then
    echo -e "${RED}FAIL: $ERRORS broken link(s) found.${NC}"
    exit 1
else
    echo -e "${GREEN}PASS: No broken links.${NC}"
    exit 0
fi
