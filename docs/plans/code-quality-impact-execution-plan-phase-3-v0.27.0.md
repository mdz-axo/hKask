# hKask v0.27.0 — Code Quality Execution Plan (Phase 3)

**Status:** Proposed  
**Depends on:**
- `docs/plans/code-quality-impact-execution-plan-v0.27.0.md` (Phase 1)
- `docs/plans/code-quality-impact-execution-plan-phase-2-v0.27.0.md` (Phase 2)  
**Created:** 2026-06-15  
**Scope:** System-level assurance, resilience, and release-governance hardening across `crates/*`, `mcp-servers/*`, CI, and docs contracts

---

## 1) Objective

Move from “quality controls present” (Phases 1–2) to **assurance at scale**:
- machine-checkable invariants,
- cross-surface conformance,
- resilience and performance budgets,
- migration/security hardening,
- release-grade quality gating.

---

## 2) Priority Stack (Phase 3, highest impact first)

1. Formalize critical invariants as machine-checkable contracts
2. Cross-surface conformance suite (CLI/API/MCP parity)
3. Scenario-level resilience testing (headless chaos-lite)
4. Performance/SLO budget enforcement in CI
5. State migration hardening with real fixtures
6. Security hardening from threat-model → regression tests
7. Documentation-as-contract automation
8. Architectural decoupling + dependency-direction enforcement
9. Deterministic/flaky test harness hardening
10. Unified release-readiness gate command/report

---

## 3) Delivery Strategy (6 waves)

## Wave 1 — Contractual assurance core

### Task 1 — Machine-checkable invariant contracts
**Assumption:** Core safety properties must be executable, not only documented.  
**Expected outcome:** invariant tests fail fast on contract violations.

**Initial contract set**
- OCAP attenuation monotonicity
- Consent boundary fail-closed behavior
- Wallet accounting conservation (`sum(ledger deltas) == current_balance`)
- Span namespace validity + category mapping stability

**PR slices**
- PR 3.1.1: Add `contracts/` test module pattern in key crates (`hkask-types`, `hkask-wallet`, `hkask-api`, `hkask-cns`).
- PR 3.1.2: Encode invariants as property/contract tests with named IDs.
- PR 3.1.3: Add CI target `contract-tests`.

**Acceptance criteria**
- Each invariant represented by executable test(s).
- Contract failure output is specific and actionable.

**Validation**
- `cargo test -p hkask-types`
- `cargo test -p hkask-wallet`
- `cargo test -p hkask-api`
- `cargo test -p hkask-cns`

---

### Task 2 — Cross-surface conformance suite
**Assumption:** Same domain behavior should be equivalent across CLI/API/MCP surfaces.  
**Expected outcome:** behavior drift is detected automatically.

**Initial parity domains**
- Settings behavior (range checks, merge semantics)
- Auth/capability error semantics
- Selected memory and spec operations where multiple surfaces exist

**PR slices**
- PR 3.2.1: Define conformance cases in one canonical test manifest.
- PR 3.2.2: Add adapters to execute same case through CLI/API/MCP paths.
- PR 3.2.3: CI parity job with diff output.

**Acceptance criteria**
- Canonical scenarios pass identically across surfaces.
- Drift produces test failure with per-surface diff.

**Validation**
- `cargo test -p hkask-cli`
- `cargo test -p hkask-api`
- relevant MCP integration tests

---

## Wave 2 — Resilience and performance guarantees

### Task 3 — Headless resilience scenarios (chaos-lite)
**Assumption:** Restart/interruption scenarios expose failures normal tests miss.  
**Expected outcome:** graceful handling of daemon/socket/keychain/transient dependency faults.

**PR slices**
- PR 3.3.1: Add scenario harness for daemon socket disconnect/reconnect.
- PR 3.3.2: Add keychain unavailable / credential resolution failure scenarios.
- PR 3.3.3: Add transient MCP upstream failure scenarios with retry policy assertions.

**Acceptance criteria**
- No uncontrolled panic/crash in defined scenarios.
- Expected degraded-mode errors are typed and consistent.

**Validation**
- targeted scenario test jobs (integration suites)

---

### Task 4 — Performance/SLO budget enforcement
**Assumption:** Regressions will reappear without explicit budget gates.  
**Expected outcome:** enforceable latency/resource ceilings in CI.

**Initial budget candidates**
- MCP server startup time ceiling
- Tool dispatch latency percentile targets for representative tools
- Memory growth bounds for long-running loop tests
- DB operation latency envelopes for hot paths

**PR slices**
- PR 3.4.1: Define benchmark harness and baseline capture process.
- PR 3.4.2: Add CI budget checks with threshold files.
- PR 3.4.3: Add fail-fast reporting for budget deltas.

**Acceptance criteria**
- CI fails on budget exceedance.
- Baselines versioned and reviewable.

**Validation**
- benchmark/perf CI jobs

---

## Wave 3 — Data durability and security depth

### Task 5 — Migration hardening with fixture DBs
**Assumption:** Schema/data evolution needs backward compatibility confidence.  
**Expected outcome:** safe upgrade behavior from historical versions.

**PR slices**
- PR 3.5.1: Create fixture corpus for prior schema versions.
- PR 3.5.2: Add migration integration tests (upgrade, integrity, rollback policy where applicable).
- PR 3.5.3: Add migration checklist gate to release process.

