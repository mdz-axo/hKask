#!/bin/bash
# Check for .unwrap() calls on hot paths in CNS and agents crates.
#
# Production code in hkask-cns and hkask-agents must not use .unwrap()
# outside of #[cfg(test)] modules. Use Result, .expect(), or OCAP-gated
# fallthrough instead.
#
# Exits 0 if no violations, 1 if violations found.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

hot_dirs=(
    "$repo_root/crates/hkask-cns/src"
    "$repo_root/crates/hkask-agents/src"
)

has_violation=false
violations=""

# For each .rs file, use awk to strip #[cfg(test)] mod blocks and then
# check the remaining (production-only) code for .unwrap() calls.
# The awk script tracks brace depth to correctly handle nested braces.
strip_test_blocks() {
    local file="$1"
    awk '
    /^#\[cfg\(test\)\]/ {
        # Found #[cfg(test)] — enter skip mode, wait for opening brace
        skip = 1
        next
    }
    skip == 1 && /{/ {
        # First opening brace after #[cfg(test)] — start tracking depth
        gsub(/\{/, "{")
        depth = gsub(/\{/, "{")
        gsub(/\}/, "}")
        depth -= gsub(/\}/, "}")
        if (depth <= 0) {
            # Single-line block like: mod tests { } (unlikely but handle)
            depth = 0
            skip = 0
        } else {
            skip = 2
        }
        next
    }
    skip == 1 {
        # Lines between #[cfg(test)] and the opening brace (e.g., "mod tests")
        next
    }
    skip == 2 {
        # Inside a #[cfg(test)] block — track brace depth
        depth += gsub(/\{/, "{")
        depth -= gsub(/\}/, "}")
        if (depth <= 0) {
            depth = 0
            skip = 0
        }
        next
    }
    { print }
    ' "$file"
}

for dir in "${hot_dirs[@]}"; do
    if [ ! -d "$dir" ]; then
        echo "WARNING: Hot-path directory not found: $dir"
        continue
    fi

    while IFS= read -r file; do
        [ -f "$file" ] || continue

        # Get production-only code (test blocks stripped), then check for .unwrap()
        prod_violations=$(strip_test_blocks "$file" | grep -n '\.unwrap()' || true)

        if [ -n "$prod_violations" ]; then
            # Report violations with original file paths
            while IFS= read -r line; do
                [ -z "$line" ] && continue
                # Extract the line number from the stripped output and map it
                # back to the original file. Since we can't easily map line numbers
                # after stripping, just report the production-code violations directly.
                violations="${violations}${file##$repo_root/}:${line}"$'\n'
                has_violation=true
            done <<< "$prod_violations"
        fi
    done < <(find "$dir" -name '*.rs')
done

if $has_violation; then
    echo "VIOLATION: .unwrap() on hot paths (CNS/agents) outside test code:"
    echo "$violations"
    echo ""
    echo "Use proper error handling (Result, .expect(), or OCAP-gated fallthrough) instead."
    exit 1
fi

echo "OK: No .unwrap() on hot paths outside test code in CNS/agents"
