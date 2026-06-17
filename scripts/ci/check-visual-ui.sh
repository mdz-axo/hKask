#!/usr/bin/env bash
# Visual UI infrastructure check — CI security invariant (P3 §5 enforcement).
#
# Prohibition #1: No visual UI, dashboards, Grafana, Prometheus, or monitoring
# stacks. hKask is headless — CLI/MCP/API only. CNS provides observability.
#
# Exit 0 = clean. Exit 1 = violations found.

set -euo pipefail

echo "=== No Visual UI Infrastructure (P3 §5) ==="

FAILED=0

for root in crates mcp-servers; do
    [ -d "$root" ] || continue

    while IFS= read -r -d '' file; do
        # Skip test files
        [[ "$file" == */tests/* ]] && continue

        violations=$(grep -niE 'grafana|prometheus|dashboard|web.*frontend|visual.*ui|html.*render' "$file" 2>/dev/null || true)

        if [ -n "$violations" ]; then
            while IFS= read -r vline; do
                echo "  ❌ $file:$vline"
                FAILED=1
            done <<< "$violations"
        fi
    done < <(find "$root" -name '*.rs' -print0 2>/dev/null)
done

if [ "$FAILED" -eq 1 ]; then
    echo ""
    echo "FAIL: Visual UI infrastructure references found. Use CLI/MCP/API only."
    exit 1
fi

echo ""
echo "PASS: No visual UI infrastructure."
exit 0
