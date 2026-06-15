#!/usr/bin/env bash
# docs/ci/regenerate-corpus-inventory.sh — Regenerate corpus_inventory.yaml skeleton
# Usage: bash docs/ci/regenerate-corpus-inventory.sh [--output FILE]
#
# Scans all docs/ for .md/.yaml files, extracts YAML frontmatter fields,
# and produces a corpus_inventory.yaml skeleton. Mechanical fields (path,
# category, status, version, mds_categories, last_updated) are populated
# automatically. Classification fields (staleness_signal, governing_principles,
# disposition, notes) are left as TODO for manual/agent review.
#
# Output goes to stdout by default; use --output to write to a file.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$DOCS_DIR/.." && pwd)"

OUTPUT=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output) OUTPUT="$2"; shift 2 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# ── Helper: extract frontmatter field ───────────────────────────────────
extract_field() {
    local file="$1" field="$2" default="$3"
    local val
    val=$(head -30 "$file" 2>/dev/null | awk '/^---$/{f=1; next} /^---$/{f=0} f==1 && /^'"$field"':/{print}' | head -1 | sed "s/^$field: *//" | tr -d '"' | xargs)
    echo "${val:-$default}"
}

# ── Helper: infer category from path ─────────────────────────────────────
infer_category() {
    local rel="$1"
    case "$rel" in
        architecture/*)    echo "architecture" ;;
        specifications/*)  echo "specification" ;;
        plans/*)           echo "plan" ;;
        status/*)          echo "status" ;;
        guides/*)          echo "guide" ;;
        user-guides/*)     echo "user-guide" ;;
        research/*)        echo "research" ;;
        handoffs/*)        echo "handoff" ;;
        *)                 echo "cross-cutting" ;;
    esac
}

# ── Generate YAML header ────────────────────────────────────────────────
now=$(date -I)
workspace_version=$(grep -oP '^version\s*=\s*"\K[^"]+' "$PROJECT_ROOT/Cargo.toml" | head -1)

exec 3>&1
if [[ -n "$OUTPUT" ]]; then
    exec 1>"$OUTPUT"
fi

cat <<YAML_HEADER
# hKask Corpus Inventory — Auto-generated skeleton
# Generated: $now
# Workspace version: $workspace_version
#
# Mechanical fields (path, category, status, version, mds_categories,
# last_updated) are populated from document frontmatter.
# Classification fields (staleness_signal, governing_principles,
# disposition, notes) are marked TODO — fill manually or via agent sweep.
#
# Regenerate: bash docs/ci/regenerate-corpus-inventory.sh --output docs/status/corpus_inventory.yaml

documents:
YAML_HEADER

# ── Scan documents ──────────────────────────────────────────────────────
count=0
active=0
stale=0
with_frontmatter=0
without_frontmatter=0

while IFS= read -r -d '' file; do
    # Skip excluded directories
    if [[ "$file" == *"/docs/archive/"* ]]; then continue; fi
    if [[ "$file" == *"/docs/ci/"* ]]; then continue; fi
    if [[ "$file" == *"/docs/generated/"* ]]; then continue; fi
    if [[ "$file" == *"/docs/handoffs/"* ]]; then continue; fi
    if [[ "$file" == *"/.agents/"* ]]; then continue; fi

    # Only .md and .yaml files
    if [[ "$file" != *.md ]] && [[ "$file" != *.yaml ]]; then continue; fi

    rel="${file#$DOCS_DIR/}"
    count=$((count + 1))

    # Check for YAML frontmatter
    has_fm=false
    if head -1 "$file" 2>/dev/null | grep -q '^---$'; then
        has_fm=true
        with_frontmatter=$((with_frontmatter + 1))
    else
        without_frontmatter=$((without_frontmatter + 1))
    fi

    category=$(infer_category "$rel")

    if [[ "$has_fm" == true ]]; then
        status=$(extract_field "$file" "status" "Draft")
        version=$(extract_field "$file" "version" "$workspace_version")
        mds_categories=$(extract_field "$file" "mds_categories" "TODO")
        last_updated=$(extract_field "$file" "last_updated" "$now")
    else
        status="Draft"
        version="$workspace_version"
        mds_categories="TODO"
        last_updated="$now"
    fi

    # Track active vs stale
    if [[ "$status" == "Active" ]]; then
        active=$((active + 1))
    elif [[ "$status" == "Deprecated" ]] || [[ "$status" == "Superseded" ]]; then
        stale=$((stale + 1))
    fi

    cat <<YAML_ENTRY
  - path: $rel
    category: $category
    status: $status
    last_updated: $last_updated
    version: $version
    mds_categories: $mds_categories
    staleness_signal: TODO
    governing_principles: TODO
    disposition: keep
    notes: TODO
YAML_ENTRY

done < <(find "$DOCS_DIR" -type f \( -name "*.md" -o -name "*.yaml" \) -not -path "*/archive/*" -not -path "*/ci/*" -not -path "*/generated/*" -not -path "*/handoffs/*" -print0 | sort -z)

# ── Summary ─────────────────────────────────────────────────────────────
cat <<YAML_SUMMARY

summary:
  total_documents: $count
  active_keep: $active
  stale_refresh: $stale
  archived: 0
  newly_created: 0
  total_with_frontmatter: $with_frontmatter
  total_without_frontmatter: $without_frontmatter
  missing_referenced: TODO
  version_anomalies: TODO
  documents_not_in_master_index: TODO
  documents_not_in_scaffold: TODO
  documents_not_in_portal: TODO
  notes: "Auto-generated skeleton — TODO fields require manual/agent classification sweep."
YAML_SUMMARY

# Restore stdout
if [[ -n "$OUTPUT" ]]; then
    exec 1>&3
    echo "Corpus inventory skeleton written to: $OUTPUT"
    echo "Documents: $count (active: $active, stale: $stale)"
    echo "With frontmatter: $with_frontmatter"
    echo "Without frontmatter: $without_frontmatter"
    echo "TODO fields require manual/agent classification sweep."
fi
