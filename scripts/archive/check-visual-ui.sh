#!/usr/bin/env bash
# No visual UI infrastructure — headless constraint enforcement (P3 §5).
#
# Scans crates/ for references to Grafana, Prometheus, dashboards,
# or visual UI frameworks. hKask is a headless system — CLI/MCP/API only.
#
# Exit 0 = clean. Exit 1 = visual UI references found.

set -euo pipefail

echo "=== No visual UI infrastructure ==="

violations=$(grep -r 'grafana\|prometheus\|dashboard\|visual.*ui' crates/ --include="*.rs" || true)

if [ -n "$violations" ]; then
    echo "HEADLESS CONSTRAINT VIOLATED: Visual UI references detected:"
    echo "$violations"
    exit 1
fi

echo "PASS: No visual UI infrastructure."
exit 0
