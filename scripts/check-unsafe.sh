#!/usr/bin/env bash
# No undocumented unsafe blocks — CI security invariant check.
#
# Every `unsafe` block in production code must carry a `// SAFETY:` comment
# explaining why the invariants are upheld. Test code excluded.
#
# Exit 0 = clean. Exit 1 = undocumented unsafe found.

set -euo pipefail

echo "=== No undocumented unsafe blocks ==="

FAILED=0

for root in crates mcp-servers; do
    [ -d "$root" ] || continue

    while IFS= read -r -d '' file; do
        # Skip test files
        echo "$file" | grep -q '/tests/\|test_' && continue

        unsafe_lines=$(grep -n 'unsafe\s*{\|unsafe\s\+fn' "$file" 2>/dev/null || true)
        [ -z "$unsafe_lines" ] && continue

        while IFS= read -r uline; do
            line_num=$(echo "$uline" | cut -d: -f1)
            start=$((line_num > 3 ? line_num - 3 : 1))
            if ! sed -n "${start},${line_num}p" "$file" | grep -q 'SAFETY:'; then
                echo "  UNDOCUMENTED: $file:$line_num"
                FAILED=1
            fi
        done <<< "$unsafe_lines"
    done < <(find "$root" -name '*.rs' -print0 2>/dev/null)
done

if [ "$FAILED" -eq 1 ]; then
    echo "FAIL: Undocumented unsafe blocks found."
    exit 1
fi

echo "PASS: All unsafe blocks documented (or none exist)."
exit 0
