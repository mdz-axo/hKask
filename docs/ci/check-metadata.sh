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
    "mds_categories:"
)

VALID_STATUSES=("Active" "Draft" "Deprecated" "Superseded")
VALID_MDS_CATEGORIES=("domain" "composition" "trust" "lifecycle" "curation")

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

    # Skip handoff files (transient, no frontmatter required per HANDOFF_LIFECYCLE.md §4)
    if [[ "$file" == *"/docs/handoffs/"* ]]; then
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
        continue
    fi

    # Check status is a valid lifecycle state
    doc_status=$(grep "^status:" "$file" | head -1 | sed 's/^status: *//' | tr -d '"' | xargs)
    valid_status=false
    for vs in "${VALID_STATUSES[@]}"; do
        if [[ "$doc_status" == "$vs" ]]; then
            valid_status=true
            break
        fi
    done
    if [[ "$valid_status" == false ]]; then
        echo -e "${YELLOW}[WARN]${NC} $rel_file → status: '$doc_status' is not a recognized lifecycle state (valid: ${VALID_STATUSES[*]})"
        WARNINGS=$((WARNINGS + 1))
    fi

    # Check mds_categories use valid 5-category taxonomy
    if grep -q "^mds_categories:" "$file"; then
        cats=$(grep "^mds_categories:" "$file" | head -1 | sed 's/^mds_categories: *//' | tr -d '[]"' | tr ',' ' ' | xargs)
        for cat in $cats; do
            valid_cat=false
            for vc in "${VALID_MDS_CATEGORIES[@]}"; do
                if [[ "$cat" == "$vc" ]]; then
                    valid_cat=true
                    break
                fi
            done
            if [[ "$valid_cat" == false ]]; then
                echo -e "${YELLOW}[WARN]${NC} $rel_file → mds_categories: '$cat' is not a valid MDS category (valid: ${VALID_MDS_CATEGORIES[*]})"
                WARNINGS=$((WARNINGS + 1))
            fi
        done
    fi

    # Flag deprecated ddmvss_categories usage
    if grep -q "ddmvss_categories:" "$file"; then
        echo -e "${RED}[VIOLATION]${NC} $rel_file → uses deprecated ddmvss_categories (migrate to mds_categories)"
        ERRORS=$((ERRORS + 1))
    fi

done < <(find "$DOCS_DIR" -name "*.md" -not -path "*/archive/*" -print0)

echo ""
echo "=== Results ==="
echo "Documents checked: $TOTAL"
echo -e "Missing metadata: ${RED}$ERRORS${NC}"
echo -e "Warnings: ${YELLOW}$WARNINGS${NC}"

if [[ $ERRORS -gt 0 ]]; then
    echo -e "${RED}FAIL: $ERRORS document(s) missing required metadata.${NC}"
    exit 1
else
    echo -e "${GREEN}PASS: All documents have required metadata.${NC}"
    exit 0
fi
