#!/bin/bash
# Public seam inventory generator — Phase 2 Task 1 / PR 2.1.1
#
# Walks all .rs files in crates/ and mcp-servers/, extracts public items
# (pub fn, pub struct, pub enum, pub trait, pub type), and cross-references
# with // REQ:-tagged tests to produce a coverage-linked inventory.
#
# Output:
#   docs/status/public-seam-inventory.md  — human-readable markdown
#   docs/status/public-seam-inventory.json — machine-readable JSON (CNS/R7.3)
# Exit 0 if inventory matches existing file, exit 1 on drift (CI gate).
#
# Usage:
#   scripts/public-seam-inventory.sh            # Generate + compare
#   scripts/public-seam-inventory.sh --write     # Generate + overwrite
#   scripts/public-seam-inventory.sh --check     # CI mode: fail on drift

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
output="$repo_root/docs/status/public-seam-inventory.md"
json_output="$repo_root/docs/status/public-seam-inventory.json"
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

# ── JSON helpers ────────────────────────────────────────────────────────────────

# Escape a string for JSON (backslash, double-quote, control chars).
json_escape() {
    local s="${1-}"
    # Backslash must be first to avoid re-escaping
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//$'\n'/\\n}"
    s="${s//$'\r'/\\r}"
    s="${s//$'\t'/\\t}"
    printf '%s' "$s"
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
                local relfile="${file#$repo_root/}"
                echo "FN|$crate|$modpath|$fn_name|$relfile:$linenum|$sig" >> "$outfile"
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
                local relfile="${file#$repo_root/}"
                echo "ST|$crate|$modpath|$st_name|$relfile:$linenum|$sig" >> "$outfile"
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
                local relfile="${file#$repo_root/}"
                echo "EN|$crate|$modpath|$en_name|$relfile:$linenum|$sig" >> "$outfile"
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
                local relfile="${file#$repo_root/}"
                echo "TR|$crate|$modpath|$tr_name|$relfile:$linenum|$sig" >> "$outfile"
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
                local relfile="${file#$repo_root/}"
                echo "TY|$crate|$modpath|$ty_name|$relfile:$linenum|$sig" >> "$outfile"
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

                local relfile="${file#$repo_root/}"
                echo "REQ|$crate|$modpath|$req_id|$req_desc|$test_fn|$relfile:$req_linenum" >> "$outfile"
            done < "$req_matches"
            rm -f "$req_matches"
        done < <(find "$dir" -name '*.rs' -print0)
    done
}

# ── risk classification ──────────────────────────────────────────────────────

# Classify a public item by risk tier: high, medium, or low.
# Output format: "tier:category" for use in inventory tables.
classify_risk() {
    local kind="$1" crate="$2" fn_name="$3"

    case "$kind" in
        ST|EN|TR|TY)
            echo "medium:Type Declaration"
            ;;
        FN)
            # Accessor/constructor patterns — low risk individually
            case "$fn_name" in
                new|new_*|from_*|with_*|as_*|to_*|into_*|\
                is_*|has_*|get_*|set_*|try_*|default|\
                builder|build|len|is_empty|iter|iter_mut|\
                run|start|stop|shutdown|close|open)
                    echo "low:Accessor/Constructor"
                    ;;
                *)
                    # Context-based classification
                    case "$crate" in
                        hkask-api)
                            echo "high:API Route Handler"
                            ;;
                        hkask-mcp-*)
                            echo "high:MCP Tool Handler"
                            ;;
                        hkask-mcp)
                            echo "high:Core Logic"
                            ;;
                        *)
                            echo "high:Core Logic"
                            ;;
                    esac
                    ;;
            esac
            ;;
        *) echo "medium:Unknown" ;;
    esac
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
        # Extract potential item name references, scoped by crate
        echo "$cr:$rid:$rdesc:$tfn" >> "${req_data}.terms"
    done < "$reqs_file"

    # Now process public items and determine coverage

    # Group by crate
    local current_crate=""
    local crate_covered=0

    # Output: markdown
    cat <<'HEADER'
# Public Seam Inventory

