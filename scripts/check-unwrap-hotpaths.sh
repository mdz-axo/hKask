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

# For each .rs file, find the line ranges of #[cfg(test)] mod blocks,
# then check .unwrap() calls that fall outside those ranges.
for dir in "${hot_dirs[@]}"; do
    if [ ! -d "$dir" ]; then
        echo "WARNING: Hot-path directory not found: $dir"
        continue
    fi

    while IFS= read -r file; do
        [ -f "$file" ] || continue

        # Extract #[cfg(test)] module line ranges: start_line end_line
        # A #[cfg(test)] block looks like:
        #   #[cfg(test)]
        #   mod tests { ... }
        # We need to find the matching closing brace.
        test_ranges=()

        # Find all lines with #[cfg(test)]
        cfg_test_lines=()
        while IFS= read -r line; do
            [ -z "$line" ] && continue
            cfg_test_lines+=("$line")
        done < <(grep -n '#\[cfg(test)\]' "$file" 2>/dev/null || true)

        for cfg_line in "${cfg_test_lines[@]:-}"; do
            start_line="$(echo "$cfg_line" | cut -d: -f1)"

            # Find the opening brace of the mod block (could be same line or next line)
            mod_start="$start_line"
            found_brace=false
            for ((i = mod_start; i <= mod_start + 5; i++)); do
                line_content="$(sed -n "${i}p" "$file")"
                if echo "$line_content" | grep -q '{'; then
                    mod_start="$i"
                    found_brace=true
                    break
                fi
            done

            if [ "$found_brace" = false ]; then
                continue
            fi

            # Find matching closing brace
            depth=0
            end_line="$mod_start"
            total_lines="$(wc -l < "$file")"
            for ((i = mod_start; i <= total_lines; i++)); do
                line_content="$(sed -n "${i}p" "$file")"
                # Count braces (ignore braces inside strings/comments approximately)
                opens="$(echo "$line_content" | tr -cd '{' | wc -c)"
                closes="$(echo "$line_content" | tr -cd '}' | wc -c)"
                depth=$((depth + opens - closes))
                if [ $depth -le 0 ]; then
                    end_line="$i"
                    break
                fi
            done

            test_ranges+=("$start_line $end_line")
        done

        # Now find all .unwrap() calls and check if they're outside test ranges
        while IFS= read -r match; do
            [ -z "$match" ] && continue
            linenum="$(echo "$match" | cut -d: -f2)"

            # Skip comment lines
            content="$(echo "$match" | cut -d: -f3-)"
            if echo "$content" | grep -qE '^\s*//'; then
                continue
            fi

            # Check if this line is inside any cfg(test) range
            in_test=false
            for range in "${test_ranges[@]:-}"; do
                range_start="$(echo "$range" | cut -d' ' -f1)"
                range_end="$(echo "$range" | cut -d' ' -f2)"
                if [ "$linenum" -ge "$range_start" ] && [ "$linenum" -le "$range_end" ]; then
                    in_test=true
                    break
                fi
            done

            if [ "$in_test" = true ]; then
                continue
            fi

            violations="${violations}${match}"$'\n'
            has_violation=true
        done < <(grep -rn '\.unwrap()' "$file" | grep -v '^\s*//')
    done < <(find "$dir" -name '*.rs')
done

if $has_violation; then
    echo "VIOLATION: .unwrap() on hot paths (CNS/agents) outside test code:"
    echo "$violations"
    echo ""
    echo "Use proper error handling (Result, .expect(), OCAP-gated fallthrough) instead."
    exit 1
fi

echo "OK: No .unwrap() on hot paths outside test code in CNS/agents"
