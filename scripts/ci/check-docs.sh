#!/usr/bin/env bash
# Documentation link checker — CI gate for docs workflow.
#
# Scans markdown files in docs/ for broken internal cross-references.
# Checks that linked .md files exist within the repository.
# Excludes archive/ directory (historical docs with known relative-link drift).
#
# Exit 0 = all links valid. Exit 1 = broken links found.

set -euo pipefail

echo "=== Documentation Link Check ==="

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
FAILED=0

cd "$REPO_ROOT"

extract_path() {
    # Extract the path portion from a markdown link like [text](path.md)
    # Input: [text](path/to/file.md) or [text](path/to/file.md
    echo "$1" | grep -oP '(?<=\()[^)]*\.md' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//'
}

while IFS= read -r -d '' mdfile; do
    # Find all markdown links to .md files
    # Well-formed:  [text](path.md)
    # Malformed:    [text](path.md  (missing closing paren)
    # Excludes: [ ] and [x] checkbox patterns
    raw_links=$(grep -oP '\[(?!\s|\]|x\])[^\]]*\]\([^)]*\.md\)?' "$mdfile" 2>/dev/null || true)

    if [ -z "$raw_links" ]; then
        continue
    fi

    while IFS= read -r raw_link; do
        link=$(extract_path "$raw_link")
        [ -z "$link" ] && continue

        # Skip external URLs
        [[ "$link" =~ ^https?:// ]] && continue
        # Skip anchor-only links
        [[ "$link" =~ ^# ]] && continue

        # Resolve relative to the file's directory
        filedir=$(dirname "$mdfile")
        target="$filedir/$link"

        if [ ! -f "$target" ]; then
            echo "  ❌ $mdfile → $link (not found)"
            FAILED=1
        fi
    done <<< "$raw_links"
done < <(find docs/ -name '*.md' -not -path '*/target/*' -not -path '*/archive/*' -print0 2>/dev/null)

if [ "$FAILED" -eq 1 ]; then
    echo ""
    echo "FAIL: Broken internal documentation links found."
    exit 1
fi

echo ""
echo "PASS: All internal documentation links valid."
exit 0