**Generated:** GENERATED_DATE
**Source:** `scripts/public-seam-inventory.sh`
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
            reqs_in_crate=$(grep -cF "REQ|${crate}|" "$reqs_file" 2>/dev/null || true)
            echo "| $crate | $items | $covered | $((items - covered)) | ${pct}% | $reqs_in_crate |"

            # Emit detailed items for previous crate
            echo ""
            echo "### $crate"
            echo ""
            echo "| Kind | Item | Module | Location | Risk Tier | REQ |"
            echo "|------|------|--------|----------|-----------|-----|"
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

        # Check: does this module have any REQ tests?
        if grep -qFx "$cr|$mp" "$req_data" 2>/dev/null; then
            is_covered=true
        fi

        # Check: does the item name appear in this crate's REQ descriptions or test fn names?
        # Terms file format: crate:req_id:description:test_fn
        if [ "$is_covered" = false ] && grep -qi "^${cr}:.*${name}" "${req_data}.terms" 2>/dev/null; then
            is_covered=true
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

        local risk_tier
        risk_tier=$(classify_risk "$kind" "$cr" "$name")
        local risk_label="${risk_tier%%:*}"
        local risk_cat="${risk_tier#*:}"
        local risk_icon
        case "$risk_label" in
            high) risk_icon="🔴" ;;
            medium) risk_icon="🟡" ;;
            low) risk_icon="🟢" ;;
            *) risk_icon="⚪" ;;
        esac

        crate_lines="${crate_lines}| $kind_label | \`$name\` | $mp | $loc | $risk_icon $risk_cat | $coverage_marker |
"
    done < <(sort -t'|' -k2,2 -k3,3 "$items_file")

    # Flush last crate
    if [ "$crate" != "" ]; then
        local pct=0
        [ "$items" -gt 0 ] && pct=$(( covered * 100 / items ))
        local reqs_in_crate
        reqs_in_crate=$(grep -cF "REQ|${crate}|" "$reqs_file" 2>/dev/null || true)
        echo "| $crate | $items | $covered | $((items - covered)) | ${pct}% | $reqs_in_crate |"
        echo ""
        echo "### $crate"
        echo ""
        echo "| Kind | Item | Module | Location | Risk Tier | REQ |"
        echo "|------|------|--------|----------|-----------|-----|"
        echo "$crate_lines"

        total_covered=$((total_covered + covered))
        total_crates=$((total_crates + 1))
    fi

    # Generate priority list from the same data
    generate_priority_list "$items_file" "$reqs_file"

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

# ── JSON inventory generation (machine-readable, CNS/R7.3 observability) ──────

