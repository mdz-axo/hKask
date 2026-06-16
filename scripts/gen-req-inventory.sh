#!/bin/bash
# Generate REQ contract inventory document
set -euo pipefail

OUT="docs/architecture/core/REQ_CONTRACT_INVENTORY.md"
TMP=$(mktemp)

echo "Generating $OUT ..."

# Header
cat > "$TMP" << 'HEADER'
---
title: "REQ Contract Inventory"
audience: [architects, developers, agents, curators]
last_updated: 2026-06-16
version: "0.27.0"
status: "Draft"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle]
---

# REQ Contract Inventory

**Purpose:** Catalog of every `/// REQ:` contract on public functions across crates and MCP servers. Each entry shows the REQ ID, its contract terms (pre/post/inv), and the function it annotates. This is the raw material for designing the rSolidity contract vocabulary.

## Summary by Crate

HEADER

# Count by crate
echo "| Crate | Count | Domain |" >> "$TMP"
echo "|-------|-------|--------|" >> "$TMP"

for dir in crates/hkask-*; do
    crate=$(basename "$dir")
    count=$(grep -rn "/// REQ:" "$dir/src/"*.rs 2>/dev/null | wc -l) || true
    count="${count// /}"
    [ "$count" -gt 0 ] || continue
    case "$crate" in
        hkask-services) d="Service layer" ;;
        hkask-types) d="Type system" ;;
        hkask-storage) d="Storage" ;;
        hkask-agents) d="Agent runtime" ;;
        hkask-cli) d="CLI surface" ;;
        hkask-cns) d="CNS observability" ;;
        hkask-inference) d="Inference" ;;
        hkask-keystore) d="Keystore" ;;
        hkask-mcp) d="MCP framework" ;;
        hkask-memory) d="Memory" ;;
        hkask-communication) d="Communication" ;;
        hkask-templates) d="Templates" ;;
        hkask-test-harness) d="Test harness" ;;
        hkask-api) d="API surface" ;;
        hkask-wallet) d="Wallet" ;;
        hkask-condenser) d="Condenser" ;;
        *) d="Other" ;;
    esac
    echo "| $crate | $count | $d |" >> "$TMP"
done

for dir in mcp-servers/mcp-*; do
    [ -d "$dir" ] || continue
    server=$(basename "$dir")
    count=$(grep -rn "/// REQ:" "$dir/src/"*.rs 2>/dev/null | wc -l) || true
    count="${count// /}"
    [ "$count" -gt 0 ] || continue
    echo "| $server | $count | MCP server |" >> "$TMP"
done

echo "" >> "$TMP"
echo "## Per-Crate Contract Detail" >> "$TMP"
echo "" >> "$TMP"

# Process each crate
for dir in crates/hkask-* mcp-servers/mcp-*; do
    crate=$(basename "$dir")
    [ -d "$dir" ] || continue
    count=$(grep -rn "/// REQ:" "$dir/src/"*.rs 2>/dev/null | wc -l) || true
    count="${count// /}"
    [ "$count" -gt 0 ] || continue

    echo "### $crate ($count contracts)" >> "$TMP"
    echo "" >> "$TMP"

    grep -rnH "/// REQ:" "$dir/src/"*.rs 2>/dev/null | while IFS=: read -r file line rest; do
        id=$(echo "$rest" | sed 's/^[[:space:]]*\/\/\/ REQ: //' | sed 's/^[[:space:]]*//')
        id_clean=$(echo "$id" | tr -d '[:space:]')

        # Read next 6 lines
        context=$(sed -n "$((line+1)),$((line+6))p" "$file" 2>/dev/null || true)

        # Extract terms
        pre=$(echo "$context" | grep "pre:" | sed 's/^[[:space:]]*\/\/\/ //; s/^pre: //' | tr '\n' ';' | sed 's/;$//' | tr -d '|' || true)
        post=$(echo "$context" | grep "post:" | sed 's/^[[:space:]]*\/\/\/ //; s/^post: //' | tr '\n' ';' | sed 's/;$//' | tr -d '|' || true)
        inv=$(echo "$context" | grep "inv:" | sed 's/^[[:space:]]*\/\/\/ //; s/^inv: //' | tr '\n' ';' | sed 's/;$//' | tr -d '|' || true)

        # Check principle anchoring
        if echo "$id_clean" | grep -qE '^P[0-9]+-'; then
            princ="✅ anchored"
        else
            princ="⚠ unanchored"
        fi

        # Check if bare
        if [ -z "$pre" ] && [ -z "$post" ] && [ -z "$inv" ]; then
            status="🔴 bare"
        elif [ -z "$pre" ] || [ -z "$post" ]; then
            status="🟡 partial"
        else
            status="🟢 full"
        fi

        echo "#### $id_clean ($status)" >> "$TMP"
        echo "" >> "$TMP"
        echo "- **Principle:** $princ" >> "$TMP"
        [ -n "$pre" ] && echo "- **Pre:** $pre" >> "$TMP"
        [ -n "$post" ] && echo "- **Post:** $post" >> "$TMP"
        [ -n "$inv" ] && echo "- **Inv:** $inv" >> "$TMP"
        echo "- **File:** $file:$line" >> "$TMP"
        echo "" >> "$TMP"
    done
    echo "" >> "$TMP"
done

# Footer
cat >> "$TMP" << 'FOOTER'

---

## Next Steps

1. **Review the inventory** — identify patterns, gaps, and inconsistencies
2. **Design rSolidity vocabulary** — how `require!()`, `assert!()`, `revert!()`, `emit!()`, `#[ocap]` map to these contracts
3. **Pick a starting contract** — rewrite one well-formed contract in rSolidity to establish the pattern
4. **Write the rSolidity crate** — `crates/hkask-rsolidity/` with macro implementations
5. **Migrate contracts one crate at a time** — strangler fig: old `/// REQ:` stays, new `#[rSolidity]` replaces

Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)
FOOTER

# Move to final location
mv "$TMP" "$OUT"
echo "Done. $OUT ($(wc -l < "$OUT") lines)"
