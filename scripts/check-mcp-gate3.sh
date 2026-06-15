#!/usr/bin/env bash
# MCP startup Gate-3 consistency check — CI gate for Task 10 (Wave 6).
#
# Verifies that all MCP servers call verify_startup_gates() at startup,
# enforcing the P4 three-gate pattern (auth → assignment → capability).
# This prevents regression of Wave 1's security boundary work.
#
# Exit 0 = all MCP servers have Gate-3 verification.
# Exit 1 = one or more MCP servers lack Gate-3 verification.

set -euo pipefail

FAILED=0

echo "=== MCP Startup Gate-3 Consistency Check ==="
echo ""

for server_dir in mcp-servers/*/; do
    server=$(basename "$server_dir")
    main="${server_dir}src/main.rs"

    if [ ! -f "$main" ]; then
        continue
    fi

    if grep -q 'verify_startup_gates' "$main"; then
        echo "  ✅ $server — verify_startup_gates() called at startup"
    else
        echo "  ❌ $server — MISSING verify_startup_gates() at startup"
        FAILED=1
    fi
done

echo ""
if [ "$FAILED" -eq 1 ]; then
    echo "FAIL: One or more MCP servers lack Gate-3 startup verification."
    echo "Add verify_startup_gates() call per Wave 1 / PR 1.2 pattern."
    exit 1
else
    echo "PASS: All MCP servers enforce Gate-3 startup verification."
    exit 0
fi
