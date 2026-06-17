#!/usr/bin/env bash
# spec/graph/query — Cross-reference extractor for markdown documents
#
# Extracts what a document claims to reference and verifies each path exists.
# No graph database. No RDF. Just: "here's what this doc claims to reference,
# and here's what actually exists at that path."
#
# Usage:
#   ./graph-query.sh <file.md>              # Single file (shows broken + summary)
#   ./graph-query.sh <file.md> -v           # Verbose: show all references
#   ./graph-query.sh <file.md> -b           # Only show broken references
#   ./graph-query.sh <directory>            # All .md files in directory (recursive)
#
# Extracts two reference types:
#   1. Markdown links:     [text](path) or [text](path#fragment)
#   2. Code paths:         `crates/hkask-cns/src/foo.rs` (backtick-wrapped paths)
#
# Resolution: tries doc-relative first, then repo-root fallback.
# Skips: http/https URLs, anchors (#section), absolute paths, glob patterns.
#
# Real issues found (2026-06-10):
#   - TODO.md references spans.rs (deleted), git_cas.rs (deleted)
#   - TODO.md references DOCUMENTATION_REFRESH_DDNVSS.md (never existed)
#   - TODO.md references old archive paths (pre-date current archive)

set -euo pipefail

INPUT="${1:-}"
MODE="${2:-}"

usage() {
    echo "Usage: $(basename "$0") <file.md|directory> [-v|--verbose] [-b|--broken-only]"
    echo ""
    echo "Extracts cross-references from markdown docs and verifies they exist."
    echo ""
    echo "Options:"
    echo "  -v, --verbose     Show ALL references (default: broken + summary)"
    echo "  -b, --broken-only Show only broken references"
    exit 1
}

if [ -z "$INPUT" ]; then
    usage
fi

if [ ! -e "$INPUT" ]; then
    echo "ERROR: Path does not exist: $INPUT" >&2
    exit 1
fi

SHOW_ALL=false

case "${MODE:-}" in
    -v|--verbose) SHOW_ALL=true ;;
    "") ;;
    *) usage ;;
esac

# Determine repo root (where this script lives is docs/ci/)
REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

# Collect markdown files to process
FILES=()
if [ -f "$INPUT" ]; then
    FILES+=("$INPUT")
elif [ -d "$INPUT" ]; then
    while IFS= read -r -d '' f; do
        # Skip auto-generated status files
        [[ "$f" == *"docs/status/"* ]] && continue
        # Skip audit files (investigative/analytical, reference planned paths)
        [[ "$f" == *"docs/audit/"* ]] && continue
        # Skip CI scripts themselves
        [[ "$f" == *"docs/ci/"* ]] && continue
        # Skip handoff files (transient)
        [[ "$f" == *"docs/handoffs/"* ]] && continue
        # Skip archive files
        [[ "$f" == *"docs/archive/"* ]] && continue
        FILES+=("$f")
    done < <(find "$INPUT" -name "*.md" -print0)
else
    echo "ERROR: Not a file or directory: $INPUT" >&2
    exit 1
fi

if [ ${#FILES[@]} -eq 0 ]; then
    echo "No markdown files found in: $INPUT" >&2
    exit 1
fi

TOTAL_REFS=0
BROKEN_REFS=0

# Extract and verify references from each file
for doc in "${FILES[@]}"; do
    doc_rel="${doc#$REPO_ROOT/}"

    # ── Type 1: Markdown links ──────────────────────────────────────────
    # Match [text](path) patterns, skipping URLs (http/https)
    while IFS= read -r ref; do
        [ -z "$ref" ] && continue
        # Skip external URLs, anchors, absolute paths, and non-file patterns
        [[ "$ref" =~ ^https?:// ]] && continue
        [[ "$ref" =~ ^# ]] && continue
        [[ "$ref" =~ ^/ ]] && continue
        [[ "$ref" =~ ^~ ]] && continue
        [[ "$ref" =~ \* ]] && continue  # glob patterns
        [[ "$ref" =~ " " ]] && continue  # fragments with spaces (e.g., "MDS.md §7.1")
        [[ "$ref" == "" ]] && continue

        # Resolve relative to the doc's directory, then repo-root fallback
        doc_dir="$(dirname "$doc")"
        if [ -e "$doc_dir/$ref" ]; then
            resolved="$doc_dir/$ref"
        elif [ -e "$REPO_ROOT/$ref" ]; then
            resolved="$REPO_ROOT/$ref"
        else
            resolved="$doc_dir/$ref"
        fi

        # Remove fragment (#section)
        resolved="${resolved%%#*}"

        TOTAL_REFS=$((TOTAL_REFS + 1))

        if [ -e "$resolved" ]; then
            if $SHOW_ALL; then
                echo "doc: $doc_rel | ref: $ref | type: link | exists"
            fi
        else
            BROKEN_REFS=$((BROKEN_REFS + 1))
            echo "doc: $doc_rel | ref: $ref | type: link | MISSING"
        fi
    done < <(grep -oP '\[[^\]]*\]\(([^)]+)\)' "$doc" 2>/dev/null | \
             sed -E 's/\[[^]]*\]\(([^)]*)\)/\1/' | \
             grep -v '^https\?://' | grep -v '^#' | sort -u)

    # ── Type 2: Code paths in backticks ─────────────────────────────────
    # Match `path/to/file.rs` or `path/to/file.md` patterns (likely code/docs references)
    while IFS= read -r ref; do
        [ -z "$ref" ] && continue
        # Only match paths that look like file references (contain .rs, .md, .toml, .yaml, .json)
        [[ "$ref" =~ \.(rs|md|toml|yaml|json)$ ]] || continue
        # Skip absolute paths and non-file patterns
        [[ "$ref" =~ ^/ ]] && continue
        [[ "$ref" =~ ^~ ]] && continue
        [[ "$ref" =~ \* ]] && continue
        [[ "$ref" =~ " " ]] && continue

        doc_dir="$(dirname "$doc")"
        # Try doc-relative first, then repo-root fallback
        if [ -e "$doc_dir/$ref" ]; then
            resolved="$doc_dir/$ref"
        elif [ -e "$REPO_ROOT/$ref" ]; then
            resolved="$REPO_ROOT/$ref"
        else
            resolved="$doc_dir/$ref"
        fi

        TOTAL_REFS=$((TOTAL_REFS + 1))

        if [ -e "$resolved" ]; then
            if $SHOW_ALL; then
                echo "doc: $doc_rel | ref: $ref | type: code_path | exists"
            fi
        else
            BROKEN_REFS=$((BROKEN_REFS + 1))
            echo "doc: $doc_rel | ref: $ref | type: code_path | MISSING"
        fi
    done < <(grep -oP '`([^`]+\.(rs|md|toml|yaml|json))`' "$doc" 2>/dev/null | \
             sed -E 's/`([^`]+)`/\1/' | sort -u)
done

# ── Summary ──────────────────────────────────────────────────────────
echo ""
echo "=== Results ==="
echo "Files checked: ${#FILES[@]}"
echo "References extracted: $TOTAL_REFS"
if [ "$BROKEN_REFS" -eq 0 ]; then
    echo "Broken: 0"
    echo "PASS: No broken references."
else
    echo "Broken: $BROKEN_REFS"
    echo "FAIL: $BROKEN_REFS broken reference(s) found."
fi
