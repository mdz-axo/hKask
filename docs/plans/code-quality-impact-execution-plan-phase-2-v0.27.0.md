# hKask v0.27.0 — Code Quality Execution Plan (Phase 2)

**Status:** In Progress — Wave 1 (Tasks 1–1.1 ✅, Task 2 next)
**Depends on:** `docs/plans/code-quality-impact-execution-plan-v0.27.0.md` (Phase 1)  
**Created:** 2026-06-15  
**Last Updated:** 2026-06-15  
**Scope:** `crates/*`, `mcp-servers/*`, CI, and `docs/status/*`

---

## 1) Objective

Drive sustained quality after Phase 1 by hardening specification traceability (P8), protocol stability (P4 boundaries), error consistency, and long-term maintainability through automated checks and contract-based testing.

---

## 2) Priority Stack (Phase 2, highest impact first)

1. Public seam inventory generator (pub API ↔ REQ mapping)
2. Mutation/property testing for critical parsers and validators
3. MCP error taxonomy standardization
4. Daemon protocol compatibility contract tests (version skew resilience)
5. Idempotency/retry-safety tests for critical operations
6. Oversized crate root façade refactor (non-breaking)
7. Concurrency stress tests for lock-heavy runtime paths
8. Stronger typed IDs/newtypes at API boundaries
9. Strangler migration playbook template
10. Quality trend reporting in `docs/status`

---

## 3) Delivery Strategy (5 waves)

## Wave 1 — Visibility + correctness baseline

### Task 1 — Public seam inventory generator ✅
**Assumption:** P8 can't be sustained without an always-current seam map.  
**Expected outcome:** machine-generated inventory of public seams and REQ-test linkage.

**PR slices**
- ✅ PR 2.1.1: `scripts/audit/public-seam-inventory.sh` — bash generator (zero new deps). Walks all 26 workspace members, extracts 5 public item kinds (fn/struct/enum/trait/type), strips `#[cfg(test)]` blocks, cross-references `// REQ:` tags. `--write` and `--check` modes.
- ✅ PR 2.1.2: `docs/status/public-seam-inventory.md` — 2,336 public items, 572 REQ tests (initial). After Task 1.1 fix: 2,342 items, 579 REQ tests, **39% coverage** (corrected from inflated 49%). Per-crate detail tables with relative paths and risk-tier classification.
- ✅ PR 2.1.3: `.github/workflows/ci.yml` — added `Public seam inventory check (P8 traceability)` step to `security-invariants` job. Fails on drift.

**Acceptance criteria**
- ✅ Inventory generated deterministically in CI.
- ✅ Delta in seam/test mapping visible in PRs.

**Artifacts**
- `scripts/audit/public-seam-inventory.sh` (465 lines)
- `docs/status/public-seam-inventory.md` (2,565 lines)
- `.github/workflows/ci.yml` (+3 lines)

---

### Task 1.1 — Inventory quality fix + first-pass triage ✅
**Assumption:** the raw inventory has known quality gaps that must be fixed before subsequent tasks can rely on it.  
**Expected outcome:** accurate coverage numbers, risk-tiered inventory, and an actionable priority list.

**Findings from Task 1**
- Cross-crate name matching inflated coverage by ~10pp: `hkask-mcp` reported 15 covered with 0 REQ tests.
- `hkask-api` at 2% is the worst-covered high-surface crate (3 covered / 137 items, 1 REQ test).
- Five crates with 0 covered items: `hkask-mcp`, `hkask-mcp-communication`, `hkask-mcp-memory`, `hkask-mcp-spec`, `hkask-mcp-condenser`.
- ~200 uncovered fns are accessor/constructor patterns — low risk individually.
- 876 items classified as high-risk uncovered (see priority list).

**PR slices**
- ✅ PR 2.1.4: Fixed cross-crate matching — scoped name-proximity to crate via `${cr}:${rid}:${rdesc}:${tfn}` term format and `grep -qi "^${cr}:.*${name}"` check.
- ✅ PR 2.1.5: Added `classify_risk()` function with 5-tier classification (Accessor/Constructor, Type Declaration, MCP Tool Handler, API Route Handler, Core Logic). Risk tier column added to inventory detail tables.
- ✅ PR 2.1.6: Generated `docs/status/public-seam-priority.md` — top-100 high-risk uncovered items (876 total high-risk across all crates). Per-crate summary included.

**Acceptance criteria**
- ✅ Crates with 0 REQ tests show 0% coverage.
- ✅ Per-item risk tier visible in inventory.
- ✅ Top-100 priority list routes to specific crate.

**Corrected Metrics (post-fix)**

