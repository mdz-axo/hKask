#!/bin/bash
# Public seam inventory generator — Phase 2 Task 1 / PR 2.1.1
#
# Walks all .rs files in crates/ and mcp-servers/, extracts public items
# (pub fn, pub struct, pub enum, pub trait, pub type), and cross-references
# with // REQ:-tagged tests to produce a coverage-linked inventory.
#
# Output: docs/status/public-seam-inventory.md
# Exit 0 if inventory matches existing file, exit 1 on drift (CI gate).
#
# Usage:
#   scripts/audit/public-seam-inventory.sh            # Generate + compare
#   scripts/audit/public-seam-inventory.sh --write     # Generate + overwrite
#   scripts/audit/public-seam-inventory.sh --check     # CI mode: fail on drift

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
output="$repo_root/docs/status/public-seam-inventory.md"
mode="${1:---check}"

# ── helpers ───────────────────────────────────────────────────────────────────

crate_name_from_path() {
    local path="$1"
    # Strip repo_root prefix
    local rel="${path#$repo_root/}"
    # Extract crate name (crates/<name>/... or mcp-servers/<name>/...)
    echo "$rel" | cut -d/ -f2
}

module_path_from_file() {
    local path="$1"
    # Strip repo_root prefix, then crate/src/ prefix
    local rel="${path#$repo_root/}"
    # crates/hkask-types/src/foo/bar.rs → hkask-types::foo::bar
    # mcp-servers/hkask-mcp-condenser/src/main.rs → hkask-mcp-condenser
    local crate
    crate=$(echo "$rel" | cut -d/ -f2)
    local rest="${rel#*/src/}"
    rest="${rest%.rs}"
    rest="${rest//\//::}"
    if [ "$rest" = "lib" ]; then
        echo "$crate"
    elif [ "$rest" = "main" ] || [ "$rest" = "bin" ]; then
        echo "$crate"
    else
        echo "${crate}::${rest}"
    fi
}

# ── collect public items ──────────────────────────────────────────────────────

