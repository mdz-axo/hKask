---
title: "Contract-First Migration & Replicant Contract Proposal — Strategic Plan"
audience: [engineers, architects, agents]
last_updated: 2026-06-15
version: "0.27.0"
status: "Draft"
domain: "Cross-cutting"
mds_categories: [lifecycle, curation]
---

# Contract-First Migration & Replicant Contract Proposal — Strategic Plan

**Source:** Testing Discipline audit (2026-06-15) — Grill-Me gaps #8 and #9  
**Governing principles:** P4 (Clear Boundaries), P6 (Space for Replicants), P7 (Evolutionary Architecture), P8 (Semantic Grounding), P9 (Homeostatic Self-Regulation)  
**Anchoring document:** `docs/architecture/core/TESTING_DISCIPLINE.md`

---

## 1) Objective

Two strategic capabilities that make the Testing Discipline operational at scale:

**Capability A — Contract-First Migration:** Move hKask from ~0% contracted public functions to 100%, with a phased migration plan that respects P7 (evolutionary architecture — no big-bang rewrites) and P5 (essentialism — only add contracts where they encode behavior).

**Capability B — Replicant-Driven Contract Proposals:** Build the tooling and consent pathway for agents (replicants) to propose behavioral contracts for their own functions, per §7.4 of the Testing Discipline. This operationalizes P6 (Space for Replicants) in the testing domain.

---

## 2) Current State (IS — measured)

### Contract Coverage

```bash
# Run: grep -rn "pub fn\|pub async fn" crates/ mcp-servers/ --include="*.rs" | grep -v "cfg(test)" | grep -v "/tests/" | wc -l
# Run: grep -rn "// REQ:.*pre:" crates/ mcp-servers/ --include="*.rs" | wc -l
```

**Estimated:** ~900 public functions, ~0 with `// REQ: pre:` contracts. The codebase has `// REQ:` tags on tests (from the TDD skill) but not on function signatures.

### Replicant Contract Infrastructure

**Current state:** Zero. No tooling exists for agents to:
- Discover uncontracted public functions
- Propose contracts for them
- Open PRs with contract + test
- Receive human consent (P2) for merge

---

## 3) Target State (OUGHT)

### Capability A — Contract-First Migration

| Phase | Scope | Target | Timeline |
|-------|-------|--------|----------|
| **A1 — Seed** | 3 highest-risk crates | ≥50% of `pub fn` contracted | 2 weeks |
| **A2 — Expand** | All core crates (13) | ≥80% of `pub fn` contracted | 6 weeks |
| **A3 — Complete** | All crates + MCP servers | 100% of `pub fn` contracted | 12 weeks |
| **A4 — Sustain** | All new code | 100% — CI gate enforces | Ongoing |

### Capability B — Replicant Contract Proposals

| Phase | Scope | Target |
|-------|-------|--------|
| **B1 — Discovery** | Agent can list uncontracted `pub fn` in a crate | CLI command or MCP tool |
| **B2 — Proposal** | Agent can propose a contract for a function | Generates `// REQ: pre/post` + proptest |
| **B3 — PR Flow** | Agent opens PR with contract + test; human reviews | Standard GitHub PR with P2 consent gate |
| **B4 — CNS Integration** | Contract proposal/acceptance/rejection emits CNS spans | `cns.contract.proposed`, `cns.contract.accepted`, `cns.contract.rejected` |

---

## 4) Capability A — Contract-First Migration Plan

### 4.1 Migration Principles

1. **Evolutionary, not big-bang (P7).** Contracts are added incrementally. No crate is "blocked" until fully contracted. Each PR adds contracts to the functions it touches.

2. **Risk-prioritized (P9).** Highest-risk crates first: CNS, wallet, keystore, MCP. Lowest-risk last: CLI formatting, markdown generation.

3. **Bug-driven (P7).** When a bug is found in an uncontracted function, the fix MUST include adding a contract. This is the primary migration engine.

4. **Replicant-assisted (P6).** Agents propose contracts for functions they interact with. Humans review and consent. This parallelizes the migration.

5. **No speculative contracts (P5).** Don't add contracts to functions that are never called or are pass-throughs. The deletion test applies to contracts too — if deleting the contract doesn't change any test's ability to catch bugs, the contract was ceremonial.

### 4.2 Risk-Prioritized Crate Order