| Metric | Before (buggy) | After (fixed) |
|--------|---------------|---------------|
| Overall coverage | 49% | **39%** |
| `hkask-api` | 10% | **2%** |
| `hkask-agents` | 31% | **11%** |
| `hkask-mcp` | 23% | **0%** |
| High-risk uncovered | — | **876** |

**Artifacts**
- `scripts/audit/public-seam-inventory.sh` — added `classify_risk()` and `generate_priority_list()` (+120 lines)
- `docs/status/public-seam-priority.md` — top-100 priority list with per-crate breakdown

---

### Task 2 — Mutation/property testing for critical parsers
**Assumption:** conventional unit tests miss parser edge cases and invariants.  
**Expected outcome:** stronger guarantees for parsing and token handling seams.

**Initial target seams**
- Capability/token parsing + attenuation checks
- Span namespace/category parsing
- Settings parsing/merging
- Spec/capture input validation boundaries

**PR slices**
- PR 2.2.1: Add property tests in `hkask-types` for capability/span invariants.
- PR 2.2.2: Add property tests in `hkask-api`/`hkask-services` for settings merge invariants.
- PR 2.2.3: Add mutation test pass (or lightweight equivalent) for selected parsers.

**Acceptance criteria**
- Property tests cover malformed/partial/round-trip scenarios.
- At least one mutation-equivalent check catches intentionally injected parser faults.

**Validation**
- `cargo test -p hkask-types`
- `cargo test -p hkask-api`
- `cargo test -p hkask-services`

---

## Wave 2 — Boundary consistency + protocol durability

### Task 3 — MCP error taxonomy standardization
**Assumption:** inconsistent error envelopes degrade reliability for all clients.  
**Expected outcome:** predictable cross-server error behavior.

**PR slices**
- PR 2.3.1: Define canonical MCP error mapping table (`invalid_argument`, `permission_denied`, `internal`, etc.).
- PR 2.3.2: Align all MCP servers to shared mapping helper.
- PR 2.3.3: Add golden tests for representative tool failures in each category.

**Acceptance criteria**
- Equivalent faults produce equivalent error class and shape across servers.
- No ad-hoc string-only error contracts for expected failure modes.

**Validation**
- `cargo test -p hkask-mcp`
- `cargo test -p hkask-mcp-spec`
- `cargo test -p hkask-mcp-research`

---

### Task 4 — Daemon protocol compatibility contract tests
**Assumption:** protocol drift between daemon/CLI/MCP creates hidden break risk.  
**Expected outcome:** explicit compatibility contract with versioned behavior tests.

**PR slices**
- PR 2.4.1: Define protocol schema contract for request/response variants.
- PR 2.4.2: Add compatibility tests for older/newer envelope tolerance.
- PR 2.4.3: Add explicit failure-mode tests (unknown fields, missing optional fields, unsupported variants).

**Acceptance criteria**
- Protocol behavior under version skew is tested/documented.
- Contract tests fail on incompatible changes.

**Validation**
- `cargo test -p hkask-mcp daemon`

---

### Task 5 — Idempotency and retry-safety tests
**Assumption:** retry paths are under-tested and can corrupt state or duplicate effects.  
**Expected outcome:** critical operations are explicitly safe under retries.

**Target operations**
- Capability/assignment checks
- Experience persistence
- Backup/restore metadata operations
- Selected wallet and settings operations

**PR slices**
- PR 2.5.1: Define idempotency contract matrix (operation × retry semantics).
- PR 2.5.2: Add tests for duplicate calls and interrupted call replay.
- PR 2.5.3: Fix any discovered non-idempotent behavior where required.

**Acceptance criteria**
- Documented and tested retry behavior for all critical operations.

**Validation**
- `cargo test -p hkask-services`
- `cargo test -p hkask-mcp`
- `cargo test -p hkask-storage`

---

## Wave 3 — Structure and maintainability

### Task 6 — Oversized crate root façade refactor (non-breaking)
**Assumption:** very broad root exports increase coupling and reduce depth.  
**Expected outcome:** cleaner public façades while preserving external API compatibility.

**PR slices**
- PR 2.6.1: Identify top 3 oversized crate roots (`hkask-services`, `hkask-types`, `hkask-storage`) and classify exports.
- PR 2.6.2: Introduce internal façade modules (`mod api`, `mod internals`) and re-export intentionally.
- PR 2.6.3: Deprecation-free migration preserving existing external call sites (no breaking semver changes in v0.27.x line).

**Acceptance criteria**
- Reduced root-level export sprawl without behavior changes.
- Public surface intent documented.

**Validation**
- `cargo check --workspace`
- crate-level tests for touched crates

---

