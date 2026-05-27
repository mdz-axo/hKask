#!/bin/bash
# hKask Documentation Metadata Checker
# Verifies all markdown files have proper YAML frontmatter headers
#
# Usage: ./check-metadata.sh [docs_dir]

DOCS_DIR="${1:-./docs}"
MISSING_METADATA=()
VALID_COUNT=0

echo "=== hKask Documentation Metadata Checker ==="
echo "Scanning: $DOCS_DIR"
echo ""

# Find all markdown files (excluding archive directory)
for file in $(find "$DOCS_DIR" -name "*.md" -type f ! -path "*/archive/*" | sort); do
    # Check if file starts with YAML frontmatter
    first_line=$(head -n 1 "$file" 2>/dev/null || echo "")
    
    if [[ "$first_line" == "---" ]]; then
        # Check for required fields
        has_title=$(grep -c "^title:" "$file" 2>/dev/null || echo "0")
        has_version=$(grep -c "^version:" "$file" 2>/dev/null || echo "0")
        has_status=$(grep -c "^status:" "$file" 2>/dev/null || echo "0")
        has_last_updated=$(grep -c "^last_updated:" "$file" 2>/dev/null || echo "0")
        
        if [[ "$has_title" -gt 0 && "$has_version" -gt 0 && "$has_status" -gt 0 && "$has_last_updated" -gt 0 ]]; then
            ((VALID_COUNT++))
        else
            MISSING_METADATA+=("$file (incomplete frontmatter)")
        fi
    else
        MISSING_METADATA+=("$file (no frontmatter)")
    fi
done

# Report results
TOTAL=$(find "$DOCS_DIR" -name "*.md" -type f ! -path "*/archive/*" | wc -l)
echo "Total files scanned: $TOTAL"
echo "Files with valid metadata: $VALID_COUNT"
echo "Files missing/incomplete metadata: ${#MISSING_METADATA[@]}"
echo ""

if [[ ${#MISSING_METADATA[@]} -eq 0 ]]; then
    echo "✓ All files have valid metadata"
    exit 0
else
    echo "✗ Files needing metadata updates:"
    for file in "${MISSING_METADATA[@]}"; do
        echo "  - $file"
    done
    echo ""
    echo "Required frontmatter fields:"
    echo "  - title"
    echo "  - version"
    echo "  - status"
    echo "  - last_updated"
    echo ""
    echo "Example:"
    echo "---"
    echo "title: \"Document Title\""
    echo "audience: [architects, developers]"
    echo "last_updated: YYYY-MM-DD"
    echo "togaf_phase: \"Phase\""
    echo "version: \"X.Y.Z\""
    echo "status: \"Active|Draft|Deprecated\""
    echo "domain: \"Domain\""
    echo "---"
    exit 1
fi
