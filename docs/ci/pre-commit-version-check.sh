#!/usr/bin/env bash
# docs/ci/pre-commit-version-check.sh — Pre-commit hook: flag version anomalies
# Usage: bash docs/ci/pre-commit-version-check.sh
#        or symlink as .git/hooks/pre-commit
#
# Flags staged .md/.yaml files whose version: field diverges from
# workspace Cargo.toml. Uses the same exclusion list as sync-versions.sh.
# Non-blocking — warns but does not prevent commit.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# ── Detect workspace version ────────────────────────────────────────────
WORKSPACE_VERSION=$(grep -oP '^version\s*=\s*"\K[^"]+' "$PROJECT_ROOT/Cargo.toml" | head -1)
if [[ -z "$WORKSPACE_VERSION" ]]; then
    echo "ERROR: Could not detect workspace version from Cargo.toml"
    exit 0  # Non-blocking — don't prevent commit on tool failure
fi

# ── Exclusion list (same as sync-versions.sh) ───────────────────────────
EXCLUSIONS=(
    "docs/specifications/specs/MDS_SCAFFOLD.md"
    "docs/specifications/specs/TRACEABILITY_MATRIX.md"
    "docs/architecture/reference/template-header-standard.md"
    "docs/architecture/reference/hKask-Curator-persona.md"
    "docs/architecture/reference/utoipa-implementation.md"
    "docs/architecture/reference/okapi-integration.md"
    "docs/plans/TODO.md"
)

# ── Get staged files ────────────────────────────────────────────────────
STAGED=$(git diff --cached --name-only --diff-filter=ACM 2>/dev/null || echo "")

if [[ -z "$STAGED" ]]; then
    exit 0  # Nothing staged
fi

VIOLATIONS=0

while IFS= read -r file; do
    # Only .md and .yaml files
    if [[ "$file" != *.md ]] && [[ "$file" != *.yaml ]]; then continue; fi
    # Skip non-docs files
    if [[ "$file" != docs/* ]]; then continue; fi
    # Skip archive
    if [[ "$file" == docs/archive/* ]]; then continue; fi
    # Skip CI scripts
    if [[ "$file" == docs/ci/* ]]; then continue; fi
    # Skip handoffs (no frontmatter required)
    if [[ "$file" == docs/handoffs/* ]]; then continue; fi

    # Check exclusion list
    excluded=false
    for ex in "${EXCLUSIONS[@]}"; do
        if [[ "$file" == "$ex" ]]; then
            excluded=true
            break
        fi
    done
    if [[ "$excluded" == true ]]; then continue; fi

    # Check if file has version field
    if ! git show ":0:$file" 2>/dev/null | grep -q "^version:"; then
        continue  # No version field — skip
    fi

    current_version=$(git show ":0:$file" 2>/dev/null | grep "^version:" | head -1 | sed 's/^version: *//' | tr -d '"' | xargs)

    if [[ "$current_version" != "$WORKSPACE_VERSION" ]]; then
        echo -e "${YELLOW}[VERSION ANOMALY]${NC} $file: $current_version (workspace: $WORKSPACE_VERSION)"
        VIOLATIONS=$((VIOLATIONS + 1))
    fi

done <<< "$STAGED"

if [[ $VIOLATIONS -gt 0 ]]; then
    echo ""
    echo -e "${YELLOW}⚠  $VIOLATIONS file(s) have version fields diverging from workspace $WORKSPACE_VERSION${NC}"
    echo "   Run 'bash docs/ci/sync-versions.sh' to auto-fix, or add to exclusion list if intentional."
    echo "   This hook is non-blocking — commit will proceed."
fi

exit 0  # Always non-blocking
