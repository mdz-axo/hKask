#!/usr/bin/env bash
# Runtime .unwrap() denylist — CI gate for Wave 2 regression prevention.
#
# Flags any .unwrap() or .expect() calls on hot paths (CNS, agents, inference,
# services, MCP servers) that could panic at runtime. These should use proper
# error handling (Result propagation, .ok()/or_else(), or explicit match).
#
# Test code and build scripts are excluded.
#
# Exit 0 = no unwrap/expect on hot paths.
# Exit 1 = one or more unwrap/expect calls found on hot paths.

set -euo pipefail

FAILED=0

echo "=== Runtime .unwrap() Denylist Check ==="
echo ""

# Hot-path crates where panics are unacceptable
HOT_CRATES=(
    "hkask-cns"
    "hkask-agents"
    "hkask-inference"
    "hkask-services"
    "hkask-mcp"
    "hkask-communication"
    "hkask-condenser"
)

for crate in "${HOT_CRATES[@]}"; do
    crate_dir="crates/${crate}"
    if [ ! -d "$crate_dir" ]; then
        continue
    fi

    violations=$(grep -rn '\.unwrap()\|\.expect(' "$crate_dir/src" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" | grep -v "/tests/" | grep -v "test_" \
        | grep -v "#\[test\]" | grep -v "#\[tokio::test" \
        | grep -v 'assert!(' | grep -v 'assert_eq!(' | grep -v 'panic!(' \
        | grep -v '\.expect("must' | grep -v '\.expect("should' \
        | grep -v '\.expect("test' \
        | grep -v "// REQ:" \
        || true)

    if [ -n "$violations" ]; then
        echo "  ❌ $crate — .unwrap()/.expect() on hot path:"
        echo "$violations" | while IFS= read -r line; do
            echo "      $line"
        done
        FAILED=1
    else
        echo "  ✅ $crate — no unwrap/expect on hot paths"
    fi
done

# Also check MCP servers
for server_dir in mcp-servers/*/; do
    server=$(basename "$server_dir")
    violations=$(grep -rn '\.unwrap()\|\.expect(' "$server_dir/src" --include="*.rs" 2>/dev/null \
        | grep -v "cfg(test)" | grep -v "/tests/" | grep -v "test_" \
        | grep -v "#\[test\]" | grep -v "#\[tokio::test" \
        | grep -v 'assert!(' | grep -v 'assert_eq!(' | grep -v 'panic!(' \
        | grep -v '\.expect("must' | grep -v '\.expect("should' \
        | grep -v '\.expect("test' \
        || true)

    if [ -n "$violations" ]; then
        echo "  ❌ mcp-servers/$server — .unwrap()/.expect() on hot path:"
        echo "$violations" | while IFS= read -r line; do
            echo "      $line"
        done
        FAILED=1
    else
        echo "  ✅ mcp-servers/$server — no unwrap/expect on hot paths"
    fi
done

echo ""
if [ "$FAILED" -eq 1 ]; then
    echo "FAIL: .unwrap()/.expect() calls found on hot paths."
    echo "Replace with proper error handling: Result propagation, .ok()/or_else(), or explicit match."
    exit 1
else
    echo "PASS: No unwrap/expect calls on hot paths."
    exit 0
fi