collect_public_items() {
    local scopes="$1"  # "crates" or "mcp-servers" or "both"
    local outfile="$2"

    local search_dirs=()
    if [ "$scopes" = "both" ] || [ "$scopes" = "crates" ]; then
        search_dirs+=("$repo_root/crates")
    fi
    if [ "$scopes" = "both" ] || [ "$scopes" = "mcp-servers" ]; then
        search_dirs+=("$repo_root/mcp-servers")
    fi

    > "$outfile"

    for dir in "${search_dirs[@]}"; do
        [ -d "$dir" ] || continue

        while IFS= read -r -d '' file; do
            [ -f "$file" ] || continue

            local crate modpath
            crate=$(crate_name_from_path "$file")
            modpath=$(module_path_from_file "$file")

            # Skip test files (identified by _test suffix or test/ directory)
            case "$file" in
                *_test.rs) continue ;;
                */tests/*) continue ;;
            esac

            # Extract public items from this file (non-test context only).
            # We use a simple awk to strip #[cfg(test)] blocks, then grep for
            # public declarations in the remaining code.
            local stripped
            stripped=$(awk '
                /^#\[cfg\(test\)\]/ { skip=1; next }
                skip==1 && /{/ { depth=1; skip=2; next }
                skip==1 { next }
                skip==2 {
                    depth += gsub(/\{/, "{")
                    depth -= gsub(/\}/, "}")
                    if (depth <= 0) { depth=0; skip=0 }
                    next
                }
                { print }
            ' "$file")

            # Extract matching lines into temp file to avoid pipefail killing the script
            local matches
            matches=$(mktemp)

            # Find pub fn signatures
            echo "$stripped" | grep -n '^[[:space:]]*pub[[:space:]]\+fn[[:space:]]' > "$matches" 2>/dev/null || true
            while IFS=: read -r linenum line; do
                [ -z "$linenum" ] && continue
                local fn_name
                fn_name=$(echo "$line" | sed -n 's/.*pub[[:space:]]\+\(async[[:space:]]\+\)\?fn[[:space:]]\+\([a-zA-Z_][a-zA-Z0-9_]*\).*/\2/p')
                [ -n "$fn_name" ] || continue
                local sig
                sig=$(echo "$line" | sed 's/^[[:space:]]*//' | cut -c1-120)
                echo "FN|$crate|$modpath|$fn_name|$file:$linenum|$sig" >> "$outfile"
            done < "$matches"

            # Find pub struct declarations
            echo "$stripped" | grep -n '^[[:space:]]*pub[[:space:]]\+struct[[:space:]]' > "$matches" 2>/dev/null || true
            while IFS=: read -r linenum line; do
                [ -z "$linenum" ] && continue
                local st_name
                st_name=$(echo "$line" | sed -n 's/.*pub[[:space:]]\+struct[[:space:]]\+\([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/p')
                [ -n "$st_name" ] || continue
                local sig
                sig=$(echo "$line" | sed 's/^[[:space:]]*//' | cut -c1-120)
                echo "ST|$crate|$modpath|$st_name|$file:$linenum|$sig" >> "$outfile"
            done < "$matches"

            # Find pub enum declarations
            echo "$stripped" | grep -n '^[[:space:]]*pub[[:space:]]\+enum[[:space:]]' > "$matches" 2>/dev/null || true
            while IFS=: read -r linenum line; do
                [ -z "$linenum" ] && continue
                local en_name
                en_name=$(echo "$line" | sed -n 's/.*pub[[:space:]]\+enum[[:space:]]\+\([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/p')
                [ -n "$en_name" ] || continue
                local sig
                sig=$(echo "$line" | sed 's/^[[:space:]]*//' | cut -c1-120)
                echo "EN|$crate|$modpath|$en_name|$file:$linenum|$sig" >> "$outfile"
            done < "$matches"

            # Find pub trait declarations
            echo "$stripped" | grep -n '^[[:space:]]*pub[[:space:]]\+\(unsafe[[:space:]]\+\)\?trait[[:space:]]' > "$matches" 2>/dev/null || true
            while IFS=: read -r linenum line; do
                [ -z "$linenum" ] && continue
                local tr_name
                tr_name=$(echo "$line" | sed -n 's/.*pub[[:space:]]\+\(unsafe[[:space:]]\+\)\?trait[[:space:]]\+\([a-zA-Z_][a-zA-Z0-9_]*\).*/\2/p')
                [ -n "$tr_name" ] || continue
                local sig
                sig=$(echo "$line" | sed 's/^[[:space:]]*//' | cut -c1-120)
                echo "TR|$crate|$modpath|$tr_name|$file:$linenum|$sig" >> "$outfile"
            done < "$matches"

            # Find pub type aliases
            echo "$stripped" | grep -n '^[[:space:]]*pub[[:space:]]\+type[[:space:]]' > "$matches" 2>/dev/null || true
            while IFS=: read -r linenum line; do
                [ -z "$linenum" ] && continue
                local ty_name
                ty_name=$(echo "$line" | sed -n 's/.*pub[[:space:]]\+type[[:space:]]\+\([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/p')
                [ -n "$ty_name" ] || continue
                local sig
                sig=$(echo "$line" | sed 's/^[[:space:]]*//' | cut -c1-120)
                echo "TY|$crate|$modpath|$ty_name|$file:$linenum|$sig" >> "$outfile"
            done < "$matches"

            rm -f "$matches"

        done < <(find "$dir" -name '*.rs' -print0)
    done
}

# ── collect REQ-tagged tests ─────────────────────────────────────────────────

collect_req_tests() {
    local scopes="$1"
    local outfile="$2"

    local search_dirs=()
    if [ "$scopes" = "both" ] || [ "$scopes" = "crates" ]; then
        search_dirs+=("$repo_root/crates")
    fi
    if [ "$scopes" = "both" ] || [ "$scopes" = "mcp-servers" ]; then
        search_dirs+=("$repo_root/mcp-servers")
    fi

    > "$outfile"

    for dir in "${search_dirs[@]}"; do
        [ -d "$dir" ] || continue

        while IFS= read -r -d '' file; do
            [ -f "$file" ] || continue

            local crate modpath
            crate=$(crate_name_from_path "$file")
            modpath=$(module_path_from_file "$file")

            # Extract REQ annotations and the test function that follows them.
            # Pattern: // REQ: <id> — <description> followed by #[test] and fn
            local req_matches
            req_matches=$(mktemp)
            grep -n '// REQ:' "$file" > "$req_matches" 2>/dev/null || true
            while IFS=: read -r req_linenum req_line; do
                local req_id req_desc test_fn
                # Parse REQ line
                req_id=$(echo "$req_line" | sed -n 's/.*REQ:[[:space:]]*\([^[:space:]-]*\).*/\1/p')
                req_desc=$(echo "$req_line" | sed 's/.*REQ:[[:space:]]*[^[:space:]-]*[[:space:]-]*//' | sed 's/^[[:space:]]*//')
                [ -n "$req_id" ] || continue

                # Look ahead in the file for the test function (within ~20 lines after REQ)
                test_fn=""
                local lineno=$req_linenum
                while [ "$lineno" -lt $((req_linenum + 25)) ]; do
                    lineno=$((lineno + 1))
                    local next_line
                    next_line=$(sed -n "${lineno}p" "$file" 2>/dev/null || true)
                    [ -z "$next_line" ] && break
                    # Match fn <name>(...) where name starts with test_ or contains _test
                    if echo "$next_line" | grep -q 'fn[[:space:]]\+\(test_\|.*_test\)'; then
                        test_fn=$(echo "$next_line" | sed -n 's/.*fn[[:space:]]\+\([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/p')
                        break
                    fi
                    # Also match any fn in test context (within #[cfg(test)])
                    if echo "$next_line" | grep -q 'fn[[:space:]]\+[a-zA-Z_]'; then
                        test_fn=$(echo "$next_line" | sed -n 's/.*fn[[:space:]]\+\([a-zA-Z_][a-zA-Z0-9_]*\).*/\1/p')
                        break
                    fi
                done

                echo "REQ|$crate|$modpath|$req_id|$req_desc|$test_fn|$file:$req_linenum" >> "$outfile"
            done < "$req_matches"
            rm -f "$req_matches"
        done < <(find "$dir" -name '*.rs' -print0)
    done
}

