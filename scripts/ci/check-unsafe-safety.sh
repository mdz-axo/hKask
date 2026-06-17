#!/usr/bin/env bash
# Unsafe documentation policy check — CI gate for Task 9 (Wave 5).
#
# Verifies that every `unsafe` block in production code carries a
# `// SAFETY:` comment explaining why the invariants are upheld.
# This enforces the Rust unsafe documentation policy.
#
# Exit 0 = all unsafe blocks have safety documentation.
# Exit 1 = one or more unsafe blocks lack safety documentation.

set -euo pipefail

FAILED=0

echo "=== Unsafe Documentation Policy Check ==="
echo ""

# Scan crates/ and mcp-servers/ for unsafe blocks without SAFETY comments
for root in crates mcp-servers; do
    if [ ! -d "$root" ]; then
        continue
    fi

    while IFS= read -r -d '' file; do
        # Skip test files
        if echo "$file" | grep -q '/tests/\|test_'; then
            continue
        fi

        # Find lines with `unsafe {` or `unsafe fn`
        unsafe_lines=$(grep -n 'unsafe\s*{\|unsafe\s\+fn' "$file" 2>/dev/null || true)
        if [ -z "$unsafe_lines" ]; then
            continue
        fi

        while IFS= read -r uline; do
            line_num=$(echo "$uline" | cut -d: -f1)
            # Check the 3 lines above for a SAFETY comment
            start=$((line_num > 3 ? line_num - 3 : 1))
            has_safety=$(sed -n "${start},${line_num}p" "$file" | grep -q 'SAFETY:' && echo "yes" || echo "no")

            if [ "$has_safety" = "no" ]; then
                echo "  ❌ $file:$line_num — unsafe block missing // SAFETY: comment"
                FAILED=1
            fi
        done <<< "$unsafe_lines"
    done < <(find "$root" -name '*.rs' -print0 2>/dev/null)
done

echo ""
if [ "$FAILED" -eq 1 ]; then
    echo "FAIL: One or more unsafe blocks lack safety documentation."
    echo "Add a // SAFETY: comment above each unsafe block explaining why invariants hold."
    exit 1
else
    echo "PASS: All unsafe blocks have safety documentation (or no unsafe blocks exist)."
    exit 0
fi