# Build a comprehensive JSON inventory from the same items_file and reqs_file.
# Output: JSON on stdout — caller redirects to file.
build_json_inventory() {
    local items_file="$1"
    local reqs_file="$2"

    local gen_date
    gen_date=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    # Pre-compute per-module REQ coverage lookup
    local req_data
    req_data=$(mktemp)
    local req_terms
    req_terms=$(mktemp)
    local crate_req_counts
    crate_req_counts=$(mktemp)

    while IFS='|' read -r _ cr mp rid rdesc tfn loc; do
        echo "$cr|$mp" >> "$req_data"
        echo "$cr" >> "$crate_req_counts"
        echo "$cr:$rid:$rdesc:$tfn" >> "${req_terms}"
    done < "$reqs_file"

    # ── Build per-crate data ──
    local crate_json=""
    local current_crate=""
    local crate_items_json=""
    local crate_total=0
    local crate_covered=0
    local crate_high_risk_uncovered=0
    local first_crate=true

    # Also collect high-risk uncovered for priority section
    local priority_items=""
    local priority_count=0

    while IFS='|' read -r kind cr mp name loc sig; do
        [ -z "$kind" ] && continue

        # Flush previous crate when crate changes
        if [ "$current_crate" != "" ] && [ "$cr" != "$current_crate" ]; then
            local pct=0
            [ "$crate_total" -gt 0 ] && pct=$(( crate_covered * 100 / crate_total ))
            local reqs_in_crate
            reqs_in_crate=$(grep -cF "REQ|${current_crate}|" "$reqs_file" 2>/dev/null || true)

            local crate_entry
            crate_entry=$(printf '{"total_items":%d,"covered":%d,"uncovered":%d,"coverage_pct":%d,"req_tests":%d,"high_risk_uncovered":%d,"items":[%s]}' \
                "$crate_total" "$crate_covered" "$((crate_total - crate_covered))" "$pct" "$reqs_in_crate" "$crate_high_risk_uncovered" "$crate_items_json")

            if $first_crate; then
                first_crate=false
                crate_json="\"$(json_escape "$current_crate")\":$crate_entry"
            else
                crate_json="$crate_json,\"$(json_escape "$current_crate")\":$crate_entry"
            fi

            current_crate="$cr"
            crate_total=1
            crate_covered=0
            crate_high_risk_uncovered=0
            crate_items_json=""
        else
            current_crate="$cr"
            crate_total=$((crate_total + 1))
        fi

        # Determine coverage
        local is_covered=false
        if grep -qFx "$cr|$mp" "$req_data" 2>/dev/null; then
            is_covered=true
        fi
        if [ "$is_covered" = false ] && grep -qi "^${cr}:.*${name}" "${req_terms}" 2>/dev/null; then
            is_covered=true
        fi

        if $is_covered; then
            crate_covered=$((crate_covered + 1))
        fi

        # Risk classification
        local risk_tier
        risk_tier=$(classify_risk "$kind" "$cr" "$name")
        local risk_label="${risk_tier%%:*}"
        local risk_cat="${risk_tier#*:}"

        # Track high-risk uncovered for priority
        if [ "$is_covered" = false ] && [ "$risk_label" = "high" ]; then
            crate_high_risk_uncovered=$((crate_high_risk_uncovered + 1))
            if [ "$priority_count" -lt 100 ]; then
                priority_count=$((priority_count + 1))
                local kind_label
                case "$kind" in
                    FN) kind_label="fn" ;;
                    ST) kind_label="struct" ;;
                    EN) kind_label="enum" ;;
                    TR) kind_label="trait" ;;
                    TY) kind_label="type" ;;
                    *)  kind_label="$kind" ;;
                esac
                local escaped_name escaped_mp escaped_loc escaped_cat
                escaped_name=$(json_escape "$name")
                escaped_mp=$(json_escape "$mp")
                escaped_loc=$(json_escape "$loc")
                escaped_cat=$(json_escape "$risk_cat")
                local escaped_crate
                escaped_crate=$(json_escape "$cr")
                local pri_entry
                pri_entry=$(printf '{"rank":%d,"crate":"%s","kind":"%s","name":"%s","module":"%s","location":"%s","risk_category":"%s"}' \
                    "$priority_count" "$escaped_crate" "$kind_label" "$escaped_name" "$escaped_mp" "$escaped_loc" "$escaped_cat")
                if [ -z "$priority_items" ]; then
                    priority_items="$pri_entry"
                else
                    priority_items="$priority_items,$pri_entry"
                fi
            fi
        fi

        # Build item JSON entry
        local kind_label
        case "$kind" in
            FN) kind_label="fn" ;;
            ST) kind_label="struct" ;;
            EN) kind_label="enum" ;;
            TR) kind_label="trait" ;;
            TY) kind_label="type" ;;
            *)  kind_label="$kind" ;;
        esac

        local covered_str
        if $is_covered; then covered_str="true"; else covered_str="false"; fi

        local escaped_name escaped_mp escaped_loc escaped_cat
        escaped_name=$(json_escape "$name")
        escaped_mp=$(json_escape "$mp")
        escaped_loc=$(json_escape "$loc")
        escaped_cat=$(json_escape "$risk_cat")

        local item_entry
        item_entry=$(printf '{"kind":"%s","name":"%s","module":"%s","location":"%s","risk_tier":"%s","risk_category":"%s","covered":%s,"req_tags":[]}' \
            "$kind_label" "$escaped_name" "$escaped_mp" "$escaped_loc" "$risk_label" "$escaped_cat" "$covered_str")

        if [ -z "$crate_items_json" ]; then
            crate_items_json="$item_entry"
        else
            crate_items_json="$crate_items_json,$item_entry"
        fi
    done < <(sort -t'|' -k2,2 -k3,3 "$items_file")

    # Flush last crate
    if [ "$current_crate" != "" ]; then
        local pct=0
        [ "$crate_total" -gt 0 ] && pct=$(( crate_covered * 100 / crate_total ))
        local reqs_in_crate
        reqs_in_crate=$(grep -cF "REQ|${current_crate}|" "$reqs_file" 2>/dev/null || true)

        local crate_entry
        crate_entry=$(printf '{"total_items":%d,"covered":%d,"uncovered":%d,"coverage_pct":%d,"req_tests":%d,"high_risk_uncovered":%d,"items":[%s]}' \
            "$crate_total" "$crate_covered" "$((crate_total - crate_covered))" "$pct" "$reqs_in_crate" "$crate_high_risk_uncovered" "$crate_items_json")

        if $first_crate; then
            crate_json="\"$(json_escape "$current_crate")\":$crate_entry"
        else
            crate_json="$crate_json,\"$(json_escape "$current_crate")\":$crate_entry"
        fi
    fi

    # ── Compute totals ──
    local total_items=0 total_covered=0 total_crates=0 total_reqs=0
    total_items=$(wc -l < "$items_file" 2>/dev/null || echo 0)
    total_reqs=$(wc -l < "$reqs_file" 2>/dev/null || echo 0)

    # Count covered items and crates from the data we just processed
    # Re-scan to get accurate totals (the per-crate loop tracked these)
    local all_covered=0 all_crates=0
    while IFS='|' read -r kind cr mp name loc sig; do
        [ -z "$kind" ] && continue
        local is_cov=false
        if grep -qFx "$cr|$mp" "$req_data" 2>/dev/null; then is_cov=true; fi
        if [ "$is_cov" = false ] && grep -qi "^${cr}:.*${name}" "${req_terms}" 2>/dev/null; then is_cov=true; fi
        if $is_cov; then all_covered=$((all_covered + 1)); fi
    done < "$items_file"
    all_crates=$(cut -d'|' -f2 "$items_file" | sort -u | wc -l)

    local overall_pct=0
    [ "$total_items" -gt 0 ] && overall_pct=$(( all_covered * 100 / total_items ))

    # ── Per-crate priority counts ──
    local pri_per_crate_json=""
    local pri_first=true
    # Re-use the high-risk uncovered counts we tracked per crate
    # We need to rebuild from items_file since we only tracked in the loop above
    local pri_crate_counts
    pri_crate_counts=$(mktemp)
    while IFS='|' read -r kind cr mp name loc sig; do
        [ -z "$kind" ] && continue
        local is_cov=false
        if grep -qFx "$cr|$mp" "$req_data" 2>/dev/null; then is_cov=true; fi
        if [ "$is_cov" = false ] && grep -qi "^${cr}:.*${name}" "${req_terms}" 2>/dev/null; then is_cov=true; fi
        if [ "$is_cov" = false ]; then
            local rt
            rt=$(classify_risk "$kind" "$cr" "$name")
            local rl="${rt%%:*}"
            if [ "$rl" = "high" ]; then
                echo "$cr" >> "$pri_crate_counts"
            fi
        fi
    done < "$items_file"

    local prev_crate=""
    while IFS= read -r pc; do
        if [ "$pc" != "$prev_crate" ]; then
            local cnt
            cnt=$(grep -cFx "$pc" "$pri_crate_counts" 2>/dev/null || echo 0)
            local escaped_pc
            escaped_pc=$(json_escape "$pc")
            if $pri_first; then
                pri_first=false
                pri_per_crate_json="\"$escaped_pc\":$cnt"
            else
                pri_per_crate_json="$pri_per_crate_json,\"$escaped_pc\":$cnt"
            fi
            prev_crate="$pc"
        fi
    done < <(sort "$pri_crate_counts" | uniq)
    rm -f "$pri_crate_counts"

    # ── Emit JSON ──
    local escaped_gen_date
    escaped_gen_date=$(json_escape "$gen_date")

    cat <<JSONEOF
{
  "generated": "$escaped_gen_date",
  "source": "scripts/public-seam-inventory.sh",
  "purpose": "P8 traceability — machine-readable public seam inventory for CNS observability (R7.3 watcher)",
  "totals": {
    "total_items": $total_items,
    "covered": $all_covered,
    "uncovered": $((total_items - all_covered)),
    "coverage_pct": ${overall_pct}.0,
    "req_tests": $total_reqs,
    "crates_analyzed": $all_crates
  },
  "crates": {
    $crate_json
  },
  "priority": {
    "total_high_risk_uncovered": $priority_count,
    "top_100": [
      $priority_items
    ],
    "per_crate_counts": {
      $pri_per_crate_json
    }
  }
}
JSONEOF

    rm -f "$req_data" "${req_terms}" "$crate_req_counts"
}

