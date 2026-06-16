#!/usr/bin/env bash
# No unwrap on hot paths — CI security invariant check.
#
# Scans CNS, agents, inference, services, and MCP server crates for
# .unwrap() calls that could panic at runtime. Test code excluded.
#
# Exit 0 = clean. Exit 1 = violations found.

set -euo pipefail

echo "=== No unwrap on hot paths ==="

violations=$(grep -rn '\.unwrap()' crates/hkask-cns/src crates/hkask-agents/src crates/hkask-inference/src crates/hkask-services/src crates/hkask-mcp/src crates/hkask-communication/src crates/hkask-condenser/src --include="*.rs" 2>/dev/null | grep -v "cfg(test)" | grep -v "/tests/" | grep -v "test_" | grep -v "#\[test\]" | grep -v "#\[tokio::test" | grep -v 'assert!(' | grep -v 'assert_eq!(' | grep -v 'panic!(' | grep -v '\.expect("must' | grep -v '\.expect("should' | grep -v '\.expect("test' || true)

for server_dir in mcp-servers/*/; do
    sv=$(grep -rn '\.unwrap()' "$server_dir/src" --include="*.rs" 2>/dev/null | grep -v "cfg(test)" | grep -v "/tests/" | grep -v "test_" | grep -v "#\[test\]" | grep -v "#\[tokio::test" | grep -v 'assert!(' | grep -v 'assert_eq!(' | grep -v 'panic!(' || true)
    violations="${violations}${violations:+$'\n'}${sv}"
done

if [ -n "$violations" ]; then
    echo "VIOLATIONS: .unwrap() on hot paths:"
    echo "$violations"
    exit 1
fi

echo "PASS: No .unwrap() on hot paths."
exit 0
