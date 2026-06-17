#!/usr/bin/env bash
# No wildcard capabilities — CI security invariant check.
#
# Scans crates/ for '"*"' usage in capability/token context.
# Excludes CapabilityChecker authority, boundary-matching checks,
# enforcement rejections, and test code.
#
# Exit 0 = clean. Exit 1 = wildcard capabilities found.

set -euo pipefail

echo "=== No wildcard capabilities ==="

violations=$(grep -rnF '"*"' crates/ --include="*.rs" \
    | grep -v "cfg(test)" | grep -v "test_" | grep -v "// REQ:" \
    | grep -v "registry.rs" | grep -v "checker.rs" | grep -v "register_agent" | grep -vF 'vec!["' \
    | grep -v "verification.rs" \
    | grep -v '== "\*"' | grep -v '!= "\*"' \
    | grep -v 'Wildcard' | grep -v 'Err(' \
    | grep -v '"\*",$' || true)

if [ -n "$violations" ]; then
    echo "SECURITY INVARIANT VIOLATED: Wildcard capabilities detected:"
    echo "$violations"
    exit 1
fi

echo "PASS: No wildcard capabilities."
exit 0
