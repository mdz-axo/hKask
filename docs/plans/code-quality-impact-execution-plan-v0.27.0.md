# hKask v0.27.0 — Code Quality & Smell Reduction Execution Plan

**Status:** Proposed  
**Owner:** Engineering  
**Created:** 2026-06-15  
**Scope:** `crates/*` + `mcp-servers/*` (headless-only, no UI additions)

---

## 1) Objective

Execute the previously ranked top-10 improvements in **impact order**, with small, verifiable PR slices, preserving existing behavior while raising reliability, security-boundary consistency, and spec-test traceability (P8).

---

## 2) Guiding Constraints (enforced throughout)

- **Think Before Coding:** each task starts with explicit assumption + expected observable outcome.
- **Simplicity First:** no speculative framework work; only task-linked changes.
- **Surgical Changes:** touch only target seams for each task.
- **Goal-Driven Execution:** every PR has measurable acceptance checks.

Project constraints preserved:
- Headless only.
- No monitoring stack additions.
- No `todo!()`, `unimplemented!()`, or deprecated surface introduced.

---

## 3) Priority Stack (highest impact first)

1. Uniform P4 Gate-3 capability checks across MCP servers.
2. Remove runtime panic-prone `.unwrap()` in public/runtime paths.
3. P8 traceability closure: REQ-tagged tests for public seams.
4. Eliminate pass-through/stub abstractions (notably capability validator).
5. Complete strangler extraction for settings domain.
6. CNS loop integrity upgrades (delay/gain/fidelity observability).
7. Stronger typed CNS span modeling.
8. G2 surface-control policy (`>7` public items justification/containment).
9. Enforce `unsafe` safety-doc policy by CI check.
10. Add headless CI quality gates for persistence of improvements.

---

## 4) Delivery Strategy (6 waves)

## Wave 1 (Security boundary correctness)

### Task 1 — Uniform MCP Gate-3 capability verification
**Assumption:** Startup gate inconsistency creates avoidable OCAP drift risk.  
**Expected outcome:** all MCP servers apply auth + assignment + capability checks consistently.

**PR slices**
- PR 1.1: Extract shared startup verification helper in `hkask-mcp` crate (or shared service utility).
- PR 1.2: Adopt helper in all MCP server mains:
  - `hkask-mcp-condenser`
  - `hkask-mcp-memory`
  - `hkask-mcp-media`
  - `hkask-mcp-companies`
  - `hkask-mcp-docproc`
  - `hkask-mcp-training`
  - `hkask-mcp-replica`
  - `hkask-mcp-communication`
  - `hkask-mcp-spec` (align to same pattern where applicable)
- PR 1.3: Add/expand startup gate tests for at least one representative server + shared helper unit tests.

**Acceptance criteria**
- Every MCP server startup path performs Gate-3 capability verification (or explicit documented exception).
- No server logs “P4 dual-gate complete” without capability validation.

**Validation**
- `cargo test -p hkask-mcp`
- `cargo check --workspace`

---

## Wave 2 (Runtime reliability)

### Task 2 — Replace runtime `.unwrap()` in production paths with typed errors
**Assumption:** panic paths in runtime seams are high-impact reliability risk.  
**Expected outcome:** lock poisoning / missing state yields structured errors, not process aborts.

**PR slices**
- PR 2.1: `hkask-mcp-media` runtime lock/option unwrap removal in tool handlers.
- PR 2.2: `hkask-mcp-docproc` runtime lock unwrap removal.
- PR 2.3: `hkask-templates` registry runtime lock unwrap removal where public seam reachable.

**Acceptance criteria**
- Zero `.unwrap()` in non-test runtime handler paths for targeted crates.
- Error messages map to existing error taxonomy (`McpToolError`/`ServiceError` etc.).

**Validation**
- `cargo test -p hkask-mcp-media`
- `cargo test -p hkask-mcp-docproc`
- `cargo test -p hkask-templates`

---

## Wave 3 (Spec traceability + dead abstraction removal)

### Task 3 — P8 REQ coverage expansion for high-surface crates
**Assumption:** low REQ density on public seams weakens behavioral guarantees.  
**Expected outcome:** measurable increase in public-seam behavioral test traceability.

**Initial target crates (highest public surface):**
- `hkask-types`
- `hkask-agents`
- `hkask-api`

**PR slices**
- PR 3.1: Add REQ-tagged tests for core public seam groups in `hkask-types` (event/span/capability parsing contracts).
- PR 3.2: Add REQ-tagged tests for mode transitions and pod constraints in `hkask-agents` where missing.
- PR 3.3: Add REQ-tagged route-behavior tests in `hkask-api` (auth/error/merge semantics for public endpoints).

**Acceptance criteria**
- Each targeted public seam area has at least one explicit REQ-tagged behavioral test.
- No declarative architectural claim added without provenance in docs/comments.

**Validation**
- `cargo test -p hkask-types`
- `cargo test -p hkask-agents`
- `cargo test -p hkask-api`

---

### Task 4 — Replace pass-through capability validator stub
**Assumption:** pass-through validator masks missing registration-time guarantees.  
**Expected outcome:** either enforce real capability requirement checks or remove dead layer.

**PR slices**
- PR 4.1: Define minimal capability requirement schema for template registration.
- PR 4.2: Implement actual `validate_capabilities` logic against declared requirements.
- PR 4.3: Add REQ-tagged tests for positive, negative, and attenuated-token cases.

**Acceptance criteria**
- `CapabilityAwareValidator` is no longer unconditional `Ok(())`.
- Registration-time rejection path is exercised by tests.

