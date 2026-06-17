#!/usr/bin/env bash
# Deprecated annotation check — CI security invariant (P7 enforcement).
#
# Scans for #[deprecated] annotations in production code.
# Prohibition #2: Deprecated code earns deletion, not annotation.
#
# Exit 0 = clean. Exit 1 = violations found.

set -euo pipefail

echo "=== No Deprecated (P7) ==="

FAILED=0

for root in crates mcp-servers; do
    [ -d "$root" ] || continue

    while IFS= read -r -d '' file; do
        # Skip test files
        [[ "$file" == */tests/* ]] && continue

        violations=$(grep -n '#\[deprecated' "$file" 2>/dev/null || true)

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
    echo "FAIL: #[deprecated] annotations found. Delete deprecated code, don't annotate."
    exit 1
fi

echo ""
echo "PASS: No deprecated annotations."
exit 0