# ── priority list generation ─────────────────────────────────────────────────

generate_priority_list() {
    local items_file="$1"
    local reqs_file="$2"

    local priority_output="$repo_root/docs/status/public-seam-priority.md"
    local temp_priority
    temp_priority=$(mktemp)

    # Collect uncovered items with risk tier classification
    while IFS='|' read -r kind cr mp name loc sig; do
        [ -z "$kind" ] && continue

        # Determine coverage (same logic as build_inventory)
        local is_covered=false

        # Check module-level REQ coverage
        if grep -qFx "$cr|$mp" "$reqs_file.module_data" 2>/dev/null; then
            is_covered=true
        fi

        if [ "$is_covered" = false ]; then
            local risk_tier
            risk_tier=$(classify_risk "$kind" "$cr" "$name")
            local risk_label="${risk_tier%%:*}"

            # Only include high-risk uncovered items
            if [ "$risk_label" = "high" ]; then
                local kind_label
                case "$kind" in
                    FN) kind_label="fn" ;;
                    ST) kind_label="struct" ;;
                    EN) kind_label="enum" ;;
                    TR) kind_label="trait" ;;
                    TY) kind_label="type" ;;
                    *)  kind_label="$kind" ;;
                esac
                echo "$risk_tier|$cr|$kind_label|$name|$mp|$loc" >> "$temp_priority"
            fi
        fi
    done < "$items_file"

    local priority_count
    priority_count=$(wc -l < "$temp_priority" 2>/dev/null || echo 0)

    cat > "$priority_output" <<PRIORITY_HEADER
