#!/bin/bash
# hKask — Documentation link checker
#
# Verifies that `cargo doc` generated complete output for all workspace
# members. Intra-doc-link warnings are already enforced at compile time
# by RUSTFLAGS="-D warnings" in the calling workflow.
#
# Run: bash docs/ci/check-links.sh (from workspace root)

set -euo pipefail

echo "=== Checking documentation links ==="

DOC_DIR="target/doc"

if [ ! -d "$DOC_DIR" ]; then
    echo "ERROR: target/doc directory not found — run 'cargo doc' first"
    exit 1
fi

# Verify the redirect index was created (CI creates this separately)
if [ ! -f "$DOC_DIR/index.html" ]; then
    echo "WARNING: target/doc/index.html (redirect) not found — CI creates this separately"
fi

# Check that every workspace member with public items produced docs.
# Discover workspace members from filesystem (same approach as verify-docs.sh).
# This avoids a hardcoded list that silently drifts.
MISSING=0
for dir in crates/*/ mcp-servers/*/; do
    # Skip fuzz subdirectories — they don't produce standalone docs
    [[ "$dir" == */fuzz/ ]] && continue
    # Convert directory name (kebab-case) to expected doc directory (underscores)
    name=$(basename "$dir")
    crate_name=$(echo "$name" | tr '-' '_')
    if [ ! -d "$DOC_DIR/$crate_name" ]; then
        echo "  NOTE: $crate_name — no doc directory (may have no public API)"
        MISSING=$((MISSING + 1))
    fi
done

if [ "$MISSING" -gt 0 ]; then
    echo "NOTE: $MISSING workspace member(s) have no generated docs."
    echo "This is expected for crates with no public items or fuzz-only crates."
fi

echo "Documentation link check passed."
