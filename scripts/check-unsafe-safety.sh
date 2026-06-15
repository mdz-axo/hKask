#!/usr/bin/env bash
# Unsafe documentation policy check — CI gate for Task 9 (Wave 5).
#
# Every non-test `unsafe` block must have a proximate `SAFETY:` comment
# explaining why the unsafe operation is sound. This check flags any
# `unsafe {` that lacks a `SAFETY:` comment within the preceding 5 lines.
#
# Test-only unsafe (env var manipulation in test helpers) is excluded
# from this check — those are covered by the test harness isolation.
#
# Exit 0 = all non-test unsafe blocks have SAFETY: comments.
# Exit 1 = one or more non-test unsafe blocks lack SAFETY: comments.

set -euo pipefail

FAILED=0

echo "=== Unsafe Documentation Policy Check ==="
echo ""

# Find all .rs files in crates/ (excluding target/)
while IFS= read -r file; do
    # Skip test modules and test files
    if echo "$file" | grep -qE '/(test|tests)/'; then
        continue
    fi

    # Extract unsafe blocks with context (5 lines before)
    # We look for `unsafe {` that is NOT preceded by `SAFETY:` within 5 lines
    line_num=0
    buffer=()
    in_test_module=false

    while IFS= read -r line; do
        line_num=$((line_num + 1))
        buffer+=("$line")
        if [ ${#buffer[@]} -gt 6 ]; then
            buffer=("${buffer[@]:1}")
        fi

        # Track test module boundaries
        if echo "$line" | grep -qE '#\[cfg\(test\)\]|mod tests \{'; then
            in_test_module=true
        fi
        if [ "$in_test_module" = true ] && echo "$line" | grep -qE '^\}'; then
            in_test_module=false
        fi

        # Check for unsafe block
        if echo "$line" | grep -qE 'unsafe \{'; then
            if [ "$in_test_module" = true ]; then
                continue
            fi

            # Check if any of the preceding 5 lines contain SAFETY:
            has_safety=false
            for buf_line in "${buffer[@]}"; do
                if echo "$buf_line" | grep -qE 'SAFETY:'; then
                    has_safety=true
                    break
                fi
            done

            # Also check the current line for inline SAFETY comment
            if echo "$line" | grep -qE '// SAFETY:'; then
                has_safety=true
            fi

            if [ "$has_safety" = false ]; then
                echo "  ❌ $file:$line_num — unsafe block without SAFETY: comment"
                echo "     $line"
                FAILED=1
            fi
        fi
    done < "$file"
done < <(find crates/ -name '*.rs' -not -path '*/target/*')

echo ""
if [ "$FAILED" -eq 1 ]; then
    echo "FAIL: One or more non-test unsafe blocks lack SAFETY: documentation."
    echo "Add a // SAFETY: comment within 5 lines before each unsafe { block."
    exit 1
else
    echo "PASS: All non-test unsafe blocks have SAFETY: documentation."
    exit 0
fi
