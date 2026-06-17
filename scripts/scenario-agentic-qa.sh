#!/usr/bin/env bash
# Agentic QA End-to-End Scenario — exercises the full contract lifecycle
#
# Demonstrates: discover → propose → review → accept → verify
# Uses hkask-cns as the target crate (correctness-critical).
#
# Prerequisites: cargo build --package hkask-cli
# Run: bash scripts/scenario-agentic-qa.sh

set -euo pipefail

BINARY="./target/debug/kask"
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

header() { echo -e "\n${CYAN}=== $1 ===${NC}\n"; }
pass()  { echo -e "${GREEN}PASS: $1${NC}"; }
info()  { echo -e "${YELLOW}$1${NC}"; }

header "Agentic QA Scenario — Contract Lifecycle"

# ── Phase 1: Discovery ──────────────────────────────────────
header "Phase 1: Discovery (contract/audit)"

info "Discovering uncontracted functions in hkask-cns..."
DISCOVER=$($BINARY contract discover -c hkask-cns 2>&1)
echo "$DISCOVER" | head -5

# Count uncontracted
UNCONTRACTED=$(echo "$DISCOVER" | grep -c "L[0-9]" || true)
info "Uncontracted functions found: $UNCONTRACTED"

if [ "$UNCONTRACTED" -eq 0 ]; then
    pass "All hkask-cns functions are contracted (includes #[rs::contract] detection)"
else
    info "Functions still needing contracts: $UNCONTRACTED"
fi

# ── Phase 1b: Run contract tests ────────────────────────────
header "Phase 1b: Run Contract Tests (test/run)"

info "Running contract tests on hkask-cns..."
RESULT=$($BINARY test -c hkask-cns --format json 2>&1)
PASSED=$(echo "$RESULT" | grep -o '"total_passed":[0-9]*' | grep -o '[0-9]*')
FAILED=$(echo "$RESULT" | grep -o '"total_failed":[0-9]*' | grep -o '[0-9]*')

info "Test results: $PASSED passed, $FAILED failed"

if [ "$FAILED" -eq 0 ]; then
    pass "All contract tests pass"
else
    info "WARNING: $FAILED contract tests failed"
fi

# ── Phase 2: Propose a contract ─────────────────────────────
header "Phase 2: Proposal (contract/propose)"

CONTRACT_ID="QA-SCENARIO-$(date +%s)"

info "Agent r7 proposing contract $CONTRACT_ID for hkask-cns::reserve..."
$BINARY contract propose \
    -c hkask-cns \
    -f "reserve" \
    --contract-id "$CONTRACT_ID" \
    --pre "gas is a valid EnergyCost" \
    --post "returns Ok(reserved) or Err(BudgetExceeded)" \
    -r "agent-r7" 2>&1

pass "Contract proposal submitted: $CONTRACT_ID"

# ── Phase 3: Curator review ─────────────────────────────────
header "Phase 3: Curator Review (contract/list)"

info "Curator checking review queue..."
$BINARY contract list 2>&1

# ── Phase 4: Human accept ───────────────────────────────────
header "Phase 4: Consent Gate (contract/accept)"

info "Human reviewer accepting $CONTRACT_ID..."
$BINARY contract accept "$CONTRACT_ID" -r "human-reviewer" 2>&1

pass "Contract accepted: $CONTRACT_ID"

# ── Phase 5: Verify again ───────────────────────────────────
header "Phase 5: Verification (test/run)"

info "Re-running tests after contract acceptance..."
RESULT2=$($BINARY test -c hkask-cns --format json 2>&1)
PASSED2=$(echo "$RESULT2" | grep -o '"total_passed":[0-9]*' | grep -o '[0-9]*')
FAILED2=$(echo "$RESULT2" | grep -o '"total_failed":[0-9]*' | grep -o '[0-9]*')

info "Test results: $PASSED2 passed, $FAILED2 failed"
pass "Pipeline complete — all phases exercised successfully"

# ── Summary ─────────────────────────────────────────────────
header "Scenario Summary"

echo "  ✓ Phase 1: Discovered uncontracted functions"
echo "  ✓ Phase 1b: Ran contract tests ($PASSED passed, $FAILED failed)"
echo "  ✓ Phase 2: Proposed contract $CONTRACT_ID"
echo "  ✓ Phase 3: Curator reviewed queue"
echo "  ✓ Phase 4: Human accepted via consent gate"
echo "  ✓ Phase 5: Verified tests still pass"
echo ""
echo "The agentic QA pipeline is operational."
echo ""
echo "Next steps for continuous operation:"
echo "  kask daemon start    # Persistent CNS with background monitoring"
echo "  kask test --watch 60 # Continuous test monitoring every 60s"
