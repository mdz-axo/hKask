#!/usr/bin/env bash
# No hardcoded secrets — CI security invariant check.
#
# Scans crates/ for hardcoded API keys, passwords, and credentials.
# Detects patterns like FW_API_KEY="...", DI_API_KEY="...", password="...".
#
# Exit 0 = clean. Exit 1 = hardcoded secrets found.

set -euo pipefail

echo "=== No hardcoded secrets ==="

violations=$(grep -rn 'FW_API_KEY[[:space:]]*=[[:space:]]*"\|DI_API_KEY[[:space:]]*=[[:space:]]*"\|password[[:space:]]*=[[:space:]]*"' crates/ --include="*.rs" || true)

if [ -n "$violations" ]; then
    echo "SECURITY INVARIANT VIOLATED: Hardcoded secrets detected:"
    echo "$violations"
    exit 1
fi

echo "PASS: No hardcoded secrets."
exit 0
