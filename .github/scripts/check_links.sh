#!/bin/bash
# Link Checker for hKask Documentation
# Validates all markdown links in docs/ directory

set -e

DOCS_DIR="docs"
ERRORS=0

echo "=== hKask Documentation Link Checker ==="
echo "Scanning: ${DOCS_DIR}/"
echo ""

# Find all markdown files (excluding archive)
FILES=$(find ${DOCS_DIR} -name "*.md" ! -path "docs/archive/*" -type f)

for file in ${FILES}; do
    # Extract all markdown links [text](url)
    LINKS=$(grep -oE '\[.*\]\([^)]+\)' "$file" 2>/dev/null || true)
    
    if [ -z "$LINKS" ]; then
        continue
    fi
    
    # Check each link
    echo "$LINKS" | while read -r link; do
        # Extract URL from markdown link
        URL=$(echo "$link" | grep -oE '\([^)]+\)' | tr -d '()')
        
        # Skip external URLs (http/https/mailto)
        if [[ "$URL" =~ ^https?:// ]] || [[ "$URL" =~ ^mailto: ]]; then
            continue
        fi
        
        # Skip anchor-only links
        if [[ "$URL" =~ ^# ]]; then
            continue
        fi
        
        # Resolve relative path
        FILE_DIR=$(dirname "$file")
        if [[ ! "$URL" =~ ^/ ]]; then
            TARGET="${FILE_DIR}/${URL}"
        else
            TARGET="${DOCS_DIR}${URL}"
        fi
        
        # Normalize path (remove ../)
        TARGET=$(realpath -m "$TARGET" 2>/dev/null || echo "$TARGET")
        
        # Check if target exists
        if [ ! -f "$TARGET" ]; then
            echo "❌ Broken link in ${file}: ${URL}"
            ERRORS=$((ERRORS + 1))
        fi
    done
done

if [ ${ERRORS} -eq 0 ]; then
    echo "✅ All internal links are valid"
    exit 0
else
    echo ""
    echo "Found ${ERRORS} broken link(s)"
    exit 1
fi
