#!/usr/bin/env bash
# No deprecated — P7 constraint enforcement.
#
# Scans crates/ for #[deprecated] attributes.
# Deprecated code earns deletion, not annotation (P7).
#
# Exit 0 = clean. Exit 1 = deprecated code found.

set -euo pipefail

echo "=== No deprecated (P7) ==="

violations=$(grep -rn '#\[deprecated\]' crates/ --include="*.rs" || true)

if [ -n "$violations" ]; then
    echo "CONSTRAINT P7 VIOLATED: Deprecated code detected:"
    echo "$violations"
    exit 1
fi

echo "PASS: No deprecated code."
exit 0
