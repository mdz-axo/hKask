#!/usr/bin/env bash
# REQ traceability check — CI gate for Task 10 (Wave 6).
#
# Counts REQ-tagged tests per crate and reports crates with zero REQ coverage.
# This is a trend monitor — it doesn't fail on low coverage, but flags crates
# that have public seams with no behavioral test traceability.
#
# Exit 0 always (trend monitor, not a hard gate).

set -euo pipefail

echo "=== REQ Traceability Check ==="
echo ""

total_tests=0
total_reqs=0

for crate_dir in crates/*/; do
    crate=$(basename "$crate_dir")
    src="${crate_dir}src"

    if [ ! -d "$src" ]; then
        continue
    fi

    # Count test functions and REQ tags
    test_count=0
    req_count=0
    for f in $(find "$src" -name '*.rs' -not -path '*/target/*' 2>/dev/null); do
        tc=$(grep -cE '#\[test\]|#\[tokio::test\]' "$f" 2>/dev/null || echo 0)
        rc=$(grep -c '// REQ:' "$f" 2>/dev/null || echo 0)
        # grep -cE may return multiple counts; take the first
        tc=$(echo "$tc" | head -1)
        rc=$(echo "$rc" | head -1)
        test_count=$((test_count + tc))
        req_count=$((req_count + rc))
    done

    total_tests=$((total_tests + test_count))
    total_reqs=$((total_reqs + req_count))

    if [ "$test_count" -gt 0 ] && [ "$req_count" -eq 0 ]; then
        echo "  ⚠️  $crate: $test_count tests, 0 REQ tags — no traceability"
    elif [ "$test_count" -gt 0 ]; then
        echo "  ✅ $crate: $test_count tests, $req_count REQ tags"
    else
        echo "  ℹ️  $crate: no tests"
    fi
done

echo ""
echo "Totals: $total_tests tests, $total_reqs REQ tags across all crates"
echo ""
echo "PASS: REQ traceability scan complete."
exit 0