| Priority | Crate | Reason | `pub fn` est. | Target Phase |
|----------|-------|--------|---------------|-------------|
| **P0** | `hkask-cns` | Cybernetic control loop — correctness-critical | ~50 | A1 |
| **P0** | `hkask-wallet` | Financial transactions — data-loss risk | ~40 | A1 |
| **P0** | `hkask-keystore` | Encryption/key derivation — security-critical | ~30 | A1 |
| **P1** | `hkask-mcp` | MCP daemon — all tool dispatch flows through here | ~60 | A2 |
| **P1** | `hkask-condenser` | Context compression — algorithmic correctness | ~20 | A2 |
| **P1** | `hkask-inference` | Inference routing — fallback behavior | ~25 | A2 |
| **P1** | `hkask-storage` | Persistence layer — data integrity | ~80 | A2 |
| **P2** | `hkask-services` | Business logic — service orchestration | ~100 | A2 |
| **P2** | `hkask-agents` | Agent orchestration — multi-agent coordination | ~40 | A2 |
| **P2** | `hkask-templates` | Template validation — manifest correctness | ~30 | A2 |
| **P3** | `hkask-types` | Foundation types — consumed by all crates | ~120 | A3 |
| **P3** | `hkask-memory` | Salience/semantic memory | ~25 | A3 |
| **P3** | `hkask-improv` | Interaction protocols | ~20 | A3 |
| **P4** | `hkask-cli` | CLI formatting — low correctness risk | ~80 | A3 |
| **P4** | `hkask-api` | API routes — mostly pass-through to services | ~50 | A3 |
| **P4** | `hkask-communication` | Matrix transport | ~15 | A3 |
| **P5** | MCP servers (10) | Tool handlers — varied risk by server | ~200 | A3 |

### 4.3 Migration Mechanics

**For each function, the migration step is:**

1. Identify the function's behavior from existing tests, docs, and usage
2. Write the contract as a `// REQ:` doc-comment:
   ```
   /// REQ: <spec_id>
   /// pre:  <caller obligations>
   /// post: <function guarantees>
   /// inv:  <cross-operation invariants, if any>
   ```
3. If no spec exists for this function's behavior, create a minimal one via `spec/goal/capture`
4. Verify the contract with a property-based test (or note why PBT is inappropriate — e.g., I/O-bound)
5. If existing tests already verify the contract, add the `// REQ:` tag to them
6. If the contract reveals a bug (existing behavior violates the stated postcondition), fix the implementation

**Contract debt tracking:**

Functions without contracts are **contract debt**. Tracked via:
```bash
grep -rn "pub fn\|pub async fn" crates/ --include="*.rs" | grep -v "cfg(test)" | grep -v "/tests/" | grep -v "// REQ:.*pre:"
```

This command lists uncontracted public functions. The count should decrease over time.

### 4.4 Phase A1 — Seed (Weeks 1–2)

**Target crates:** `hkask-cns`, `hkask-wallet`, `hkask-keystore`  
**Target coverage:** ≥50% of `pub fn` contracted  
**Strategy:** Focus on the highest-risk functions first — governed tool dispatch, wallet transaction recording, key derivation.

**PR slices:**

- **PR A1.1:** Contract `hkask-cns` governed tool and energy budget functions (~25 functions)
- **PR A1.2:** Contract `hkask-wallet` transaction and balance functions (~20 functions)
- **PR A1.3:** Contract `hkask-keystore` key derivation and encryption functions (~15 functions)

**Verification:**
```bash
for crate in hkask-cns hkask-wallet hkask-keystore; do
  pub_fns=$(grep -rn "pub fn\|pub async fn" crates/$crate/src/ --include="*.rs" | grep -v "cfg(test)" | wc -l)
  contracted=$(grep -rn "// REQ:.*pre:" crates/$crate/src/ --include="*.rs" | wc -l)
  echo "$crate: $contracted / $pub_fns contracted"
done
```

### 4.5 Phase A2 — Expand (Weeks 3–8)

**Target crates:** All 13 core crates  
**Target coverage:** ≥80%  
**Strategy:** One crate per week. Replicants assist on lower-priority crates.

### 4.6 Phase A3 — Complete (Weeks 9–12)

**Target:** 100% of `pub fn` across all crates + MCP servers  
**Strategy:** Mop-up phase. Remaining uncontracted functions are either low-risk (CLI formatting) or complex (MCP server tool handlers). Replicants handle the low-risk ones; humans handle the complex ones.

