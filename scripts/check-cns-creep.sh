#!/usr/bin/env bash
# check-cns-creep.sh — defend against CNS namespace creep.
#
# Scans all tracing::target: "cns.*" strings in crates/ and verifies each
# is registered in CANONICAL_NAMESPACES (exact match, not hierarchical prefix).
#
# This catches ad-hoc sub-namespaces that would otherwise silently validate
# via the hierarchical is_canonical rule.
#
# Usage: scripts/check-cns-creep.sh
# Exit: 0 = all targets registered, 1 = unregistered targets found

set -euo pipefail

# Extract all cns.* tracing targets from Rust source files.
# Matches: target: "cns.foo.bar" (with optional whitespace)
targets=$(grep -roh 'target: "cns\.[^"]*"' crates/ mcp-servers/ \
    | sed 's/target: "//;s/"//' \
    | sort -u)

if [ -z "$targets" ]; then
    echo "check-cns-creep: no cns.* targets found"
    exit 0
fi

# Check each target against CANONICAL_NAMESPACES in event.rs.
# We extract the array entries and check for exact match.
canonical=$(grep -o '"cns\.[^"]*"' crates/hkask-types/src/event.rs \
    | sed 's/"//g' \
    | sort -u)

unregistered=0
for target in $targets; do
    if ! echo "$canonical" | grep -qx "$target"; then
        echo "check-cns-creep: UNREGISTERED target '$target'"
        unregistered=$((unregistered + 1))
    fi
done

if [ "$unregistered" -gt 0 ]; then
    echo "check-cns-creep: $unregistered unregistered cns.* target(s) found"
    echo "Add them to CANONICAL_NAMESPACES in crates/hkask-types/src/event.rs"
    exit 1
fi

echo "check-cns-creep: all cns.* targets registered"