# ── cross-reference: match public items to REQ tests ──────────────────────────

build_inventory() {
    local items_file="$1"
    local reqs_file="$2"

    # Build a lookup: which crate+modpath have REQ tests, and which item names
    # appear in REQ descriptions or test function names.
    local req_data
    req_data=$(mktemp)
    # Also build per-crate REQ count for summary
    local crate_req_counts
    crate_req_counts=$(mktemp)

    while IFS='|' read -r _ cr mp rid rdesc tfn loc; do
        # Record REQ presence per module
        echo "$cr|$mp" >> "$req_data"
        # Record REQ presence per crate
        echo "$cr" >> "$crate_req_counts"
        # Extract potential item name references from REQ description and test fn
        echo "$rid:$rdesc:$tfn" >> "${req_data}.terms"
    done < "$reqs_file"

    # Now process public items and determine coverage
    local crate_summary
    crate_summary=$(mktemp)

    # Group by crate
    local current_crate=""
    local crate_items=0
    local crate_covered=0

    # Output: markdown
    cat <<'HEADER'
# Public Seam Inventory

**Generated:** GENERATED_DATE
**Source:** `scripts/audit/public-seam-inventory.sh`
**Purpose:** P8 traceability — maps public API items to REQ-tagged test coverage.

Each public item is classified:
- 🟢 **Covered** — at least one `// REQ:` test in the same file or module
- 🔴 **Uncovered** — no REQ-tagged test found in the same file

---

## Summary

HEADER

    # Count totals
    local total_items=0
    local total_covered=0
    local total_crates=0
    local total_reqs=0

    total_reqs=$(wc -l < "$reqs_file" 2>/dev/null || echo 0)
    total_items=$(wc -l < "$items_file" 2>/dev/null || echo 0)

    # Per-crate counts
    echo "| Crate | Public Items | Covered | Uncovered | Coverage % | REQ Tests |"
    echo "|-------|-------------|---------|-----------|------------|-----------|"

    # Process items sorted by crate (use process substitution to avoid subshell)
    local crate=""
    local items=0
    local covered=0
    local crate_lines=""

    while IFS='|' read -r kind cr mp name loc sig; do
        [ -z "$kind" ] && continue
        if [ "$crate" != "" ] && [ "$cr" != "$crate" ]; then
            # Flush previous crate summary
            local pct=0
            [ "$items" -gt 0 ] && pct=$(( covered * 100 / items ))
            local reqs_in_crate
            reqs_in_crate=$(grep -c "^$crate|" "$reqs_file" 2>/dev/null || echo 0)
            echo "| $crate | $items | $covered | $((items - covered)) | ${pct}% | $reqs_in_crate |"

            # Emit detailed items for previous crate
            echo ""
            echo "### $crate"
            echo ""
            echo "| Kind | Item | Module | Location | REQ Coverage |"
            echo "|------|------|--------|----------|-------------|"
            echo "$crate_lines"

            total_covered=$((total_covered + covered))
            total_crates=$((total_crates + 1))

            crate="$cr"
            items=1
            covered=0
            crate_lines=""
        else
            crate="$cr"
            items=$((items + 1))
        fi

        # Determine coverage: item covered if its module/file has any REQ tests
        # OR if the item name appears in a REQ description or test function name
        local is_covered=false
        local coverage_marker="🔴"

        # Check: does this module have any REQ tests? (|| true for pipefail)
        if grep -qF "$cr|$mp" "$req_data" 2>/dev/null || [ $? -eq 1 ]; then
            if [ $? -eq 0 ]; then
                is_covered=true
            fi
        fi

        # Check: does the item name appear in any REQ description or test fn name?
        if [ "$is_covered" = false ]; then
            if grep -qi "$name" "${req_data}.terms" 2>/dev/null; then
                is_covered=true
            fi
        fi

        if $is_covered; then
            coverage_marker="🟢"
            covered=$((covered + 1))
        fi

        local kind_label
        case "$kind" in
            FN) kind_label="fn" ;;
            ST) kind_label="struct" ;;
            EN) kind_label="enum" ;;
            TR) kind_label="trait" ;;
            TY) kind_label="type" ;;
            *)  kind_label="$kind" ;;
        esac

        crate_lines="${crate_lines}| $kind_label | \`$name\` | $mp | $loc | $coverage_marker |
