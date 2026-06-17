#!/usr/bin/env bash
# Public surface governance check — CI gate for Task 8 (Wave 5).
#
# Flags crates with >7 public items in lib.rs that lack a
# PUBLIC_SURFACE.md justification document in docs/architecture/.
#
# Deep-module discipline (Ousterhout): modules with >7 public
# functions should justify their surface breadth. This check
# enforces that justification exists for oversized crates.
#
# Exit 0 = all oversized crates have justification docs.
# Exit 1 = one or more oversized crates lack justification.

set -euo pipefail

THRESHOLD=7
DOCS_DIR="docs/architecture"
FAILED=0

echo "=== Public Surface Governance Check ==="
echo "Threshold: >${THRESHOLD} public items requires PUBLIC_SURFACE.md"
echo ""

for crate_dir in crates/*/; do
    crate=$(basename "$crate_dir")
    lib="${crate_dir}src/lib.rs"

    if [ ! -f "$lib" ]; then
        continue
    fi

    # Count pub items (pub fn, pub struct, pub enum, pub trait, pub mod, pub use, pub type, pub const, pub static)
    pub_count=$(grep -cE '^\s*pub (fn|struct|enum|trait|mod|use|type|const|static|unsafe fn|async fn)' "$lib" 2>/dev/null || echo 0)

    if [ "$pub_count" -gt "$THRESHOLD" ]; then
        justification="${DOCS_DIR}/PUBLIC_SURFACE-${crate}.md"
        if [ -f "$justification" ]; then
            echo "  ✅ $crate ($pub_count pub items) — justified: $justification"
        else
            echo "  ❌ $crate ($pub_count pub items) — MISSING justification: $justification"
            FAILED=1
        fi
    else
        echo "  ✅ $crate ($pub_count pub items) — within threshold"
    fi
done

echo ""
if [ "$FAILED" -eq 1 ]; then
    echo "FAIL: One or more crates exceed the public surface threshold without justification."
    echo "Add a PUBLIC_SURFACE-<crate>.md file to docs/architecture/ explaining why the surface is large."
    exit 1
else
    echo "PASS: All oversized crates have public surface justification docs."
    exit 0
fi