### Task 7 — Concurrency stress tests for lock-heavy paths
**Assumption:** lock contention/deadlock risks remain after panic-path cleanup.  
**Expected outcome:** confidence under concurrent access and burst loads.

**Target seams**
- `hkask-mcp-media` state locks
- `hkask-mcp-docproc` index/cv accumulators
- `hkask-templates` registry sqlite lock paths

**PR slices**
- PR 2.7.1: Add multi-thread stress harnesses per target crate.
- PR 2.7.2: Add timeout-based deadlock detection in tests.
- PR 2.7.3: Apply minimal lock-scope reductions if tests expose contention hotspots.

**Acceptance criteria**
- No deadlocks in stress tests.
- Measurable contention hotspots documented.

**Validation**
- `cargo test -p hkask-mcp-media -- --nocapture`
- `cargo test -p hkask-mcp-docproc -- --nocapture`
- `cargo test -p hkask-templates -- --nocapture`

---

## Wave 4 — Type strengthening + migration discipline

### Task 8 — Stronger typed IDs/newtypes at API boundaries
**Assumption:** stringly request/response fields permit invalid states.  
**Expected outcome:** validated types at ingress/egress for high-risk domains.

**PR slices**
- PR 2.8.1: Inventory stringly identifiers/flags in API and MCP inputs.
- PR 2.8.2: Introduce newtypes/enums with fallible constructors for top-risk fields.
- PR 2.8.3: Add REQ-tagged tests for rejection/acceptance boundaries.

**Acceptance criteria**
- Reduced stringly seams in selected boundary structs.
- Invalid states prevented by type/API contract.

**Validation**
- `cargo test -p hkask-api`
- `cargo test -p hkask-types`

---

### Task 9 — Strangler migration playbook template
**Assumption:** repeated migrations need a standardized safety template.  
**Expected outcome:** each future extraction follows dual-path delegation discipline.

**PR slices**
- PR 2.9.1: Add `docs/plans/strangler-migration-playbook.md` template with mandatory checklist:
  - old path delegates
  - new path parity
  - no premature deletion
  - rollback criteria
- PR 2.9.2: Add link from architecture/principles docs and contributor guidance.

**Acceptance criteria**
- Future service extractions use one standard migration checklist.

**Validation**
- Docs lint/check if present

---

## Wave 5 — Sustained governance

### Task 10 — Quality trend reporting in `docs/status`
**Assumption:** quality work regresses without visible trend telemetry.  
**Expected outcome:** periodic, auditable quality snapshots.

**Metrics (headless, script-generated)**
- Public seams vs REQ-tagged test coverage trend
- Runtime `.unwrap()` hotspot counts (non-test)
- MCP Gate-3 consistency status
- Undocumented `unsafe` count

**PR slices**
- PR 2.10.1: Add status report generator script.
- PR 2.10.2: Emit `docs/status/code-quality-trends.md`.
- PR 2.10.3: CI job to refresh/validate trend report.

**Acceptance criteria**
- Trend report updates are part of normal CI workflow.
- Regressions become visible in review.

**Validation**
- CI report generation job passes

---

## 4) Suggested Timeline (Phase 2)

- **Week 1:** Tasks 1–2
- **Week 2:** Tasks 3–4
- **Week 3:** Task 5 + Task 6 (start)
- **Week 4:** Task 6 (finish) + Task 7
- **Week 5:** Tasks 8–9
- **Week 6:** Task 10 + stabilization

---

## 5) Risks and Controls

- **Test runtime blow-up** (property/stress tests): isolate nightly/extended jobs if needed.
- **Non-breaking façade refactor complexity:** preserve compatibility with conservative re-exports.
- **Protocol contract churn:** gate schema changes via explicit versioning and migration notes.

---

## 6) Done Definition (Phase 2)

Phase 2 is complete when:

1. Public seam inventory is generated and CI-enforced.
2. Critical parser/property invariants are covered with property/mutation-style tests.
3. MCP error taxonomy is consistent across servers.
4. Daemon protocol compatibility is contract-tested for version skew scenarios.
5. Critical operations have documented retry/idempotency behavior and tests.
6. Oversized crate root surfaces are reorganized behind intentional façades.
7. Lock-heavy runtime paths pass concurrency stress tests.
8. Selected API boundaries are upgraded from stringly fields to validated types.
9. Strangler migration playbook exists and is adopted for new migrations.
10. Code-quality trend reporting is generated under `docs/status` and maintained by CI.

---

## 7) Recommended kickoff PR (Phase 2)

Start with **Task 1 / PR 2.1.1** (public seam inventory generator), because it improves decision quality for every subsequent task and creates an objective baseline for P8 improvements.
