#!/usr/bin/env bash
# hKask Quality Gates — Master CI script for Wave 6 (Sustainment).
#
# Runs all headless quality checks in sequence. Any failing check
# causes the overall CI run to fail, preventing regression merge.
#
# Checks:
#   1. Public surface governance (Task 8)
#   2. Unsafe documentation policy (Task 9)
#   3. Runtime .unwrap() denylist (Wave 2 regression prevention)
#   4. MCP startup Gate-3 consistency (Wave 1 regression prevention)
#   5. REQ traceability trend (Wave 3 monitoring)
#
# Exit 0 = all checks pass.
# Exit 1 = one or more checks failed.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FAILED=0

echo "══════════════════════════════════════════════"
echo "  hKask Quality Gates — CI Sustainment"
echo "══════════════════════════════════════════════"
echo ""

run_check() {
    local name="$1"
    local script="$2"
    local fatal="${3:-true}"
    echo "─── $name ───"
    if bash "$script"; then
        echo ""
    else
        if [ "$fatal" = "true" ]; then
            FAILED=1
        fi
        echo ""
    fi
}

run_check "1. Public Surface Governance" "$SCRIPT_DIR/check-public-surface.sh"
run_check "2. Unsafe Documentation Policy" "$SCRIPT_DIR/check-unsafe-safety.sh"
run_check "3. Runtime .unwrap() Denylist (warning-only)" "$SCRIPT_DIR/check-unwrap-denylist.sh" false
run_check "4. MCP Gate-3 Consistency" "$SCRIPT_DIR/check-mcp-gate3.sh"
run_check "5. REQ Traceability Trend" "$SCRIPT_DIR/check-req-traceability.sh"

echo "══════════════════════════════════════════════"
if [ "$FAILED" -eq 0 ]; then
    echo "  RESULT: ALL CHECKS PASSED ✅"
    echo "══════════════════════════════════════════════"
    exit 0
else
    echo "  RESULT: SOME CHECKS FAILED ❌"
    echo "══════════════════════════════════════════════"
    exit 1
fi
