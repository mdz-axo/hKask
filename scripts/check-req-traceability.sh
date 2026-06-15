#!/usr/bin/env bash
# REQ traceability check — CI gate for Task 10 (Wave 6).
# Exit 0 always (trend monitor, not a hard gate).

set -euo pipefail

echo "=== REQ Traceability Check ==="
echo ""

total_tests=0
total_reqs=0

for crate_dir in crates/*/; do
    crate=$(basename "$crate_dir")
    src="${crate_dir}src"
    [ -d "$src" ] || continue

    test_count=$(grep -rchE '#\[test\]|#\[tokio::test\]' "$src" --include='*.rs' 2>/dev/null || true | awk '{sum+=$1} END {print sum+0}')
    test_count=${test_count:-0}
    req_count=$(grep -rch '// REQ:' "$src" --include='*.rs' 2>/dev/null || true | awk '{sum+=$1} END {print sum+0}')
    req_count=${req_count:-0}

    total_tests=$((total_tests + test_count))
    total_reqs=$((total_reqs + req_count))

    if [ "$test_count" -gt 0 ] && [ "$req_count" -eq 0 ]; then
        echo "  ⚠️  $crate: $test_count tests, 0 REQ tags"
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
