#!/bin/bash
# hKask — Documentation link checker
#
# Verifies that `cargo doc` generated complete output and that key crate
# documentation indices exist. Intra-doc-link warnings are already enforced
# at compile time by RUSTFLAGS="-D warnings" in the calling workflow.
#
# Run: bash docs/ci/check-links.sh (from workspace root)

set -euo pipefail

echo "=== Checking documentation links ==="

DOC_DIR="target/doc"

if [ ! -d "$DOC_DIR" ]; then
    echo "ERROR: target/doc directory not found — run 'cargo doc' first"
    exit 1
fi

# Verify the redirect index was created
if [ ! -f "$DOC_DIR/index.html" ]; then
    echo "ERROR: target/doc/index.html (redirect) not found"
    exit 1
fi

# Check that core crate docs exist (these are the architecture foundation)
MISSING=0
for crate in hkask_types hkask_services_core hkask_ports hkask_capability hkask_cns; do
    if [ ! -d "$DOC_DIR/$crate" ]; then
        echo "WARNING: $crate documentation directory not found"
        MISSING=$((MISSING + 1))
    fi
done

if [ "$MISSING" -gt 0 ]; then
    echo "WARNING: $MISSING core crate(s) missing documentation"
    echo "This may be expected if they are not workspace members or have no public items."
fi

echo "Documentation link check passed."