**Validation**
- `cargo test -p hkask-templates`

---

## Wave 4 (Architecture convergence)

### Task 5 — Complete settings-domain strangler extraction
**Assumption:** duplicated CLI/API settings logic causes drift.  
**Expected outcome:** single authoritative settings service used by CLI, API, REPL.

**PR slices**
- PR 5.1: Add `SettingsService` in `hkask-services` (load/merge/validate/save).
- PR 5.2: Make CLI settings command delegate to service.
- PR 5.3: Make API settings routes delegate to service.
- PR 5.4: Keep REPL persistence path aligned to same service contracts.

**Acceptance criteria**
- No duplicate merge-validation implementation between CLI/API.
- Behavior parity proven via REQ tests across surfaces.

**Validation**
- `cargo test -p hkask-services`
- `cargo test -p hkask-cli`
- `cargo test -p hkask-api`

---

### Task 6 — CNS loop telemetry: delay/gain/fidelity observability
**Assumption:** threshold-only loop visibility is insufficient for robust tuning.  
**Expected outcome:** measurable loop-quality fields available from public seams.

**PR slices**
- PR 6.1: Define typed loop-quality data model (`delay_ms`, `gain`, `fidelity_score`).
- PR 6.2: Instrument `increment_variety`/`check_variety` flow to emit these metrics.
- PR 6.3: Add REQ tests validating metric emission and bounds.

**Acceptance criteria**
- Loop quality fields exposed via CNS seams and tested.

**Validation**
- `cargo test -p hkask-cns`

---

### Task 7 — Strengthen span typing beyond string wrapper
**Assumption:** enum-level typing reduces namespace misuse at compile time.  
**Expected outcome:** safer construction/dispatch for canonical span classes.

**PR slices**
- PR 7.1: Introduce typed span-kind enum(s) mapping to canonical namespaces.
- PR 7.2: Migrate high-traffic call sites first (CNS runtime, governed tool, chat paths).
- PR 7.3: Add compatibility adapters for existing string APIs.

**Acceptance criteria**
- New typed constructors used in critical paths.
- Existing external behavior unchanged.

**Validation**
- `cargo test -p hkask-types`
- `cargo test -p hkask-cns`

---

## Wave 5 (Module depth + safety governance)

### Task 8 — Public surface control and justification policy
**Assumption:** very broad crate-level public surfaces increase shallow complexity.  
**Expected outcome:** explicit justifications and/or submodule narrowing for oversized surfaces.

**PR slices**
- PR 8.1: Add policy doc and crate-level `PUBLIC_SURFACE.md` (or equivalent) for oversized crates.
- PR 8.2: Reduce re-export breadth where non-breaking and safe.
- PR 8.3: Add CI/static check to flag newly added un-justified public items in oversized crates.

**Acceptance criteria**
- Oversized crates have documented public-surface rationale.
- Net-new public exports require justification.

---

### Task 9 — Enforce `unsafe` documentation policy in CI
**Assumption:** manual discipline degrades over time.  
**Expected outcome:** every non-test `unsafe` block has proximate `SAFETY:` rationale.

**PR slices**
- PR 9.1: Add repo script/check for `unsafe {` without nearby safety annotation.
- PR 9.2: Wire check into CI.
- PR 9.3: Fix any newly-detected violations.

**Acceptance criteria**
- CI fails on undocumented non-test unsafe blocks.

---

## Wave 6 (Sustainment)

### Task 10 — Headless quality gates for regression prevention
**Assumption:** one-time cleanup will regress without automated gates.  
**Expected outcome:** continuous enforcement of core quality signals.

**Gates to add**
- REQ traceability trend (public seam proxy metrics).
- Runtime `.unwrap()` denylist checks for selected crates.
- MCP startup Gate-3 consistency check.
- Unsafe-doc compliance check.

**Acceptance criteria**
- CI exposes pass/fail signals for all four gates.
- New regressions blocked before merge.

---

## 5) Suggested Timeline

- **Week 1:** Tasks 1–2
- **Week 2:** Task 3 (phase A) + Task 4
- **Week 3:** Task 5
- **Week 4:** Tasks 6–7
- **Week 5:** Tasks 8–9
- **Week 6:** Task 10 + stabilization

---

## 6) Risk Register (top)

- **Behavioral drift risk** while replacing unwrap paths → mitigate with REQ tests before/after each seam change.
- **Cross-crate migration risk** in settings strangler work → keep adapters and do phased delegation.
- **Typing migration churn** for spans → incremental adoption with compatibility constructors.

---

## 7) Done Definition (program level)

This plan is complete when all are true:

1. All MCP servers consistently enforce startup capability verification semantics.
2. Targeted runtime seams no longer panic via `.unwrap()` under expected fault modes.
3. High-surface crates show materially improved REQ-tagged public seam coverage.
4. Pass-through validator abstraction is removed or made behaviorally meaningful.
5. Settings logic is centralized in `hkask-services` with CLI/API/REPL parity.
6. CNS loop quality metrics (delay/gain/fidelity) are exposed and tested.
7. Span typing is strengthened in critical paths.
8. Oversized public surfaces are justified and governed.
9. Unsafe documentation policy is CI-enforced.
10. Headless quality gates prevent regression.

---

## 8) Recommended first PR kickoff (immediate)

Start with **Task 1 / PR 1.1**:
- Add shared MCP startup verifier utility (`auth -> assignment -> capability check list`)
- Include tests with mocked daemon responses
- Keep server-specific required tool lists configurable

This yields the highest immediate risk reduction per LOC changed.
