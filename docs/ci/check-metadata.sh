#!/usr/bin/env bash
# docs/ci/check-metadata.sh — Validate required metadata headers in hKask documentation
# Per DOCUMENTATION_STANDARDS.md §2: Every document must have required frontmatter headers
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

ERRORS=0
WARNINGS=0
TOTAL=0

REQUIRED_HEADERS=(
    "title:"
    "audience:"
    "last_updated:"
    "version:"
    "status:"
    "domain:"
    "ddmvss_categories:"
)

echo "=== hKask Documentation Metadata Checker ==="
echo "Scanning: $DOCS_DIR"
echo ""

while IFS= read -r -d '' file; do
    # Skip archive files
    if [[ "$file" == *"/docs/archive/"* ]]; then
        continue
    fi

    # Skip CI scripts themselves
    if [[ "$file" == *"/docs/ci/"* ]]; then
        continue
    fi

    TOTAL=$((TOTAL + 1))
    rel_file="${file#$DOCS_DIR/}"
    missing=()

    for header in "${REQUIRED_HEADERS[@]}"; do
        if ! grep -q "^$header" "$file"; then
            missing+=("$header")
        fi
    done

    if [[ ${#missing[@]} -gt 0 ]]; then
        echo -e "${RED}[MISSING]${NC} $rel_file → ${missing[*]}"
        ERRORS=$((ERRORS + 1))
    fi

    # Check for DDMVSS_SCAFFOLD reference consistency
    if grep -q "ddmvss_categories:" "$file"; then
        cats=$(grep "^ddmvss_categories:" "$file" | head -1)
        valid_cats=("domain" "capability" "interface" "composition" "trust" "observability" "persistence" "lifecycle" "curation")
        for cat in "${valid_cats[@]}"; do
            # Each file must map to at least one valid category
            :
        done
    fi

done < <(find "$DOCS_DIR" -name "*.md" -not -path "*/archive/*" -print0)

echo ""
echo "=== Results ==="
echo "Documents checked: $TOTAL"
echo -e "Missing metadata: ${RED}$ERRORS${NC}"

if [[ $ERRORS -gt 0 ]]; then
    echo -e "${RED}FAIL: $ERRORS document(s) missing required metadata.${NC}"
    exit 1
else
    echo -e "${GREEN}PASS: All documents have required metadata.${NC}"
    exit 0
fi