# Public Seam Priority List

**Generated:** $(date -u +"%Y-%m-%dT%H:%M:%SZ")
**Source:** \`scripts/public-seam-inventory.sh\`
**Purpose:** Top high-risk uncovered public items requiring REQ-tagged tests.

Items are classified as **high risk** when they are:
- API route handlers (\`hkask-api\`)
- MCP tool handlers (\`hkask-mcp-*\` servers)
- Core logic functions in other crates (non-accessor/constructor patterns)

Accessors, constructors, and type declarations are excluded — they are low/medium
risk and typically covered by struct-level or integration tests.

---

## Top High-Risk Uncovered Items (top 100)

| # | Crate | Kind | Item | Module | Location | Category |
|---|-------|------|------|--------|----------|----------|
PRIORITY_HEADER

    # Output top 100, sorted by crate then item name
    local sorted_priority
    sorted_priority=$(mktemp)
    sort -t'|' -k2,2 -k4,4 "$temp_priority" > "$sorted_priority"
    local count=0
    while IFS='|' read -r risk cr kind name mp loc; do
        [ -z "$risk" ] && continue
        [ "$count" -ge 100 ] && break
        count=$((count + 1))
        local risk_cat="${risk#*:}"
        echo "| $count | $cr | $kind | \`$name\` | $mp | $loc | $risk_cat |"
    done < "$sorted_priority" >> "$priority_output"
    rm -f "$sorted_priority"

    # Summary per crate
    echo "" >> "$priority_output"
    echo "---" >> "$priority_output"
    echo "" >> "$priority_output"
    echo "## Per-Crate High-Risk Uncovered Count" >> "$priority_output"
    echo "" >> "$priority_output"
    echo "| Crate | High-Risk Uncovered |" >> "$priority_output"
    echo "|-------|--------------------|" >> "$priority_output"

    cut -d'|' -f2 "$temp_priority" | sort | uniq -c | sort -rn | \
    while read -r cnt cr; do
        echo "| $cr | $cnt |"
    done >> "$priority_output"

    echo "" >> "$priority_output"
    echo "**Total high-risk uncovered:** $priority_count" >> "$priority_output"

    rm -f "$temp_priority"
}

# ── main ──────────────────────────────────────────────────────────────────────

main() {
    local items_file reqs_file
    items_file=$(mktemp)
    reqs_file=$(mktemp)
    local new_output new_json
    new_output=$(mktemp)
    new_json=$(mktemp)

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

    echo "Generating JSON inventory..." >&2
    build_json_inventory "$items_file" "$reqs_file" > "$new_json"

    # Mode: --write overwrites, --check compares
    if [ "$mode" = "--write" ]; then
        cp "$new_output" "$output"
        cp "$new_json" "$json_output"
        echo "Inventory written to $output ($item_count items, $req_count REQ tests)" >&2
        echo "JSON inventory written to $json_output" >&2
        rm -f "$items_file" "$reqs_file" "$new_output" "$new_json"
        exit 0
    fi

    if [ ! -f "$output" ]; then
        echo "ERROR: Inventory file does not exist at $output" >&2
        echo "Run with --write to generate it first." >&2
        rm -f "$items_file" "$reqs_file" "$new_output" "$new_json"
        exit 1
    fi

    # Strip the generated date from both before comparing (dates change every run)
    local existing_stripped new_stripped
    existing_stripped=$(mktemp)
    new_stripped=$(mktemp)
    sed 's/\*\*Generated:\*\*.*//' "$output" > "$existing_stripped"
    sed 's/\*\*Generated:\*\*.*//' "$new_output" > "$new_stripped"

    local md_ok=true json_ok=true

    if diff -q "$existing_stripped" "$new_stripped" > /dev/null 2>&1; then
        echo "OK: Public seam inventory (markdown) is current ($item_count items, $req_count REQ tests)" >&2
    else
        md_ok=false
        echo "DRIFT: Public seam inventory (markdown) is out of date." >&2
        echo "" >&2
        echo "Markdown diff:" >&2
        diff "$existing_stripped" "$new_stripped" >&2 || true
    fi

    # Check JSON drift — strip "generated" field before comparing
    if [ -f "$json_output" ]; then
        local existing_json_stripped new_json_stripped
        existing_json_stripped=$(mktemp)
        new_json_stripped=$(mktemp)
        # Remove the "generated" line (timestamps change every run)
        grep -v '"generated"' "$json_output" > "$existing_json_stripped" 2>/dev/null || true
        grep -v '"generated"' "$new_json" > "$new_json_stripped" 2>/dev/null || true

        if diff -q "$existing_json_stripped" "$new_json_stripped" > /dev/null 2>&1; then
            echo "OK: Public seam inventory (JSON) is current" >&2
        else
            json_ok=false
            echo "DRIFT: Public seam inventory (JSON) is out of date." >&2
            echo "" >&2
            echo "JSON diff:" >&2
            diff "$existing_json_stripped" "$new_json_stripped" >&2 || true
        fi
        rm -f "$existing_json_stripped" "$new_json_stripped"
    else
        echo "WARNING: JSON inventory file does not exist at $json_output" >&2
        echo "Run with --write to generate it." >&2
        json_ok=false
    fi

    rm -f "$items_file" "$reqs_file" "$new_output" "$new_json" "$existing_stripped" "$new_stripped"

    if $md_ok && $json_ok; then
        exit 0
    else
        echo "" >&2
        echo "Run 'scripts/public-seam-inventory.sh --write' to regenerate." >&2
        exit 1
    fi
}

main
