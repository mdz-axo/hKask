---
title: "Service Layer Extraction & Architecture Deepening Plan"
plan_id: "PLAN-2026-06-27-001"
status: "In Progress"
created: 2026-06-27
version: "0.1.0"
domain: "Architecture"
mds_categories: [domain, composition, lifecycle]
dependencies: []
---

# Service Layer Extraction & Architecture Deepening Plan

**Target:** Address architectural candidates 1–4 from the `improve-codebase-architecture` exploration (2026-06-27).

**Governing skills:** coding-guidelines, pragmatic-semantics, pragmatic-cybernetics, idiomatic-rust, essentialist, grill-me.

---

## Epistemic Classification

| Axis | Classification |
|------|---------------|
| Ontological Mode | Prescriptive (OUGHT) — recommended structural changes |
| Epistemic Mode | Probabilistic — friction is observed (IS), solutions are recommended, precise impact depends on implementation |
| Domain Anchoring | Core (5W1H) + Dual-Axis (PKO process: migration steps; DC+BIBO state: crate identities) |

---

## Cybernetic Framing

Each candidate is a feedback loop disruption in the system's regulatory model:

| Candidate | Broken Property | Consequence |
|-----------|----------------|-------------|
| C1 — `hkask-services` hub | **Fidelity** — regulator can't distinguish service boundaries | Variety counter sees "one big crate", misses individual service drift |
| C2 — `hkask-cli` monolith | **Closure** — business logic in CLI bypasses service-layer governance | OCAP enforcement only at surface, not at seam |
| C3 — Wide public interfaces | **Gain** — too many public items → noise drowns signal → regulator blind | Each pub fn is a disturbance path; 35 paths from one module overwhelm Ashby's Law |
| C4 — Low test coverage | **Model-reality divergence** — no behavioral verification → Good Regulator has no model | CNS spans capture activity but contracts are unverified; the regulator regulates a phantom |

---

## Step 0: Pre-Flight Infrastructure

### Step 0.1 — Verify Build & Existing Tests

**Status:** ✅ Complete (2026-06-27)

```
Goal: "cargo build --workspace" and "cargo test --workspace" both pass clean
Contract: The workspace compiles with zero errors. All existing tests pass.
Verify: Run both commands. Record any pre-existing failures as baseline.
```

**Results:**
- `cargo build --workspace` — ✅ Clean (finished in 2m 51s, all crates compiled)
- `cargo test --workspace --no-fail-fast` — ✅ Zero failures across all crates
  - All unit tests, integration tests, and doc-tests pass
  - Some doc-tests are `#[ignore]`'d (8 in hkask-storage, 8 in hkask-mcp, 7 in hkask-test-harness, 1 in hkask-tui) — these require runtime infrastructure (Matrix server, DB, etc.)
  - Pre-existing baseline: **no regressions**
- `cargo test -p hkask-cli --lib` — ✅ 49 passed, 0 failed

### Step 0.2 — Create Baseline Test Contracts for Candidate Files

**Status:** ❌ Skipped — per essentialist principle, testing pre-refactor code calcifies the problems we're fixing. Contract tests will be written against the *simplified* interfaces after extraction, not before.

### Step 0.3 — ADR-040: Record Migration Intent

**Status:** ✅ Complete (2026-06-27)

ADR created at `docs/architecture/ADRs/ADR-040-service-layer-extraction.md`.
Documents: problem statement, current state (15 modules, 5,322 LoC), extraction strategy, end state options, rollback plan.

---

## Candidate 1: `hkask-services` — Dependency Hub

**Recommendation:** Strong

### Step 1.1 — Inventory the 15 Remaining Modules

**Status:** ✅ Complete (2026-06-27)

Full inventory with disposition decisions:

### Step 1.2 — Extract One Module at a Time (Strangler-Fig)

Per-module workflow:
1. Verify existing behavioral contract tests pass
2. Move module file to destination crate. Update Cargo.toml deps.
3. Add `pub use` in destination crate's `lib.rs`
4. Update `use` statements in CLI, API, MCP callers
5. Remove module from `hkask-services/src/lib.rs`
6. `cargo test -p <destination>` passes. `cargo build --workspace` passes.
7. One commit per module.

**Inventory with dispositions:**