### 4.7 Phase A4 — Sustain (Ongoing)

**CI gate:** New `pub fn` without `// REQ: pre:` fails CI.  
**Contract debt trend:** Measured weekly; must decrease or stay at zero.

---

## 5) Capability B — Replicant-Driven Contract Proposals

### 5.1 Discovery Tool (Phase B1) ✅ COMPLETE (2026-06-15)

**Goal:** An agent can ask "which public functions in crate X lack contracts?" and receive a list.

**Delivered:** `scripts/contract-audit.sh` — a shell script with 4 output modes:
- `--summary`: per-crate table with coverage percentages
- `--json`: machine-readable JSON for MCP tool wrapping
- `--csv`: spreadsheet-importable CSV
- `<crate-name>`: detailed listing of uncontracted functions with file:line

**Implementation path chosen:** Shell script (simplest path per handoff). Can be wrapped as an MCP tool later. The script is CI-ready (exit 0 always, trend monitor not hard gate).

**Usage:**
```bash
scripts/contract-audit.sh              # all crates, color-coded
scripts/contract-audit.sh hkask-cns    # single crate, lists uncontracted fn
scripts/contract-audit.sh --json       # JSON for MCP wrapping
scripts/contract-audit.sh --summary   # table format
```

**Original implementation options (for reference):**
- **CLI command:** `kask contract audit --crate hkask-cns` — lists uncontracted `pub fn`
- **MCP tool:** `contract/audit` — same, accessible to agents via MCP
- **CNS span:** `cns.contract.coverage` — emits current coverage ratio per crate

**Next step (Phase B2):** Agent contract generation workflow using the audit script as discovery input.

### 5.2 Proposal Generation (Phase B2)

**Goal:** An agent analyzes a function's behavior (from existing tests, docs, usage patterns) and proposes a contract.

**Agent workflow:**
1. Agent calls `contract/audit` to find uncontracted functions
2. Agent selects a function it understands (one it calls or implements)
3. Agent reads the function's implementation, existing tests, and call sites
4. Agent generates a contract proposal:
   ```
   /// REQ: <spec_id>
   /// pre:  <inferred from implementation checks and test assumptions>
   /// post: <inferred from test assertions and return type>
   ```
5. Agent generates a property-based test verifying the contract
6. Agent opens a PR with the contract + test

**Quality gates on proposals:**
- The proposed contract must not be vacuously true (`pre: true, post: true`)
- The proposed test must actually exercise the function (not just `assert!(true)`)
- The proposal must include the agent's WebID as author (P12)

### 5.3 PR Flow with Consent (Phase B3)

**Goal:** Replicant opens a PR; human reviews and provides affirmative consent (P2).

**PR template for contract proposals:**
```
## Contract Proposal

**Agent:** <replicant WebID>
**Function:** <path to function>
**Spec:** <spec_id from spec/goal/capture>

### Contract
/// REQ: <spec_id>
/// pre: ...
/// post: ...

### Property-Based Test
<proptest code>

### Consent Required
- [ ] Human review: does this contract accurately describe the function's behavior?
- [ ] Human review: does the proptest verify the contract for all valid inputs?
- [ ] Human consent: approve merge (P2)
```

**Consent flow:**
1. Replicant opens PR with contract proposal
2. CI runs: proptest must pass, prohibition sweep must be clean
3. Human reviews: is the contract correct? Is the test adequate?
4. Human provides consent by approving the PR
5. Merge emits `cns.contract.accepted` CNS span

**Rejection flow:**
1. If human rejects, PR is closed with rationale comment
2. Rejection emits `cns.contract.rejected` CNS span
3. Rejected contract is archived as a curation decision

### 5.4 CNS Integration (Phase B4) 🟡 PARTIAL (2026-06-15)

**New CNS spans (registered in PRINCIPLES.md §1.4 and CANONICAL_NAMESPACES):**

| Span | Emitted When | Observer | Status |
|------|-------------|----------|--------|
| `cns.contract.proposed` | Replicant opens a contract proposal PR | Curator | ⬜ Not yet implemented |
| `cns.contract.accepted` | Human approves and merges a contract proposal | Curator | ⬜ Not yet implemented |
| `cns.contract.rejected` | Human rejects a contract proposal | Curator | ⬜ Not yet implemented |
| `cns.contract.violated` | A contracted function's proptest fails | CNS algedonic | ✅ Emission function implemented (`emit_contract_violated`) |
| `cns.contract.coverage` | Periodic coverage measurement | CNS variety | ✅ Emission function implemented (`emit_contract_coverage`) |

