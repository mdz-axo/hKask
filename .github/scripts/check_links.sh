#!/bin/bash
# Link Checker for hKask Documentation
# Validates all markdown links in the docs/ directory
# Usage: .github/scripts/check_links.sh

set -e

DOCS_DIR="${1:-docs}"
EXIT_CODE=0

echo "🔍 Checking links in $DOCS_DIR..."

# Find all markdown files excluding archive
MARKDOWN_FILES=$(find "$DOCS_DIR" -type f -name "*.md" ! -path "*/archive/*")

# Check for broken internal links
echo "Checking internal links..."
for file in $MARKDOWN_FILES; do
    # Extract links from markdown files
    LINKS=$(grep -oE '\[.*\]\([^)]+\)' "$file" 2>/dev/null | grep -oE '\([^)]+\)' | tr -d '()' || true)
    
    for link in $LINKS; do
        # Skip external links
        if [[ "$link" == http* ]]; then
            continue
        fi
        
        # Skip anchor-only links
        if [[ "$link" == \#* ]]; then
            continue
        fi
        
        # Resolve relative path
        DIR=$(dirname "$file")
        TARGET="$DIR/$link"
        
        # Normalize path
        TARGET=$(cd "$(dirname "$TARGET")" 2>/dev/null && pwd)/$(basename "$TARGET") 2>/dev/null || true
        
        # Check if target exists
        if [ ! -f "$TARGET" ]; then
            echo "❌ Broken link in $file: $link"
            EXIT_CODE=1
        fi
    done
done

# Check for TODO/FIXME comments
echo "Checking for TODO/FIXME comments..."
TODO_COUNT=$(grep -r "TODO\|FIXME" "$DOCS_DIR" --include="*.md" ! -path "*/archive/*" 2>/dev/null | wc -l || echo "0")
if [ "$TODO_COUNT" -gt 0 ]; then
    echo "⚠️  Found $TODO_COUNT TODO/FIXME comments in documentation"
    grep -n "TODO\|FIXME" "$DOCS_DIR" -r --include="*.md" ! -path "*/archive/*" 2>/dev/null || true
fi

# Summary
if [ $EXIT_CODE -eq 0 ]; then
    echo "✅ All links are valid"
else
    echo "❌ Found broken links"
fi

exit $EXIT_CODE
