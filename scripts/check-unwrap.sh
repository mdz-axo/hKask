#!/usr/bin/env bash
# No unwrap on hot paths — CI security invariant check.
#
# Scans CNS, agents, inference, services, and MCP server crates for
# .unwrap() calls that could panic at runtime.
#
# Uses awk to strip #[cfg(test)] modules before scanning, avoiding
# false positives from test-only code. Skips files in /tests/ dirs.
#
# Called by: ci-quality-gates.sh (hard gate), ci.yml security-invariants
#
# Exit 0 = clean. Exit 1 = violations found.

set -euo pipefail

echo "=== No unwrap on hot paths ==="

FAILED=0

# Use awk to track test-module state and only print non-test lines,
# then grep for .unwrap() on the filtered output.
check_dir() {
    local dir="$1"
    local label="$2"

    while IFS= read -r -d '' file; do
        # Skip files in /tests/ directories
        [[ "$file" == */tests/* ]] && continue

        # Use awk to strip #[cfg(test)] modules, then grep for .unwrap()
        local violations
        violations=$(awk '
            /^[[:space:]]*#\[cfg\(test\)\]/ {
                in_test = 1
                started = 0
                depth = 0
                next
            }
            in_test {
                n = split($0, chars, "")
                for (i = 1; i <= n; i++) {
                    if (chars[i] == "{") { depth++; started = 1 }
                    if (chars[i] == "}") depth--
                }
                # Only exit test mode after we have entered the module body
                if (started && depth <= 0) in_test = 0
                next
            }
            !in_test { print }
        ' "$file" 2>/dev/null | grep -n '\.unwrap()' || true)

        if [ -n "$violations" ]; then
            while IFS= read -r vline; do
                echo "  ❌ $label: $file:$vline"
                FAILED=1
            done <<< "$violations"
        fi
    done < <(find "$dir" -name '*.rs' -print0 2>/dev/null)
}

# Hot-path crates
for crate in hkask-cns hkask-agents hkask-inference hkask-services hkask-mcp hkask-communication hkask-condenser; do
    crate_dir="crates/${crate}"
    [ -d "$crate_dir" ] || continue
    check_dir "$crate_dir/src" "$crate"
done

# MCP servers
for server_dir in mcp-servers/*/; do
    server=$(basename "$server_dir")
    check_dir "$server_dir/src" "mcp-servers/$server"
done

if [ "$FAILED" -eq 1 ]; then
    echo ""
    echo "FAIL: .unwrap() calls found on hot paths (excluding test modules)."
    exit 1
fi

echo ""
echo "PASS: No .unwrap() on hot paths (excluding test modules)."
exit 0
