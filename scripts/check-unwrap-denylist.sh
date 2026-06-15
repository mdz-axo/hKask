#!/usr/bin/env bash
# Runtime .unwrap() denylist check — CI gate for Task 10 (Wave 6).
#
# Flags any `.unwrap()` call in non-test runtime code for selected crates.
# Test code (inside #[cfg(test)] modules or #[test] functions) is excluded.
#
# This prevents regression of Wave 2's .unwrap() elimination work.
#
# Exit 0 = no runtime .unwrap() calls in targeted crates.
# Exit 1 = one or more runtime .unwrap() calls found.

set -euo pipefail

FAILED=0

# Crates targeted by Wave 2 .unwrap() elimination
TARGET_CRATES=(
    "hkask-mcp-media"
    "hkask-mcp-docproc"
    "hkask-templates"
)

echo "=== Runtime .unwrap() Denylist Check ==="
echo "Target crates: ${TARGET_CRATES[*]}"
echo ""

for crate in "${TARGET_CRATES[@]}"; do
    # Find the crate source directory
    if [ -d "crates/$crate" ]; then
        src_dir="crates/$crate/src"
    elif [ -d "mcp-servers/$crate" ]; then
        src_dir="mcp-servers/$crate/src"
    else
        echo "  ⚠️  $crate: crate directory not found — skipping"
        continue
    fi

    # Find .unwrap() in .rs files, excluding test-only files
    # A file is considered test-only if it contains #[cfg(test)] or is in a /test/ directory
    violations=""
    while IFS= read -r file; do
        # Skip files that contain #[cfg(test)] (test modules)
        if grep -q '#\[cfg(test)\]' "$file" 2>/dev/null; then
            continue
        fi
        # Check this file for .unwrap() outside test functions
        file_violations=$(grep -n '\.unwrap()' "$file" 2>/dev/null | grep -v '#\[test\]' | grep -v '#\[tokio::test\]' || true)
        if [ -n "$file_violations" ]; then
            while IFS= read -r vline; do
                violations="${violations}${file}:${vline}"$'\n'
            done <<< "$file_violations"
        fi
    done < <(find "$src_dir" -name '*.rs' -not -path '*/target/*')

    if [ -n "$violations" ]; then
        echo "  ❌ $crate — runtime .unwrap() calls found:"
        echo "$violations" | while IFS= read -r line; do
            echo "     $line"
        done
        FAILED=1
    else
        echo "  ✅ $crate — zero runtime .unwrap() calls"
    fi
done

echo ""
if [ "$FAILED" -eq 1 ]; then
    echo "FAIL: Runtime .unwrap() calls detected in targeted crates."
    echo "Replace with .expect(), .map_err(), or span.internal_error() per Wave 2 patterns."
    exit 1
else
    echo "PASS: No runtime .unwrap() calls in targeted crates."
    exit 0
fi
