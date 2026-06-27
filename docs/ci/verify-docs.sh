#!/usr/bin/env bash
# docs/ci/verify-docs.sh — S3 Control Layer: Automated Document-Code Reconciliation
# Closes the cybernetic feedback loop by verifying doc assertions against code ground truth.
# Per DOCUMENTATION_STANDARDS.md: Zero stale references. Docs follow code.
# Fails the build when drift is detected — same lifecycle as compiler warnings.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PROJECT_ROOT="$(cd "$DOCS_DIR/.." && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

ERRORS=0
WARNINGS=0

cd "$PROJECT_ROOT"

echo "=== hKask Document Verification (S3 Control Layer) ==="
echo ""

# ────────────────────────────────────────────────────────────
# STEP 1: Build ground truth from code
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[1/6] Building ground truth from code...${NC}"

# Crate names
cargo_version=$(grep '^version' Cargo.toml | grep -oP '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
actual_crates=$(ls crates/ 2>/dev/null | sort -u)
actual_mcps=$(ls mcp-servers/ 2>/dev/null | sort -u)
actual_skills=$(ls .agents/skills/ 2>/dev/null | sort -u)

crate_count=$(echo "$actual_crates" | grep -c . || echo 0)
mcp_count=$(echo "$actual_mcps" | grep -c . || echo 0)
skill_count=$(echo "$actual_skills" | grep -c . || echo 0)

echo "  Cargo version: $cargo_version"
echo "  Crates: $crate_count"
echo "  MCP servers: $mcp_count"
echo "  Skills: $skill_count"
echo ""

# ────────────────────────────────────────────────────────────
# STEP 2: Stale crate references in docs
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[2/6] Checking stale crate references in docs...${NC}"

all_actual=$(printf "%s\n%s" "$actual_crates" "$actual_mcps" | sort -u)

# Docs that are forward-looking by design. References to non-existent
# crate names in these files are plans/aspirations, not errors.
# Also includes architecture docs about deployment infrastructure
# and future-state specifications (not current code architecture).
FORWARD_LOOKING_DIRS='^docs/(plans/|guides/|OPEN_QUESTIONS\.md|architecture/core/FUNCTIONAL_SPECIFICATION\.md|architecture/matrix-integration-architecture\.md|architecture/ADRs/)'

# Find all hkask-* references with their source file.
# Format: filepath:match (grep -H format). Exclude archive/.
# Use process substitution to avoid subshell (ERRORS/WARNINGS must survive).
while IFS=: read -r file name; do
  # Skip empty lines / edge cases
  [ -z "$file" ] && continue

  if ! echo "$all_actual" | grep -qxF "$name"; then
    # Fuzz targets use parent crate name (hkask-types-fuzz → hkask-types)
    parent_name=$(echo "$name" | sed 's/-fuzz$//')
    if echo "$all_actual" | grep -qxF "$parent_name"; then
      continue
    fi

    # Forward-looking docs (plans, guides, open questions) — non-blocking.
    # These documents reference planned/aspirational names by design.
    if echo "$file" | grep -qE "$FORWARD_LOOKING_DIRS"; then
      echo -e "  ${YELLOW}PLANNED:${NC} $name → $file (forward-looking doc)"
      WARNINGS=$((WARNINGS + 1))
      continue
    fi

    # Architecture docs or other active documentation — this is a real stale reference.
    echo -e "  ${RED}STALE:${NC} $name → $file (not in Cargo.toml/filesystem)"
    ERRORS=$((ERRORS + 1))
  fi
done < <(grep -roPH 'hkask-[a-z][a-z0-9-]*[a-z0-9]' docs/ --include='*.md' 2>/dev/null | grep -v '^docs/archive/' | sort -u)
echo ""

# ────────────────────────────────────────────────────────────
# STEP 3: Stale last_updated dates (>30 days)
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[3/6] Checking stale frontmatter dates (>30 days)...${NC}"

thirty_days_ago=$(date -d '30 days ago' +%s 2>/dev/null || echo "0")
if [ "$thirty_days_ago" != "0" ]; then
  for f in $(find docs/ -name '*.md' -not -path '*/archive/*' -not -path '*/generated/*'); do
    doc_date=$(grep -oP 'last_updated:\s*\K[0-9]{4}-[0-9]{2}-[0-9]{2}' "$f" 2>/dev/null | head -1)
    if [ -n "$doc_date" ]; then
      doc_epoch=$(date -d "$doc_date" +%s 2>/dev/null || echo "0")
      if [ "$doc_epoch" != "0" ] && [ "$doc_epoch" -lt "$thirty_days_ago" ]; then
        echo -e "  ${YELLOW}STALE:${NC} $f — last_updated $doc_date (>30 days ago)"
        WARNINGS=$((WARNINGS + 1))
      fi
    fi
  done
fi
echo ""

# ────────────────────────────────────────────────────────────
# STEP 4: Every MCP server and core crate has a README
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[4/6] Checking README coverage...${NC}"

# MCP servers: ALL must have READMEs
for dir in mcp-servers/hkask-mcp-*/; do
  name=$(basename "$dir")
  if [ ! -f "$dir/README.md" ]; then
    echo -e "  ${RED}MISSING README (MCP):${NC} $name"
    ERRORS=$((ERRORS + 1))
  fi
done

# Core crates: must have READMEs. Service impl crates: exempt.
CORE_CRATES="hkask-types hkask-storage hkask-memory hkask-cns hkask-templates hkask-agents hkask-keystore hkask-mcp hkask-cli hkask-api hkask-capability hkask-ports hkask-inference hkask-communication hkask-improv hkask-condenser hkask-acp hkask-adapter hkask-test-harness hkask-wallet hkask-wallet-types hkask-ledger hkask-services"

for crate in $CORE_CRATES; do
  if [ -d "crates/$crate" ] && [ ! -f "crates/$crate/README.md" ]; then
    echo -e "  ${RED}MISSING README (core):${NC} $crate"
    ERRORS=$((ERRORS + 1))
  fi
done

# Service impl crates: warn only
for dir in crates/hkask-services-*/; do
  name=$(basename "$dir")
  if [ ! -f "$dir/README.md" ]; then
    echo -e "  ${YELLOW}MISSING README (service impl):${NC} $name (may be intentional)"
    WARNINGS=$((WARNINGS + 1))
  fi
done
echo ""

# ────────────────────────────────────────────────────────────
# STEP 5: MCP server README tool coverage
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[5/6] Checking MCP server README tool table coverage...${NC}"

for dir in mcp-servers/hkask-mcp-*/; do
  name=$(basename "$dir")
  readme_tools=$(grep -cP '^\| \`[a-z_]+\`' "$dir/README.md" 2>/dev/null || echo 0)
  code_tools=$(grep -rn 'pub async fn' "$dir/src" --include='*.rs' 2>/dev/null | grep -c 'fn [a-z]' || echo 0)

  if [ "$readme_tools" -eq 0 ] && [ "$code_tools" -gt 0 ] 2>/dev/null; then
    echo -e "  ${YELLOW}TOOL GAP:${NC} $name — README tool table appears empty/missing, code has ~$code_tools tools"
    WARNINGS=$((WARNINGS + 1))
  fi
done
echo ""

# ────────────────────────────────────────────────────────────
# STEP 6: Key factual assertions in root READMEs
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[6/6] Verifying key assertions in README.md...${NC}"

# MCP server count
readme_mcp_count=$(grep -oP '\*\*Essential Tools\*\* \| \K[0-9]+(?= MCP servers)' README.md 2>/dev/null || echo "?")
if [ "$readme_mcp_count" != "?" ] && [ "$readme_mcp_count" != "$mcp_count" ]; then
  echo -e "  ${RED}MISMATCH:${NC} README.md claims $readme_mcp_count MCP servers, actual: $mcp_count"
  ERRORS=$((ERRORS + 1))
fi

# Skill count
readme_skill_count=$(grep -oP '\*\*Composition\*\* \| .*? \K[0-9]+(?= composable skills)' README.md 2>/dev/null || echo "?")
if [ "$readme_skill_count" != "?" ] && [ "$readme_skill_count" != "$skill_count" ]; then
  echo -e "  ${RED}MISMATCH:${NC} README.md claims $readme_skill_count skills, actual: $skill_count"
  ERRORS=$((ERRORS + 1))
fi

# AGENTS.md skill count
agents_skill_claim=$(grep -oP '\*\*\K[0-9]+(?= total\*\*)' AGENTS.md 2>/dev/null || echo "?")
if [ "$agents_skill_claim" != "?" ] && [ "$agents_skill_claim" != "$skill_count" ]; then
  echo -e "  ${RED}MISMATCH:${NC} AGENTS.md claims $agents_skill_claim skills, actual: $skill_count"
  ERRORS=$((ERRORS + 1))
fi

echo ""

# ────────────────────────────────────────────────────────────
# RESULTS
# ────────────────────────────────────────────────────────────
echo "=== Verification Results ==="
echo "Errors:   $ERRORS"
echo "Warnings: $WARNINGS"
echo ""

if [ $ERRORS -gt 0 ]; then
  echo -e "${RED}FAIL: $ERRORS error(s) found. Documentation is out of sync with code.${NC}"
  echo ""
  echo "Fix actions:"
  echo "  1. Run 'docs/ci/verify-docs.sh' after code changes to preview failures"
  echo "  2. Update stale references, versions, and counts in docs"
  echo "  3. Create missing READMEs in affected crates/MCP servers"
  echo "  4. Re-run verify-docs.sh until zero errors"
  exit 1
else
  echo -e "${GREEN}PASS: Documentation is in sync with code.${NC}"
  if [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}$WARNINGS warning(s) — review before merge.${NC}"
  fi
  exit 0
fi