| # | Module | Lines | Fns (pub) | Sister Crate? | Disposition | Rationale | Effort |
|---|--------|------:|----------:|--------------|-------------|-----------|--------|
| 1.2a | `experience` | 128 | 3 (2) | No | **→ hkask-cli** | CLI daemon bridge. Not a domain service — it's a CLI→daemon adapter. Uses `hkask_mcp::DaemonClient`. | Small |
| 1.2b | `consolidation` | 115 | 4 (4) | No | **→ hkask-services-core** | Small consolidation utility. 4 pub fns, fits core. | Small |
| 1.2c | `cloud` | 259 | 8 (7) | No | **→ hkask-cli** | Hetzner deployment config from env vars. Pure infrastructure provisioning, not a domain service. Folded from `hkask-services-cloud`. | Small |
| 1.2d | `cns` | 137 | 9 (6) | `hkask-cns` | **→ inline at call sites** | Convenience wrapper around `Arc<RwLock<CnsRuntime>>`. Callers can use `CnsRuntime` directly. 137 lines of `read().await.xxx().await`. | Small |
| 1.2e | `federation` | 142 | 8 (8) | `hkask-federation` | **→ hkask-federation** | Federation lifecycle. `hkask-federation` currently only has types (10 LoC). The service logic is here. Move it. | Small |
| 1.2f | `memory` | 210 | 6 (6) | `hkask-memory` | **→ hkask-memory** | Memory service operations. Wraps types from sister crate. Merge into sister crate. | Small |
| 1.2g | `skills` | 642 | 16 (7) | `hkask-services-skill` | **→ hkask-services-skill** | Dual-layer skill audit. Sister crate exists. Move code. | Medium |
| 1.2h | `bundle` | 329 | 7 (7) | No | **→ hkask-services-skill** | Bundle composition via LLM. Bundles are composed skills — belongs with skill service. | Small |
| 1.2i | `archival` | 339 | 7 (4) | No | **→ hkask-services-core** | Snapshot/archive operations. Small, fits core. | Small |
| 1.2j | `lifecycle` | 292 | 14 (3) | No | **→ hkask-services-core** | Server lifecycle (health, start/stop). 3 pub fns. Fits core. | Small |
| 1.2k | `verification` | 535 | 14 (2) | No | **→ hkask-services-core** | Manifest verification. Only 2 pub fns. Internal complexity is verification logic. Fits core. | Small |
| 1.2l | `compose` | 480 | 9 (2) | No | **→ new hkask-services-compose** | Prompt composition with cognition config. 2 pub fns, 480 lines of internal logic. Earns its own crate. | Medium |
| 1.2m | `curator` | 313 | 8 (4) | No | **→ hkask-agents** | Curator service. Curator logic belongs with agent orchestration in hkask-agents. | Medium |
| 1.2n | `chat` | 1,401 | 33 (13) | No | **→ new hkask-services-chat** | Core chat orchestration. Deepest module. 1,401 lines, 33 functions. Earns its own crate. Extracted LAST due to coupling. | Large |

**Deletion test applied:**
- `skill` module (line 55 in lib.rs): this is actually a small `pub mod skill;` that only re-exports `resolve_replicant_name` from `hkask-services-skill`. It's a one-line pass-through — inline at call sites.
- All other modules pass the deletion test — complexity would reappear across CLI/API if deleted. But they belong in focused crates, not a hub.

### Step 1.3 — Services Becomes a Facade

**Status:** Pending

After all modules extracted, `hkask-services` becomes a re-export facade or is deleted.

### Step 1.4 — Documentation Update

**Status:** Pending

- `docs/architecture/hKask-architecture-master.md` — update crate listing
- `docs/architecture/ADRs/ADR-040-service-layer-extraction.md` — add completion notes

---

## Candidate 2: `hkask-cli` — CLI Monolith

**Recommendation:** Strong

### Step 2.1 — Audit REPL Handlers for Business Logic

**Status:** Pending

Classify every handler as "presentation-only" or "contains business logic."

### Step 2.2 — Extract Business Logic Handlers

| # | Handler | Lines | Logic to Extract | Destination | Effort |
|---|---------|-------|-----------------|-------------|--------|
| 2.2a | handlers/kanban.rs | 1,098 | 199 match arms of kanban logic | hkask-services-kanban | Large |
| 2.2b | handlers/talk.rs | ~500 | Chat composition logic | hkask-services (chat) | Medium |
| 2.2c | handlers/mcp.rs | ~400 | MCP dispatch logic | hkask-mcp or hkask-services | Medium |
| 2.2d | handlers/feedback.rs | ~300 | Feedback validation | hkask-services-core | Small |
| 2.2e | handlers/repl_settings.rs | ~300 | Settings logic | hkask-services-core | Small |
| 2.2f | tui_bridges.rs | 1,058 | TUI ↔ service bridge | Evaluate | Medium |
| 2.2g | cli/actions.rs | 1,041 | CLI action dispatch | Evaluate | Medium |