**Acceptance criteria**
- Fixture DBs upgrade successfully and preserve invariants.
- Migration failures are explicit and recoverable.

**Validation**
- `cargo test -p hkask-storage`
- `cargo test -p hkask-services`

---

### Task 6 — Threat-model to regression-test security hardening
**Assumption:** Security posture is strongest when threats map directly to tests.  
**Expected outcome:** persistent automated coverage for key threat classes.

**Initial threat classes**
- Token forgery / malformed token handling
- Replay or duplicate submission behavior
- Capability confusion / boundary bypass attempts
- Protocol envelope fuzzing around daemon/MCP boundaries

**PR slices**
- PR 3.6.1: Publish concise threat-to-test matrix in docs.
- PR 3.6.2: Add regression tests per threat class.
- PR 3.6.3: Add security regression CI target.

**Acceptance criteria**
- Each threat class has at least one reproducible failing test before fix and passing test after.

**Validation**
- security regression suite job

---

## Wave 4 — Spec/doc fidelity and architecture control

### Task 7 — Documentation-as-contract automation
**Assumption:** Doc/spec claims drift unless mechanically linked to verification.  
**Expected outcome:** claims without verifiable anchors are blocked.

**PR slices**
- PR 3.7.1: Define doc claim annotation standard (claim ID → test/verification target).
- PR 3.7.2: Build checker for unresolved claim references.
- PR 3.7.3: Apply to core architecture docs (`PRINCIPLES`, major specs).

**Acceptance criteria**
- CI identifies claims lacking verification anchors.
- High-value docs maintain claim→test traceability.

**Validation**
- doc contract check job

---

### Task 8 — Architectural decoupling enforcement
**Assumption:** dependency drift can silently violate intended layering.  
**Expected outcome:** explicit dependency direction constraints in CI.

**PR slices**
- PR 3.8.1: Encode allowed dependency matrix (layer rules).
- PR 3.8.2: Add automated check for forbidden edges/cycles.
- PR 3.8.3: Fix/waive with explicit rationale for existing edge exceptions.

**Acceptance criteria**
- No new forbidden dependency edges.
- Layering policy visible and enforced.

**Validation**
- dependency policy CI job

---

## Wave 5 — Test-system reliability

### Task 9 — Deterministic/flaky test harness hardening
**Assumption:** flaky tests reduce trust in all gates.  
**Expected outcome:** reproducible CI signal and predictable rerun policy.

**PR slices**
- PR 3.9.1: Add flake classification tags and retry policy guidance.
- PR 3.9.2: Stabilize timing-sensitive tests (timeouts, deterministic seeds, virtual clocks where possible).
- PR 3.9.3: Add flake trend report and quarantine policy for exceptional cases.

**Acceptance criteria**
- Flake rate reduced and tracked.
- Known flaky tests isolated with explicit remediation owner.

**Validation**
- repeated CI run sampling on targeted suites

---

## Wave 6 — Release governance synthesis

### Task 10 — Unified release-readiness gate
**Assumption:** fragmented checks hide go/no-go status.  
**Expected outcome:** one command/report for release quality decision.

**Gate includes**
- contract-tests pass
- conformance suite pass
- resilience scenarios pass
- perf budgets pass
- migration suite pass
- security regressions pass
- doc-contract checks pass
- dependency policy checks pass

**PR slices**
- PR 3.10.1: Add orchestration script/command (e.g., `scripts/release/quality_gate.sh`).
- PR 3.10.2: Emit machine-readable + human-readable summary artifact.
- PR 3.10.3: Wire into release workflow docs and CI.

**Acceptance criteria**
- Single authoritative pass/fail summary for release readiness.
- Failing sub-gates clearly attributed.

**Validation**
- release gate CI pipeline

---

## 4) Suggested Timeline (Phase 3)

- **Week 1:** Tasks 1–2
- **Week 2:** Tasks 3–4
- **Week 3:** Tasks 5–6
- **Week 4:** Tasks 7–8
- **Week 5:** Task 9
- **Week 6:** Task 10 + stabilization + release dry run

---

## 5) Risks and Controls

- **Overhead risk (too many gates):** phase in gates as warning→enforced.
- **Perf benchmark noise:** use controlled runners and median/p95 windows.
- **Migration fixture drift:** version and checksum fixtures; forbid ad-hoc edits.
- **Security suite fragility:** isolate network dependencies; prefer deterministic local harnesses.

---

## 6) Done Definition (Phase 3)

Phase 3 is complete when:

1. Core invariants are codified as executable contracts.
2. Cross-surface parity suite detects behavior drift automatically.
3. Resilience scenarios prove graceful degradation for targeted fault modes.
4. Performance/resource budgets are enforced in CI.
5. Historical schema fixtures pass migration integrity tests.
6. Threat-model classes map to passing security regression tests.
7. Key docs/spec claims are trace-linked to verification artifacts.
8. Dependency direction/layering constraints are CI-enforced.
9. Flake rate is measured and controlled with deterministic harness improvements.
10. Release-readiness is decided via a unified quality gate report.

---

## 7) Recommended kickoff PR (Phase 3)

Start with **Task 1 / PR 3.1.1** (contract harness + first invariant set), because it establishes the assurance foundation that all subsequent gates consume.
