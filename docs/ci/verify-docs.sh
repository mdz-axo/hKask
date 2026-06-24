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
echo -e "${CYAN}[1/7] Building ground truth from code...${NC}"

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
echo -e "${CYAN}[2/7] Checking stale crate references in docs...${NC}"

all_actual=$(printf "%s\n%s" "$actual_crates" "$actual_mcps" | sort -u)

# Find all hkask-* references in docs (exclude archive/ — archived files are historical)
# Note: grep -roH (not -roh) gives filename:match lines we can filter
grep -roPH 'hkask-[a-z0-9_-]+' docs/ --include='*.md' 2>/dev/null | grep -v '^docs/archive/' | sed 's/^[^:]*://' | sort -u | while read -r name; do
  if ! echo "$all_actual" | grep -qxF "$name"; then
    # Check if it's fuzz-related (crates/*/fuzz/ subdirectories use parent crate name)
    parent_name=$(echo "$name" | sed 's/-fuzz$//')
    if echo "$all_actual" | grep -qxF "$parent_name"; then
      continue  # fuzz target: hkask-types-fuzz → hkask-types exists, OK
    fi
    # Check if it's a pod/K8s/deployment/future name (not actual crates)
    if echo "$name" | grep -qE 'hkask-(pod-|pods-|prod|prod-master|prod-worker|caddy|conduit|data-pvc|db-passphrase|hydrogen|ingress|master$|oauth-|pre-upgrade|secrets|sidecar|surface$|web$|testing$|test-utils|training$|mcp-rss-reader|mcp-web|ensemble)'; then
      # deployment concepts, future plans, deferred work — warn, don't error
      echo -e "  ${YELLOW}FUTURE/DEPLOYMENT:${NC} $name (planned/deferred, not a current crate)"
      WARNINGS=$((WARNINGS + 1))
      continue
    fi
    # Check if it's a historical/renamed crate (former naming scheme, v0.30 consolidation)
    if echo "$name" | grep -qE 'hkask-(a$|a2a$|b$|backups$|ca$|config$|curator$|gateway-tls$|gateway$|pods$|services-classify$|services-cloud$|services-inference-svc$)'; then
      echo -e "  ${YELLOW}HISTORICAL:${NC} $name (renamed in v0.30)"
      WARNINGS=$((WARNINGS + 1))
      continue
    fi
    echo -e "  ${RED}STALE:${NC} $name referenced in docs but not in Cargo.toml/filesystem"
    grep -rl "$name" docs/ --include='*.md' 2>/dev/null | head -3 | while read -r f; do
      echo "      in: $f"
    done
    ERRORS=$((ERRORS + 1))
  fi
done
echo ""

# ────────────────────────────────────────────────────────────
# STEP 3: Version consistency
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[3/7] Checking version consistency in doc frontmatter...${NC}"

for f in $(find docs/ -name '*.md' -not -path '*/archive/*' -not -path '*/generated/*'); do
  doc_version=$(grep -oP 'version:\s*"?\K[0-9]+\.[0-9]+\.[0-9]+' "$f" 2>/dev/null | head -1)
  if [ -n "$doc_version" ] && [ "$doc_version" != "$cargo_version" ]; then
    echo -e "  ${RED}VERSION MISMATCH:${NC} $f → $doc_version (should be $cargo_version)"
    ERRORS=$((ERRORS + 1))
  fi
done
echo ""

# ────────────────────────────────────────────────────────────
# STEP 4: Stale last_updated dates (>30 days)
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[4/7] Checking stale frontmatter dates (>30 days)...${NC}"

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
# STEP 5: Every MCP server and core crate has a README
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[5/7] Checking README coverage...${NC}"

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
# STEP 6: MCP server README tool coverage
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[6/7] Checking MCP server README tool table coverage...${NC}"

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
# STEP 7: Key factual assertions in root READMEs
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[7/7] Verifying key assertions in README.md...${NC}"

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

# AGENTS.md skill count (checks header "46 skills total")
agents_skill_claim=$(grep -oP '\*\*\K[0-9]+(?= skills total\*\*)' AGENTS.md 2>/dev/null || echo "?")
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
