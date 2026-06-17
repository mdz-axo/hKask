#!/usr/bin/env bash
# Public surface governance check — CI gate for Task 8 (Wave 5).
#
# Flags crates with >7 public items in lib.rs that lack an entry in
# the consolidated PUBLIC_SURFACE_JUSTIFICATIONS.md document.
#
# Deep-module discipline (Ousterhout): modules with >7 public
# functions should justify their surface breadth. This check
# enforces that a justification entry exists for oversized crates.
#
# Exit 0 = all oversized crates have justification entries.
# Exit 1 = one or more oversized crates lack justification.

set -euo pipefail

THRESHOLD=7
JUSTIFICATION="docs/architecture/PUBLIC_SURFACE_JUSTIFICATIONS.md"
FAILED=0

echo "=== Public Surface Governance Check ==="
echo "Threshold: >${THRESHOLD} public items requires entry in ${JUSTIFICATION}"
echo ""

if [ ! -f "$JUSTIFICATION" ]; then
    echo "FAIL: ${JUSTIFICATION} not found."
    echo "Create it with one table row per oversized crate."
    exit 1
fi

for crate_dir in crates/*/; do
    crate=$(basename "$crate_dir")
    lib="${crate_dir}src/lib.rs"

    if [ ! -f "$lib" ]; then
        continue
    fi

    # Count pub items (pub fn, pub struct, pub enum, pub trait, pub mod, pub use, pub type, pub const, pub static)
    pub_count=$(grep -cE '^\s*pub (fn|struct|enum|trait|mod|use|type|const|static|unsafe fn|async fn)' "$lib" 2>/dev/null || echo 0)

    if [ "$pub_count" -gt "$THRESHOLD" ]; then
        if grep -q "\`${crate}\`" "$JUSTIFICATION" 2>/dev/null; then
            echo "  ✅ $crate ($pub_count pub items) — justified in $JUSTIFICATION"
        else
            echo "  ❌ $crate ($pub_count pub items) — MISSING entry in $JUSTIFICATION"
            FAILED=1
        fi
    else
        echo "  ✅ $crate ($pub_count pub items) — within threshold"
    fi
done

echo ""
if [ "$FAILED" -eq 1 ]; then
    echo "FAIL: One or more crates exceed the public surface threshold without justification."
    echo "Add an entry to ${JUSTIFICATION} explaining why the surface is large."
    exit 1
else
    echo "PASS: All oversized crates have entries in ${JUSTIFICATION}."
    exit 0
fi