**Delivered:** `crates/hkask-cns/src/contract_discipline.rs` — public module with:
- `emit_contract_violated(sink, function_name, contract_id, failure_reason)` — emits `cns.contract.violated`
- `emit_contract_coverage(sink, total_pub_fns, contracted_fns, coverage_pct)` — emits `cns.contract.coverage`
- 2 self-tests with `CaptureSink` verifying event persistence

**Wiring points:**
- `emit_contract_violated` → called by CI when proptest with `// REQ:` tag fails
- `emit_contract_coverage` → called by Cybernetics Loop regulation cycle or `scripts/contract-audit.sh` CI job

### 5.5 Phase B1–B4 Timeline

| Phase | Deliverable | Est. |
|-------|------------|------|
| B1 — Discovery | `contract/audit` MCP tool + `cns.contract.coverage` span | 1 week |
| B2 — Proposal | Agent contract generation workflow (skill or template) | 2 weeks |
| B3 — PR Flow | PR template + consent gate + CI integration | 1 week |
| B4 — CNS | Remaining CNS spans + algedonic wiring | 1 week |

---

## 6) Dependency Graph

```
Capability A (Contract-First Migration)
  ├── A1 Seed (weeks 1–2)
  │     └── Depends on: contract audit tool (B1)
  ├── A2 Expand (weeks 3–8)
  │     └── Depends on: replicant proposal flow (B2–B3) for parallelization
  ├── A3 Complete (weeks 9–12)
  │     └── Depends on: CI gate (A4 prep)
  └── A4 Sustain (ongoing)
        └── Depends on: CI gate + CNS monitoring

Capability B (Replicant Contract Proposals)
  ├── B1 Discovery (week 1)
  ├── B2 Proposal (weeks 2–3)
  │     └── Depends on: B1 (discovery tool)
  ├── B3 PR Flow (week 4)
  │     └── Depends on: B2 (proposal generation)
  └── B4 CNS (week 5)
        └── Depends on: B3 (PR flow for event emission)
```

**Parallelization:** A1 and B1 can proceed in parallel. B2–B3 enable A2 to be replicant-assisted. A3 and B4 are independent.

---

## 7) Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Contracts are vacuously true (`pre: true, post: true`) | High | High — false confidence | Quality gate: vacuously true contracts fail CI |
| Replicants propose incorrect contracts | Medium | High — wrong specification | Human consent gate (P2); contract must survive proptest |
| Migration stalls on complex functions | Medium | Medium — incomplete coverage | Bug-driven migration ensures high-risk functions get contracted first |
| Contract debt tracking becomes noise | Low | Low — alert fatigue | CNS algedonic threshold prevents alert storms |
| CI gate on new `pub fn` slows development | Medium | Medium — developer friction | Gate is warning-only for first 4 weeks; hard gate after |

---

## 8) Verification Gates

### Capability A

| Phase | Gate |
|-------|------|
| A1 | ≥50% of `pub fn` contracted in CNS, wallet, keystore |
| A2 | ≥80% of `pub fn` contracted across all 13 core crates |
| A3 | 100% of `pub fn` contracted across all crates + MCP servers |
| A4 | CI gate: new `pub fn` without `// REQ: pre:` fails build |

### Capability B

| Phase | Gate |
|-------|------|
| B1 | `contract/audit` MCP tool returns accurate uncontracted function list |
| B2 | Agent successfully proposes a contract that passes human review |
| B3 | PR template used; consent gate functional; CNS spans emitted |
| B4 | All 5 contract CNS spans registered and emitting |

---

## 9) Open Questions

1. **Contract granularity:** Should every `pub fn` have a contract, or only those that encode non-trivial behavior? (Meyer says every exported routine. Pragmatic answer: start with non-trivial, expand to all.)

2. **Probabilistic contract tooling:** The `prob:` field (§7.6 of Testing Discipline) requires statistical verification — running the function N times and measuring compliance. What tooling supports this?

3. **Contract versioning:** When a contract is strengthened (post-bug-fix), how do we track the version history? Is the contract versioned with the spec?

4. **Replicant incentive:** Why would a replicant propose a contract? The current model assumes agents are cooperative. What if they're not?

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
