#!/bin/bash
# hKask Documentation Link Checker
# Verifies all markdown links in docs/ directory are valid
# 
# Usage: ./check-links.sh [docs_dir]

set -e

DOCS_DIR="${1:-./docs}"
BROKEN_LINKS=()

echo "=== hKask Documentation Link Checker ==="
echo "Scanning: $DOCS_DIR"
echo ""

# Find all markdown files (excluding archive)
MARKDOWN_FILES=$(find "$DOCS_DIR" -name "*.md" -type f ! -path "*/archive/*" 2>/dev/null | wc -l)
echo "Found $MARKDOWN_FILES markdown files"
echo ""

echo "Checking links..."

for file in $(find "$DOCS_DIR" -name "*.md" -type f ! -path "*/archive/*"); do
    # Extract relative links (not http/https)
    while IFS= read -r link; do
        # Skip external links
        if [[ "$link" =~ ^https?:// ]] || [[ "$link" =~ ^mailto: ]] || [[ "$link" =~ ^# ]]; then
            continue
        fi
        
        # Remove anchor suffix if present
        link_path="${link%%#*}"
        
        # Skip empty links
        if [[ -z "$link_path" ]]; then
            continue
        fi
        
        # Resolve relative to current file's directory
        file_dir=$(dirname "$file")
        if [[ "$link_path" != /* ]]; then
            target="$file_dir/$link_path"
        else
            target="$DOCS_DIR$link_path"
        fi
        
        # Normalize path (simplified)
        target=$(realpath -m "$target" 2>/dev/null || echo "$target")
        
        # Check if target exists
        if [[ -n "$target" ]] && [[ ! -f "$target" ]]; then
            BROKEN_LINKS+=("$file -> $link_path")
        fi
    done < <(grep -oE '\]\([^)]+\)' "$file" 2>/dev/null | sed 's/\](/(/g' | tr -d '()' || true)
done

# Report results
echo ""
if [[ ${#BROKEN_LINKS[@]} -eq 0 ]]; then
    echo "✓ All links valid"
    exit 0
else
    echo "✗ Found ${#BROKEN_LINKS[@]} broken link(s):"
    for link in "${BROKEN_LINKS[@]}"; do
        echo "  - $link"
    done
    echo ""
    echo "Recommendation: Update or remove broken links above"
    exit 1
fi
