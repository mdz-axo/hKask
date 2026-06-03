#!/bin/bash
# Check for undocumented unsafe blocks outside hkask-keystore
# All unsafe blocks must have a // SAFETY: comment on the preceding line,
# same line, or next line.

set -euo pipefail

has_violation=false
violations=""

while IFS= read -r match; do
    [ -z "$match" ] && continue
    file=$(echo "$match" | cut -d: -f1)
    linenum=$(echo "$match" | cut -d: -f2)
    content=$(echo "$match" | cut -d: -f3-)

    # SAFETY: on same line as unsafe {
    if echo "$content" | grep -q '// SAFETY:'; then
        continue
    fi
    # SAFETY: on preceding line (standard Rust convention)
    prev=$(sed -n "$((linenum-1))p" "$file")
    if echo "$prev" | grep -q '// SAFETY:'; then
        continue
    fi
    # SAFETY: on next line (inside the block)
    next=$(sed -n "$((linenum+1))p" "$file")
    if echo "$next" | grep -q '// SAFETY:'; then
        continue
    fi

    violations="${violations}${match}"$'\n'
    has_violation=true
done < <(grep -rn 'unsafe {' crates/ mcp-servers/ --include='*.rs' | grep -v 'hkask-keystore')

if $has_violation; then
    echo "VIOLATION: unsafe blocks outside hkask-keystore without SAFETY comment:"
    echo "$violations"
    exit 1
fi

echo "OK: All unsafe blocks outside hkask-keystore have SAFETY documentation"
