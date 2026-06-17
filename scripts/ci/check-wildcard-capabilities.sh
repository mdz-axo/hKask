#!/usr/bin/env bash
# Wildcard capability check — CI security invariant.
#
# Scans for capability definitions using wildcard patterns (e.g., "*", "all")
# that violate the Principle of Least Privilege (P4).
#
# Exit 0 = clean. Exit 1 = violations found.

set -euo pipefail

echo "=== No Wildcard Capabilities ==="

FAILED=0

for root in crates mcp-servers; do
    [ -d "$root" ] || continue

    while IFS= read -r -d '' file; do
        # Skip test files
        [[ "$file" == */tests/* ]] && continue

        violations=$(grep -n 'capability.*"\*"\|capability.*"all"\|capability.*::all\|scopes.*"\*"' "$file" 2>/dev/null || true)

        if [ -n "$violations" ]; then
            while IFS= read -r vline; do
                echo "  ❌ $file:$vline"
                FAILED=1
            done <<< "$violations"
        fi
    done < <(find "$root" -name '*.rs' -print0 2>/dev/null)
done

if [ "$FAILED" -eq 1 ]; then
    echo ""
    echo "FAIL: Wildcard capability patterns found. Use explicit capability names."
    exit 1
fi

echo ""
echo "PASS: No wildcard capabilities."
exit 0