"
    done < <(sort -t'|' -k2,2 -k3,3 "$items_file")

    # Flush last crate
    if [ "$crate" != "" ]; then
        local pct=0
        [ "$items" -gt 0 ] && pct=$(( covered * 100 / items ))
        local reqs_in_crate
        reqs_in_crate=$(grep -c "^$crate|" "$reqs_file" 2>/dev/null || echo 0)
        echo "| $crate | $items | $covered | $((items - covered)) | ${pct}% | $reqs_in_crate |"
        echo ""
        echo "### $crate"
        echo ""
        echo "| Kind | Item | Module | Location | REQ Coverage |"
        echo "|------|------|--------|----------|-------------|"
        echo "$crate_lines"

        total_covered=$((total_covered + covered))
        total_crates=$((total_crates + 1))
    fi

    # Emit overall summary footer
    local overall_pct=0
    [ "$total_items" -gt 0 ] && overall_pct=$(( total_covered * 100 / total_items ))
    echo ""
    echo "---"
    echo ""
    echo "## Totals"
    echo ""
    echo "| Metric | Value |"
    echo "|--------|-------|"
    echo "| Total public items | $total_items |"
    echo "| Covered (🟢) | $total_covered |"
    echo "| Uncovered (🔴) | $((total_items - total_covered)) |"
    echo "| Overall coverage | ${overall_pct}% |"
    echo "| Total REQ-tagged tests | $total_reqs |"
    echo "| Crates analyzed | $total_crates |"

    # Cleanup
    rm -f "$req_data" "${req_data}.terms" "$crate_req_counts"
}

# ── main ──────────────────────────────────────────────────────────────────────

main() {
    local items_file reqs_file
    items_file=$(mktemp)
    reqs_file=$(mktemp)
    local new_output
    new_output=$(mktemp)

    echo "Scanning public items..." >&2
    collect_public_items "both" "$items_file"

    echo "Scanning REQ-tagged tests..." >&2
    collect_req_tests "both" "$reqs_file"

    local item_count req_count
    item_count=$(wc -l < "$items_file" 2>/dev/null || echo 0)
    req_count=$(wc -l < "$reqs_file" 2>/dev/null || echo 0)
    echo "Found $item_count public items, $req_count REQ-tagged tests." >&2

    echo "Generating inventory..." >&2
    build_inventory "$items_file" "$reqs_file" > "$new_output"

    # Replace GENERATED_DATE placeholder
    local gen_date
    gen_date=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    sed "s/GENERATED_DATE/$gen_date/" "$new_output" > "${new_output}.dated" && mv "${new_output}.dated" "$new_output"

    # Mode: --write overwrites, --check compares
    if [ "$mode" = "--write" ]; then
        cp "$new_output" "$output"
        echo "Inventory written to $output ($item_count items, $req_count REQ tests)" >&2
        rm -f "$items_file" "$reqs_file" "$new_output"
        exit 0
    fi

    if [ ! -f "$output" ]; then
        echo "ERROR: Inventory file does not exist at $output" >&2
        echo "Run with --write to generate it first." >&2
        rm -f "$items_file" "$reqs_file" "$new_output"
        exit 1
    fi

    # Strip the generated date from both before comparing (dates change every run)
    local existing_stripped new_stripped
    existing_stripped=$(mktemp)
    new_stripped=$(mktemp)
    sed 's/\*\*Generated:\*\*.*//' "$output" > "$existing_stripped"
    sed 's/\*\*Generated:\*\*.*//' "$new_output" > "$new_stripped"

    if diff -q "$existing_stripped" "$new_stripped" > /dev/null 2>&1; then
        echo "OK: Public seam inventory is current ($item_count items, $req_count REQ tests)" >&2
        rm -f "$items_file" "$reqs_file" "$new_output" "$existing_stripped" "$new_stripped"
        exit 0
    else
        echo "DRIFT: Public seam inventory is out of date." >&2
        echo "Run 'scripts/audit/public-seam-inventory.sh --write' to regenerate." >&2
        echo "" >&2
        echo "Diff:" >&2
        diff "$existing_stripped" "$new_stripped" >&2 || true
        rm -f "$items_file" "$reqs_file" "$new_output" "$existing_stripped" "$new_stripped"
        exit 1
    fi
}

main
