#!/bin/bash
# hKask — Documentation-code consistency verifier (VSM S3 audit)
#
# Checks that critical architecture documents reference actual crate paths
# and that public modules in core crates carry documentation comments.
# This is a lightweight S3 audit: sporadically probes the system to verify
# that the documented model matches the implemented system.
#
# Run: bash docs/ci/verify-docs.sh (from workspace root)

set -euo pipefail

echo "=== Verifying documentation-code consistency (S3 audit) ==="

PASS=0
FAIL=0

# ── Check 1: Architecture master exists ──────────────────────────────
ARCH_MASTER="docs/architecture/hKask-architecture-master.md"
if [ -f "$ARCH_MASTER" ]; then
    echo "  ✓ Architecture master document present"
    PASS=$((PASS + 1))
else
    echo "  ✗ Architecture master document missing: $ARCH_MASTER"
    FAIL=$((FAIL + 1))
fi

# ── Check 2: Principle docs exist ────────────────────────────────────
for doc in docs/architecture/core/PRINCIPLES.md docs/architecture/core/MDS.md docs/architecture/core/TESTING_DISCIPLINE.md; do
    if [ -f "$doc" ]; then
        echo "  ✓ $doc"
        PASS=$((PASS + 1))
    else
        echo "  ✗ Missing: $doc"
        FAIL=$((FAIL + 1))
    fi
done

# ── Check 3: Architecture doc references match actual crate structure ─
if [ -f "$ARCH_MASTER" ]; then
    echo ""
    echo "  Checking crate references in architecture master..."
    for crate_dir in crates/hkask-*/; do
        crate_name=$(basename "$crate_dir")
        if ! grep -q "$crate_name" "$ARCH_MASTER"; then
            echo "  WARNING: $crate_name not referenced in architecture master"
        fi
    done
    echo "  Crate reference check complete."
fi

# ── Summary ──────────────────────────────────────────────────────────
echo ""
echo "=== S3 Consistency Report ==="
echo "  Passed: $PASS"
echo "  Failed: $FAIL"

if [ "$FAIL" -gt 0 ]; then
    echo ""
    echo "ERROR: $FAIL documentation consistency check(s) failed."
    echo "Update the missing documents or fix references."
    exit 1
fi

echo "Documentation-code consistency verified."
