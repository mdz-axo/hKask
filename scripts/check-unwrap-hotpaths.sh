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

for dir in "${hot_dirs[@]}"; do
    if [ ! -d "$dir" ]; then
        echo "WARNING: Hot-path directory not found: $dir"
        continue
    fi

    # Find all .rs files, grep for .unwrap(), skip lines that are comments
    while IFS= read -r match; do
        [ -z "$match" ] && continue

        file="$(echo "$match" | cut -d: -f1)"
        linenum="$(echo "$match" | cut -d: -f2)"

        # Skip if inside a #[cfg(test)] module.
        # Scan backwards from the matched line to the beginning of the file.
        # If we encounter #[cfg(test)] before any non-test top-level mod,
        # the line is inside a test module.
        in_test=false
        for ((i = linenum - 1; i >= 1; i--)); do
            prev_line="$(sed -n "${i}p" "$file")"

            if [[ "$prev_line" == *"#[cfg(test)]"* ]]; then
                in_test=true
                break
            fi
        done

        if [ "$in_test" = true ]; then
            continue
        fi

        violations="${violations}${match}"$'\n'
        has_violation=true
    done < <(grep -rn '\.unwrap()' "$dir" --include='*.rs' | grep -v '^\s*//')
done

if $has_violation; then
    echo "VIOLATION: .unwrap() on hot paths (CNS/agents) outside test code:"
    echo "$violations"
    echo ""
    echo "Use proper error handling (Result, .expect(), OCAP-gated fallthrough) instead."
    exit 1
fi

echo "OK: No .unwrap() on hot paths outside test code in CNS/agents"
