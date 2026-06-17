#!/usr/bin/env bash
# Documentation link checker — CI gate for docs workflow.
#
# Scans markdown files in docs/ for broken internal cross-references.
# Checks that linked .md files exist within the repository.
#
# Exit 0 = all links valid. Exit 1 = broken links found.

set -euo pipefail

echo "=== Documentation Link Check ==="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
FAILED=0

cd "$REPO_ROOT"

while IFS= read -r -d '' mdfile; do
    # Extract relative markdown links: [text](path/to/file.md)
    links=$(grep -oP '\[.*?\]\(([^)]+\.md)\)' "$mdfile" 2>/dev/null | grep -oP '(?<=\()[^)]+\.md(?=\))' || true)

    if [ -z "$links" ]; then
        continue
    fi

    while IFS= read -r link; do
        # Skip external URLs
        [[ "$link" =~ ^https?:// ]] && continue

        # Resolve relative to the file's directory
        filedir=$(dirname "$mdfile")
        target="$filedir/$link"

        if [ ! -f "$target" ]; then
            echo "  ❌ $mdfile → $link (not found)"
            FAILED=1
        fi
    done <<< "$links"
done < <(find docs/ -name '*.md' -not -path '*/target/*' -print0 2>/dev/null)

if [ "$FAILED" -eq 1 ]; then
    echo ""
    echo "FAIL: Broken internal documentation links found."
    exit 1
fi

echo ""
echo "PASS: All internal documentation links valid."
exit 0
