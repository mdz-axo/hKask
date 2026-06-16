#!/usr/bin/env bash
# No todo!/unimplemented! stubs — P6 constraint enforcement.
#
# Scans crates/ for todo!() and unimplemented!() macros in production code.
# Test code (cfg(test) modules) is excluded.
#
# Exit 0 = clean. Exit 1 = stubs found in production code.

set -euo pipefail

echo "=== No todo!/unimplemented! stubs (P6) ==="

violations=$(grep -rn 'todo!\|unimplemented!' crates/ --include="*.rs" | grep -v "cfg(test)" || true)

if [ -n "$violations" ]; then
    echo "CONSTRAINT P6 VIOLATED: Stubs detected in production code:"
    echo "$violations"
    exit 1
fi

echo "PASS: No stubs in production code."
exit 0