### Step 2.3 — Documentation Update

**Status:** Pending

---

## Candidate 3: Giant Files with Wide Interfaces

**Recommendation:** Strong

### Step 3.1 — `hkask-mcp/src/server.rs` (39 pub fns → ≤12)

**Status:** Pending

Split into sub-modules: connection, tools, resources, prompts.

### Step 3.2 — `hkask-services-context/src/context_impl.rs` (35 pub fns → ≤21)

**Status:** Pending

Split by access pattern: read, write, admin.

### Step 3.3 — `hkask-storage/src/wallet_store.rs` (25 pub fns → ≤14)

**Status:** Pending

Split by entity: transaction, account.

### Step 3.4 — Documentation Update

**Status:** Pending

---

## Candidate 4: Low Test Coverage on Giant Files

**Recommendation:** Strong

### Step 4.1 — Property-Based Test Expansion

Runs in parallel with extraction work.

| # | Test Target | Type | Invariant |
|---|------------|------|-----------|
| 4.1a | hkask-services-archival | Round-trip | Archive → retrieve → content matches |
| 4.1b | hkask-services-consolidation | Idempotency | Consolidate twice = same result |
| 4.1c | hkask-services-skill | Round-trip | Load skill → serialize → deserialize → same |
| 4.1d | hkask-services-kanban | State machine | Valid transitions only |
| 4.1e | hkask-services-context | Read/Write | Write → Read → value matches |
| 4.1f | hkask-mcp | Protocol | Request → Response round-trip |
| 4.1g | hkask-storage | Transaction | Sign → Submit → Query → status matches |

### Step 4.2 — Integration Tests for Cross-Crate Flows

**Status:** Pending

### Step 4.3 — Documentation Update

**Status:** Pending

- `docs/architecture/core/TESTING_DISCIPLINE.md` — add coverage targets
- `docs/architecture/ADRs/ADR-040` — add testing section

---

## Master Execution Order

```
Step 0 (Pre-flight)
  ├── 0.1: Build baseline
  ├── 0.2: Contract tests for 10 giant files
  └── 0.3: ADR-040

Step 1 (Candidate 1: Services hub) — depends on 0.1, 0.3
  ├── 1.1: Inventory modules
  ├── 1.2a-1.2o: Extract modules one at a time
  │     └── [parallel with Step 4 as modules complete]
  └── 1.3: Services becomes facade

Step 2 (Candidate 2: CLI monolith) — depends on 1.2
  ├── 2.1: Audit handlers
  └── 2.2a-2.2g: Extract handler logic

Step 3 (Candidate 3: Wide interfaces) — depends on 1.2
  ├── 3.1: Split server.rs (parallel to Step 2)
  ├── 3.2: Split context_impl.rs
  └── 3.3: Split wallet_store.rs

Step 4 (Candidate 4: Test coverage) — runs alongside 1, 2, 3
  ├── 4.1a-4.1g: Per-module tests
  └── 4.2: Integration tests

Step 5: Final documentation sweep
```

---

## Adversarial Risk Register

| # | Risk | Mitigation |
|---|------|-----------|
| 1 | Circular deps in modules prevent extraction | Extract into same crate; knot = co-dependency signal |
| 2 | Contract tests unwritable due to coupling | Skip, extract first, test narrowed interface |
| 3 | Build breaks for days | Atomic commits; revert single commit, try different order |
| 4 | TUI crate depends on hkask-services modules | TUI deps shrink as side effect of extraction |
| 5 | Documentation drift from old crate names | Global grep sweep in Step 5; mechanical, not architectural |
| 6 | 40+ commits unreviewable | First 3 commits establish pattern; rest are mechanical spot-checks |

---

## Expected Outcomes

| Metric | Before | After (Target) |
|--------|--------|----------------|
| `hkask-services` modules | 15 | 0–2 (facade or deleted) |
| `hkask-services` hkask-* deps | 27 | 0 (if deleted) or ≤5 (if facade) |
| `hkask-cli` lines of code | 18,023 | ≤12,000 |
| `server.rs` pub fns | 39 | ≤12 |
| `context_impl.rs` pub fns | 35 | ≤21 (via 3 sub-modules) |
| `wallet_store.rs` pub fns | 25 | ≤14 |
| Integration test files | 39 | ≥50 |
| Giant files (>1,000 lines) | 10 | ≤3 |
| New ADRs | — | 1 (ADR-040) |
