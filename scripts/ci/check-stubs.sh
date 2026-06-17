#!/usr/bin/env bash
# Stub detection check — CI security invariant (P6 enforcement).
#
# Scans for todo!(), unimplemented!(), and stub markers in production code.
# Prohibition #2: No todo!(), unimplemented!(), #[deprecated], unused traits,
# stubs, or feature flags. Stubs are debt against the Generative Space.
#
# Exit 0 = clean. Exit 1 = violations found.

set -euo pipefail

echo "=== No Stubs (P6) ==="

FAILED=0

for root in crates mcp-servers; do
    [ -d "$root" ] || continue

    while IFS= read -r -d '' file; do
        # Skip test files
        [[ "$file" == */tests/* ]] && continue

        violations=$(grep -n 'todo!(.*)\|unimplemented!(.*)' "$file" 2>/dev/null || true)

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
    echo "FAIL: todo!() / unimplemented!() stubs found. Implement or remove."
    exit 1
fi

echo ""
echo "PASS: No stubs in production code."
exit 0
