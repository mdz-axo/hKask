#!/usr/bin/env bash
# docs/ci/verify-docs.sh — Documentation Health Check (Cybernetic Feedback Loop)
#
# Verifies documentation-code alignment mechanically. Designed to run in CI
# as a required gate. Produces measurable staleness signals — documentation
# drift is a CNS variety-like counter, not accumulated debt.
#
# Per DOCUMENTATION_STANDARDS.md: Zero stale references. Docs follow code.
#
# Checks performed:
#   1. Ground truth from code (crates, MCP servers, skills, version)
#   2. Stale crate references in docs
#   3. Stale last_updated dates (>30 days)
#   4. Every MCP server and core crate has a README
#   5. MCP server README tool table coverage
#   6. Key factual assertions in root READMEs (counts, versions)
#   7. Intra-documentation hyperlink resolution (NEW)
#   8. last-verified-against commit distance from HEAD (NEW — staleness signal)
#   9. Crate documentation coverage scoring (NEW — zero-coverage crate detection)
#  10. Doc example compilation via cargo test --doc (NEW — correctness gate)
#
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
STALENESS_THRESHOLD=30  # commits behind HEAD before staleness warning

cd "$PROJECT_ROOT"

echo "=== hKask Documentation Health Check ==="
echo ""

# ────────────────────────────────────────────────────────────
# STEP 1: Build ground truth from code
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[1/10] Building ground truth from code...${NC}"

