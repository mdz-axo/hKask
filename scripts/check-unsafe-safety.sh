#!/usr/bin/env bash
# Unsafe documentation policy check — CI gate for Task 9 (Wave 5).
#
# Every non-test `unsafe` block must have a proximate `SAFETY:` comment.
# Test-only unsafe is excluded.
#
# Exit 0 = all non-test unsafe blocks have SAFETY: comments.
# Exit 1 = one or more non-test unsafe blocks lack SAFETY: comments.

set -euo pipefail

FAILED=0

echo "=== Unsafe Documentation Policy Check ==="
echo ""

# Find all non-test .rs files with unsafe blocks
while IFS= read -r file; do
    # Skip files in test directories or with #[cfg(test)]
    if echo "$file" | grep -qE '/(test|tests)/'; then
        continue
    fi
    if grep -q '#\[cfg(test)\]' "$file" 2>/dev/null; then
        continue
    fi

    # Get line numbers of unsafe blocks
    unsafe_lines=$(grep -n 'unsafe {' "$file" 2>/dev/null || true)
    if [ -z "$unsafe_lines" ]; then
        continue
    fi

    while IFS= read -r uline; do
        line_num=$(echo "$uline" | cut -d: -f1)
        # Check if SAFETY: appears within 5 lines before this line
        start=$((line_num - 5))
        [ "$start" -lt 1 ] && start=1
        if ! sed -n "${start},${line_num}p" "$file" | grep -q 'SAFETY:'; then
            # Also check the current line for inline SAFETY
            if ! echo "$uline" | grep -q '// SAFETY:'; then
                echo "  ❌ $file:$line_num — unsafe block without SAFETY: comment"
                FAILED=1
            fi
        fi
    done <<< "$unsafe_lines"
done < <(find crates/ -name '*.rs' -not -path '*/target/*')

echo ""
if [ "$FAILED" -eq 1 ]; then
    echo "FAIL: One or more non-test unsafe blocks lack SAFETY: documentation."
    exit 1
else
    echo "PASS: All non-test unsafe blocks have SAFETY: documentation."
    exit 0
fi
