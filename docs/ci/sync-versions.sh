#!/usr/bin/env bash
# docs/ci/sync-versions.sh — Synchronize document version fields with workspace Cargo.toml
# Usage: bash docs/ci/sync-versions.sh [--dry-run] [--new-version X.Y.Z]
#   --dry-run: Show what would change without applying
#   --new-version: Override auto-detection from Cargo.toml
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$DOCS_DIR/.." && pwd)"

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

DRY_RUN=false
NEW_VERSION=""

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run) DRY_RUN=true; shift ;;
        --new-version) NEW_VERSION="$2"; shift 2 ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# Detect workspace version from Cargo.toml if not provided
if [[ -z "$NEW_VERSION" ]]; then
    if [[ -f "$PROJECT_ROOT/Cargo.toml" ]]; then
        NEW_VERSION=$(grep -oP '^version\s*=\s*"\K[^"]+' "$PROJECT_ROOT/Cargo.toml" | head -1)
    fi
    if [[ -z "$NEW_VERSION" ]]; then
        echo "ERROR: Could not detect workspace version from Cargo.toml. Use --new-version."
        exit 1
    fi
fi

# Documents that track their own version (intentionally divergent)
# Format: "path:reason"
EXCLUSIONS=(
    "docs/specifications/specs/MDS_SCAFFOLD.md:MDS_SCAFFOLD tracks its own semantic version"
    "docs/specifications/specs/TRACEABILITY_MATRIX.md:TRACEABILITY_MATRIX tracks its own version"
    "docs/architecture/reference/template-header-standard.md:Template standard tracks its own version"
    "docs/architecture/reference/hKask-Curator-persona.md:Persona spec tracks its own version"
    "docs/architecture/reference/utoipa-implementation.md:Implementation guide tracks its own version"
    "docs/architecture/reference/okapi-integration.md:API contract tracks its own version"
    "docs/plans/TODO.md:TODO list tracks its own version"
)

echo "=== hKask Version Synchronizer ==="
echo "Target version: $NEW_VERSION"
echo "Mode: $([ "$DRY_RUN" = true ] && echo 'DRY RUN' || echo 'APPLY')"
echo ""

UPDATED=0
SKIPPED=0

# Build exclusion path list
EXCLUDE_PATHS=()
for entry in "${EXCLUSIONS[@]}"; do
    EXCLUDE_PATHS+=("${entry%%:*}")
done

while IFS= read -r -d '' file; do
    # Skip archive
    if [[ "$file" == *"/docs/archive/"* ]]; then continue; fi
    # Skip CI scripts
    if [[ "$file" == *"/docs/ci/"* ]]; then continue; fi
    # Skip non-markdown/yaml
    if [[ "$file" != *.md ]] && [[ "$file" != *.yaml ]]; then continue; fi

    rel="${file#$PROJECT_ROOT/}"

    # Check exclusion list
    excluded=false
    for ex in "${EXCLUDE_PATHS[@]}"; do
        if [[ "$rel" == "$ex" ]]; then
            excluded=true
            break
        fi
    done
    if [[ "$excluded" == true ]]; then
        echo -e "${YELLOW}[SKIP]${NC} $rel (excluded)"
        SKIPPED=$((SKIPPED + 1))
        continue
    fi

    # Check if file has YAML frontmatter with version field
    if ! grep -q "^version:" "$file"; then
        continue  # No version field — skip (handoffs, audit docs, etc.)
    fi

    current_version=$(grep "^version:" "$file" | head -1 | sed 's/^version: *//' | tr -d '"' | xargs)

    if [[ "$current_version" == "$NEW_VERSION" ]]; then
        continue  # Already correct
    fi

    if [[ "$DRY_RUN" == true ]]; then
        echo -e "${YELLOW}[WOULD UPDATE]${NC} $rel: $current_version → $NEW_VERSION"
    else
        # Update the version field
        sed -i "s/version: \"$current_version\"/version: \"$NEW_VERSION\"/" "$file"
        echo -e "${GREEN}[UPDATED]${NC} $rel: $current_version → $NEW_VERSION"
    fi
    UPDATED=$((UPDATED + 1))

done < <(find "$DOCS_DIR" -type f \( -name "*.md" -o -name "*.yaml" \) -not -path "*/archive/*" -print0)

echo ""
echo "=== Results ==="
echo "Updated: $UPDATED"
echo "Skipped (excluded): $SKIPPED"
echo "Target version: $NEW_VERSION"

if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}DRY RUN — no changes applied. Remove --dry-run to apply.${NC}"
fi