cargo_version=$(grep '^version' Cargo.toml | grep -oP '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
actual_crates=$(ls crates/ 2>/dev/null | sort -u)
actual_mcps=$(ls mcp-servers/ 2>/dev/null | sort -u)
actual_skills=$(ls .agents/skills/ 2>/dev/null | sort -u)
head_commit=$(git rev-parse HEAD)

crate_count=$(echo "$actual_crates" | grep -c . || echo 0)
mcp_count=$(echo "$actual_mcps" | grep -c . || echo 0)
skill_count=$(echo "$actual_skills" | grep -c . || echo 0)

echo "  Version: $cargo_version"
echo "  HEAD:    $head_commit"
echo "  Crates:  $crate_count"
echo "  MCPs:    $mcp_count"
echo "  Skills:  $skill_count"
echo ""

# ────────────────────────────────────────────────────────────
# STEP 2: Stale crate references in docs
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[2/10] Checking stale crate references in docs...${NC}"

all_actual=$(printf "%s\n%s" "$actual_crates" "$actual_mcps" | sort -u)

FORWARD_LOOKING_DIRS='^docs/(plans/|guides/|status/|OPEN_QUESTIONS\.md)'

# Non-crate identifiers that match the hkask-* pattern but are not workspace
# members (systemd service names, env var fragments, etc.). These are
# legitimate references and must not be flagged as stale.
ALLOW_NON_CRATE='hkask-daemon|hkask-daemon-user|hkask-default-passphrase-2024|hkask-db-passphrase'

while IFS=: read -r file name; do
  [ -z "$file" ] && continue
  if ! echo "$all_actual" | grep -qxF "$name"; then
    parent_name=$(echo "$name" | sed 's/-fuzz$//')
    if echo "$all_actual" | grep -qxF "$parent_name"; then
      continue
    fi
    if echo "$name" | grep -qE "^($ALLOW_NON_CRATE)$"; then
      continue
    fi
    if echo "$file" | grep -qE "$FORWARD_LOOKING_DIRS"; then
      echo -e "  ${YELLOW}PLANNED:${NC} $name → $file (forward-looking doc)"
      WARNINGS=$((WARNINGS + 1))
      continue
    fi
    echo -e "  ${RED}STALE:${NC} $name → $file (not in workspace)"
    ERRORS=$((ERRORS + 1))
  fi
done < <(grep -roPH 'hkask-[a-z][a-z0-9-]*[a-z0-9]' docs/ --include='*.md' 2>/dev/null | grep -v '^docs/archive/' | sort -u)
echo ""

# ────────────────────────────────────────────────────────────
# STEP 3: Stale last_updated dates (>30 days)
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[3/10] Checking stale frontmatter dates (>30 days)...${NC}"

thirty_days_ago=$(date -d '30 days ago' +%s 2>/dev/null || echo "0")
if [ "$thirty_days_ago" != "0" ]; then
  for f in $(find docs/ -name '*.md' -not -path '*/archive/*' -not -path '*/generated/*'); do
    doc_date=$(grep -oP 'last_updated:\s*\K[0-9]{4}-[0-9]{2}-[0-9]{2}' "$f" 2>/dev/null | head -1)
    if [ -n "$doc_date" ]; then
      doc_epoch=$(date -d "$doc_date" +%s 2>/dev/null || echo "0")
      if [ "$doc_epoch" != "0" ] && [ "$doc_epoch" -lt "$thirty_days_ago" ]; then
        echo -e "  ${YELLOW}STALE DATE:${NC} $f — last_updated $doc_date (>30 days ago)"
        WARNINGS=$((WARNINGS + 1))
      fi
    fi
  done
fi
echo ""

# ────────────────────────────────────────────────────────────
# STEP 4: Every MCP server and core crate has README
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[4/10] Checking README coverage...${NC}"

for dir in mcp-servers/hkask-mcp-*/; do
  name=$(basename "$dir")
  if [ ! -f "$dir/README.md" ]; then
    echo -e "  ${RED}MISSING README (MCP):${NC} $name"
    ERRORS=$((ERRORS + 1))
  fi
done

CORE_CRATES="hkask-types hkask-storage hkask-memory hkask-cns hkask-templates hkask-agents hkask-keystore hkask-mcp hkask-cli hkask-api hkask-capability hkask-ports hkask-inference hkask-communication hkask-improv hkask-condenser hkask-acp hkask-adapter hkask-test-harness hkask-wallet hkask-wallet-types hkask-ledger hkask-services hkask-codegraph hkask-guard hkask-database"

for crate in $CORE_CRATES; do
  if [ -d "crates/$crate" ] && [ ! -f "crates/$crate/README.md" ]; then
    echo -e "  ${RED}MISSING README (core):${NC} $crate"
    ERRORS=$((ERRORS + 1))
  fi
done

for dir in crates/hkask-services-*/; do
  name=$(basename "$dir")
  if [ ! -f "$dir/README.md" ]; then
    echo -e "  ${YELLOW}MISSING README (service):${NC} $name"
    WARNINGS=$((WARNINGS + 1))
  fi
done
echo ""

# ────────────────────────────────────────────────────────────
# STEP 5: MCP server README tool table coverage
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[5/10] Checking MCP server README tool coverage...${NC}"

for dir in mcp-servers/hkask-mcp-*/; do
  name=$(basename "$dir")
  readme_tools=$(grep -cP '^\| \`[a-z_]+\`' "$dir/README.md" 2>/dev/null || :)
  readme_tools=${readme_tools:-0}
  code_tools=$(grep -rn 'pub async fn' "$dir/src" --include='*.rs' 2>/dev/null | grep -c 'fn [a-z]' || :)
  code_tools=${code_tools:-0}
  if [ "$readme_tools" -eq 0 ] && [ "$code_tools" -gt 0 ] 2>/dev/null; then
    echo -e "  ${YELLOW}TOOL GAP:${NC} $name — README has ~$readme_tools tools, code has ~$code_tools"
    WARNINGS=$((WARNINGS + 1))
  fi
done
echo ""

# ────────────────────────────────────────────────────────────
# STEP 6: Key factual assertions in root READMEs
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[6/10] Verifying key assertions in README.md and AGENTS.md...${NC}"

readme_mcp_count=$(grep -oP '\*\*Essential Tools\*\* \| \K[0-9]+(?= MCP servers)' README.md 2>/dev/null || echo "?")
if [ "$readme_mcp_count" != "?" ] && [ "$readme_mcp_count" != "$mcp_count" ]; then
  echo -e "  ${RED}MCP MISMATCH:${NC} README.md claims $readme_mcp_count, actual: $mcp_count"
  ERRORS=$((ERRORS + 1))
fi

readme_skill_count=$(grep -oP '\*\*Composition\*\* \| .*? \K[0-9]+(?= composable skills)' README.md 2>/dev/null || echo "?")
agents_skill_claim=$(grep -oP '\*\*\K[0-9]+(?= total\*\*)' AGENTS.md 2>/dev/null || echo "?")

if [ "$agents_skill_claim" != "?" ] && [ "$agents_skill_claim" != "$skill_count" ]; then
  echo -e "  ${RED}SKILL MISMATCH:${NC} AGENTS.md claims $agents_skill_claim skills, disk: $skill_count"
  ERRORS=$((ERRORS + 1))
fi

readme_version_claim=$(grep -oP '\*\*Version:\*\* v\K[0-9]+\.[0-9]+\.[0-9]+' README.md 2>/dev/null || echo "?")
if [ "$readme_version_claim" != "?" ] && [ "$readme_version_claim" != "$cargo_version" ]; then
  echo -e "  ${RED}VERSION MISMATCH:${NC} README.md says v$readme_version_claim, Cargo.toml: v$cargo_version"
  ERRORS=$((ERRORS + 1))
fi
echo ""

# ────────────────────────────────────────────────────────────
# STEP 7: Intra-documentation hyperlink resolution (NEW)
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[7/10] Checking intra-documentation hyperlinks...${NC}"

BROKEN_LINKS=0
# Extract each target with its source file so relative links are resolved from
# the document that contains them.
link_pattern='\]\(([^)]*)\)'
while IFS= read -r match_line; do
  source_file=${match_line%%:*}
  remaining=${match_line#*:}
  while [[ "$remaining" =~ $link_pattern ]]; do
    link_part=${BASH_REMATCH[1]}
    remaining=${remaining#*"${BASH_REMATCH[0]}"}
    if echo "$link_part" | grep -qE '^https?://|^#'; then continue; fi
    clean_target=$(echo "$link_part" | sed -E 's/#.*//; s/:[0-9]+(-[0-9]+)?$//')
    source_dir=$(dirname "$source_file")
    resolved="$source_dir/$clean_target"
    if [ ! -f "$resolved" ] && [ ! -d "$resolved" ] && [ ! -f "$PROJECT_ROOT/$clean_target" ] && [ ! -d "$PROJECT_ROOT/$clean_target" ]; then
      echo -e "  ${RED}BROKEN:${NC} $source_file → $link_part"
      BROKEN_LINKS=$((BROKEN_LINKS + 1))
    fi
  done
done < <(grep -rn '\](' docs/ --include='*.md' 2>/dev/null | grep -v '^docs/archive/')

if [ "$BROKEN_LINKS" -gt 0 ]; then
  ERRORS=$((ERRORS + BROKEN_LINKS))
fi
echo ""

# ────────────────────────────────────────────────────────────
# STEP 8: last-verified-against staleness check (NEW)
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[8/10] Checking last-verified-against staleness...${NC}"

STALE_COUNT=0
while IFS=: read -r file commit_hash; do
  [ -z "$file" ] && continue
  [ -z "$commit_hash" ] && continue
  if git rev-parse --quiet --verify "$commit_hash" > /dev/null 2>&1; then
    distance=$(git rev-list --count "$commit_hash..HEAD" 2>/dev/null || echo "0")
    if [ "$distance" -gt "$STALENESS_THRESHOLD" ] 2>/dev/null; then
      echo -e "  ${YELLOW}STALE:${NC} $file — last-verified $commit_hash is $distance commits behind HEAD"
      STALE_COUNT=$((STALE_COUNT + 1))
    fi
  else
    echo -e "  ${YELLOW}UNKNOWN HASH:${NC} $file — last-verified-against: $commit_hash (not in repo)"
    WARNINGS=$((WARNINGS + 1))
  fi
done < <(grep -rn 'last-verified-against:' docs/ --include='*.md' 2>/dev/null | grep -v 'docs/archive/' | sed 's/.*last-verified-against: *"//' | sed 's/".*//')

if [ "$STALE_COUNT" -gt 0 ]; then
  echo -e "  ${YELLOW}$STALE_COUNT document(s) have stale last-verified-against hashes${NC}"
  WARNINGS=$((WARNINGS + STALE_COUNT))
else
  echo "  All last-verified-against hashes are current."
fi
echo ""

# ────────────────────────────────────────────────────────────
# STEP 9: Crate documentation coverage scoring (NEW)
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[9/10] Checking crate documentation coverage...${NC}"

ZERO_DOC_CRATES=0
for crate_dir in crates/*/; do
  crate_name=$(basename "$crate_dir")
  # Count outer (`///`) and inner crate/module (`//!`) doc comment lines in src/.
  doc_lines=$(grep -rEc '^[[:space:]]*//[/!]' "$crate_dir/src/" --include='*.rs' 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
  # Count total Rust lines
  total_lines=$(grep -rc '.' "$crate_dir/src/" --include='*.rs' 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
  if [ "$total_lines" -gt 0 ] && [ "$doc_lines" -eq 0 ] 2>/dev/null; then
    echo -e "  ${RED}ZERO DOCS:${NC} $crate_name — $total_lines lines, 0 doc comment lines"
    ZERO_DOC_CRATES=$((ZERO_DOC_CRATES + 1))
  fi
done

if [ "$ZERO_DOC_CRATES" -gt 0 ]; then
  echo -e "  ${YELLOW}$ZERO_DOC_CRATES crate(s) have zero documentation comments${NC}"
  WARNINGS=$((WARNINGS + ZERO_DOC_CRATES))
else
  echo "  All crates have some documentation coverage."
fi
echo ""

# ────────────────────────────────────────────────────────────
# STEP 10: Doc example compilation (NEW)
# ────────────────────────────────────────────────────────────
echo -e "${CYAN}[10/10] Verifying doc examples compile (cargo test --doc)...${NC}"

if cargo test --doc --workspace 2>&1 | tail -5; then
  echo -e "  ${GREEN}PASS:${NC} All doc examples compile."
else
  echo -e "  ${RED}FAIL:${NC} Some doc examples do not compile. Run 'cargo test --doc' for details."
  ERRORS=$((ERRORS + 1))
fi
echo ""

# ────────────────────────────────────────────────────────────
# RESULTS
# ────────────────────────────────────────────────────────────
echo "=== Documentation Health Check Results ==="
echo "Errors:   $ERRORS"
echo "Warnings: $WARNINGS"
echo ""

if [ $ERRORS -gt 0 ]; then
  echo -e "${RED}FAIL: $ERRORS error(s) found. Documentation is out of sync with code.${NC}"
  echo ""
  echo "Fix actions:"
  echo "  1. Run 'docs/ci/verify-docs.sh' after code changes to preview failures"
  echo "  2. Update stale references, broken links, versions, and counts"
  echo "  3. Create missing READMEs in affected crates/MCP servers"
  echo "  4. Fix doc examples that fail to compile"
  echo "  5. Re-run verify-docs.sh until zero errors"
  exit 1
else
  echo -e "${GREEN}PASS: Documentation is in sync with code.${NC}"
  if [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}$WARNINGS warning(s) — review before merge.${NC}"
    echo ""
    echo "Warnings are advisory — they do not block CI."
    echo "Staleness signals accumulate warnings when docs drift >$STALENESS_THRESHOLD commits from HEAD."
  fi
  exit 0
fi
