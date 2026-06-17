#!/usr/bin/env bash
# Hardcoded secrets check — CI security invariant.
#
# Scans for hardcoded API keys, passwords, tokens, and secrets in source code.
# Skips test files, env.example files, and documentation.
#
# Exit 0 = clean. Exit 1 = violations found.

set -euo pipefail

echo "=== No Hardcoded Secrets ==="

FAILED=0

for root in crates mcp-servers; do
    [ -d "$root" ] || continue

    while IFS= read -r -d '' file; do
        # Skip test files, examples, and docs
        [[ "$file" == */tests/* ]] && continue
        [[ "$file" == *env.example* ]] && continue

        violations=$(grep -nE '(api[_-]?key|api[_-]?secret|password|token|private[_-]?key)[[:space:]]*=[[:space:]]*"[^"]+"' "$file" 2>/dev/null | grep -v '//\|///\|/\*' | grep -v '= "{' || true)

        if [ -n "$violations" ]; then
            while IFS= read -r vline; do
                echo "  ❌ $file:$vline"
                FAILED=1
            done <<< "$violations"
        fi
    done < <(find "$root" -name '*.rs' -print0 2>/dev/null)
done

if [ "$FAILED" -eq 1 ]; then
    echo ""
    echo "FAIL: Hardcoded secrets found. Use environment variables or keystore instead."
    exit 1
fi

echo ""
echo "PASS: No hardcoded secrets."
exit 0
