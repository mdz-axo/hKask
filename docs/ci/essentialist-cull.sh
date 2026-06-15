#!/usr/bin/env bash
# docs/ci/essentialist-cull.sh — Essentialist auto-culling: find unreferenced documents
# Usage: bash docs/ci/essentialist-cull.sh [--verbose]
#   --verbose: Show referenced files in addition to unreferenced
#
# Principle P5 (Essentialism): Documents that are not referenced from any
# index (portal, architecture master, AGENTS.md) are candidates for archival.
# This script performs the deletion test at the document level — if a document
# vanishes from all indexes, does any navigational behavior break?
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$DOCS_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

VERBOSE=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --verbose) VERBOSE=true; shift ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# ── Index documents (the authoritative reference surfaces) ──────────────
# These are the only documents whose references count as "indexed."
# A document referenced from any of these is considered alive.
INDEXES=(
    "$DOCS_DIR/README.md"
    "$DOCS_DIR/architecture/hKask-architecture-master.md"
    "$PROJECT_ROOT/AGENTS.md"
)

# Verify indexes exist
for idx in "${INDEXES[@]}"; do
    if [[ ! -f "$idx" ]]; then
        echo "ERROR: Index document not found: $idx"
        exit 1
    fi
done

echo "=== hKask Essentialist Auto-Culling ==="
echo "Index documents: ${#INDEXES[@]}"
echo ""

# ── Step 1: Collect all document paths from indexes ─────────────────────
# Extract paths from markdown links: [text](path) and bare paths like `docs/path`
# Normalize all paths to be relative to docs/ directory

declare -A REFERENCED  # Maps normalized path → source index

for idx in "${INDEXES[@]}"; do
    idx_name=$(basename "$idx")
    idx_dir=$(dirname "$idx")

    # Extract paths from markdown links: [text](relative/path)
    # grep outputs: ](path)
    while IFS= read -r raw; do
        if [[ -z "$raw" ]]; then continue; fi
        # Strip '](' prefix and ')' suffix using bash parameter expansion
        link="${raw#](}"
        link="${link%)}"
        if [[ -z "$link" ]]; then continue; fi
        # Resolve relative path from the index document's directory
        abs_path=$(cd "$idx_dir" 2>/dev/null && realpath -m "$link" 2>/dev/null) || continue
        # Convert to docs/-relative path
        if [[ "$abs_path" == "$DOCS_DIR"/* ]]; then
            rel="${abs_path#$DOCS_DIR/}"
            REFERENCED["$rel"]="$idx_name"
        fi
    done < <(grep -oP '\]\([^)]+\)' "$idx" | grep -v '^http' | grep -v '^#' | sort -u)

    # Extract bare paths: `docs/path/to/file.md` (backtick-quoted)
    while IFS= read -r bare; do
        if [[ -z "$bare" ]]; then continue; fi
        # Strip leading docs/ if present
        bare="${bare#docs/}"
        REFERENCED["$bare"]="$idx_name"
    done < <(grep -oP '`docs/[^`]+\.(md|yaml)`' "$idx" | sed 's/`//g' | sort -u)
done

echo "Unique paths referenced from indexes: ${#REFERENCED[@]}"
echo ""

# ── Step 2: Find all documents in docs/ ─────────────────────────────────
# Exclude: archive/, ci/, generated/, handoffs/, .agents/

UNREFERENCED=()
REFERENCED_COUNT=0

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

    # Check if referenced
    if [[ -n "${REFERENCED[$rel]:-}" ]]; then
        REFERENCED_COUNT=$((REFERENCED_COUNT + 1))
        if [[ "$VERBOSE" == true ]]; then
            echo -e "${GREEN}[REFERENCED]${NC} $rel (from ${REFERENCED[$rel]})"
        fi
    else
        UNREFERENCED+=("$rel")
    fi

done < <(find "$DOCS_DIR" -type f \( -name "*.md" -o -name "*.yaml" \) -not -path "*/archive/*" -not -path "*/ci/*" -not -path "*/generated/*" -not -path "*/handoffs/*" -print0)

# ── Step 3: Report ──────────────────────────────────────────────────────

if [[ ${#UNREFERENCED[@]} -gt 0 ]]; then
    echo -e "${YELLOW}── Unreferenced Documents (candidates for archival) ──${NC}"
    for doc in "${UNREFERENCED[@]}"; do
        echo -e "${RED}[UNREFERENCED]${NC} $doc"
    done
    echo ""
fi

echo "=== Results ==="
echo "Total documents scanned: $((REFERENCED_COUNT + ${#UNREFERENCED[@]}))"
echo "Referenced: $REFERENCED_COUNT"
echo "Unreferenced: ${#UNREFERENCED[@]}"

if [[ ${#UNREFERENCED[@]} -eq 0 ]]; then
    echo -e "${GREEN}PASS: All documents are referenced from at least one index.${NC}"
else
    echo -e "${RED}WARNING: ${#UNREFERENCED[@]} document(s) not referenced from any index.${NC}"
    echo "Review each for archival per DOCUMENTATION_STANDARDS.md §3."
fi
