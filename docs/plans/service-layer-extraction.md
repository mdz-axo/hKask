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

**Status:** Pending

```
Goal: "cargo build --workspace" and "cargo test --workspace" both pass clean
Contract: The workspace compiles with zero errors. All existing tests pass.
Verify: Run both commands. Record any pre-existing failures as baseline.
```

### Step 0.2 — Create Baseline Test Contracts for Candidate Files

**Status:** Pending

Each of the 10 giant files gets a behavioral contract test.
Per-file: identify ONE invariant, write a proptest.

| # | Sub-step | Crate | File | Lines | Effort |
|---|----------|-------|------|-------|--------|
| 0.2a | discover_impl contract | hkask-services-corpus | discover_impl.rs | 1,726 | Medium |
| 0.2b | embed_impl contract | hkask-services-corpus | embed_impl.rs | 1,717 | Medium |
| 0.2c | context_impl contract | hkask-services-context | context_impl.rs | 1,618 | Medium |
| 0.2d | wallet_store contract | hkask-storage | wallet_store.rs | 1,534 | Low |
| 0.2e | chat.rs contract | hkask-services | chat.rs | 1,401 | Medium |
| 0.2f | executor contract | hkask-templates | executor.rs | 1,371 | Medium |
| 0.2g | splash contract | hkask-tui | splash.rs | 1,344 | Low |
| 0.2h | server contract | hkask-mcp | server.rs | 1,342 | Medium |
| 0.2i | algorithms contract | hkask-condenser | algorithms.rs | 1,335 | Low |
| 0.2j | metacognition contract | hkask-agents | metacognition.rs | 1,179 | Medium |

### Step 0.3 — ADR-040: Record Migration Intent

**Status:** Pending

```
Goal: Create ADR-040 documenting the service-layer extraction strategy
Contract: ADR follows _TEMPLATE.md. States: what, why, migration pattern (strangler-fig), rollback plan.
Verify: ADR exists at docs/architecture/ADRs/ADR-040-service-layer-extraction.md
```

---

## Candidate 1: `hkask-services` — Dependency Hub

**Recommendation:** Strong

### Step 1.1 — Inventory the 15 Remaining Modules

**Status:** Pending

Produce migration table: module name → destination crate or DELETE.

### Step 1.2 — Extract One Module at a Time (Strangler-Fig)

Per-module workflow:
1. Verify existing behavioral contract tests pass
2. Move module file to destination crate. Update Cargo.toml deps.
3. Add `pub use` in destination crate's `lib.rs`
4. Update `use` statements in CLI, API, MCP callers
5. Remove module from `hkask-services/src/lib.rs`
6. `cargo test -p <destination>` passes. `cargo build --workspace` passes.
7. One commit per module.

| # | Module | Destination | Rationale | Effort |
|---|--------|-------------|-----------|--------|
| 1.2a | archival | hkask-services-core or new | Leaf module | Small |
| 1.2b | consolidation | hkask-services-core or new | Leaf module | Small |
| 1.2c | experience | hkask-services-core or new | Leaf module | Small |
| 1.2d | compose | hkask-services-core or new | Leaf module | Small |
| 1.2e | skills | hkask-services-skill | Sister crate exists | Small |
| 1.2f | bundle | hkask-services-skill | Bundles are skill composition | Small |
| 1.2g | skill | hkask-services-skill | Sister crate exists | Small |
| 1.2h | cns | hkask-cns or refactor | Evaluate | Medium |
| 1.2i | verification | hkask-services-core | Cross-cutting, fits core | Small |
| 1.2j | curator | hkask-agents | Curator logic belongs with agent code | Medium |
| 1.2k | federation | hkask-federation | Sister crate exists | Small |
| 1.2l | lifecycle | hkask-services-core or new | Medium complexity | Medium |
| 1.2m | cloud | hkask-api or hkask-cli | Cloud provisioning is infra, not service | Medium |
| 1.2n | memory | hkask-memory | Sister crate exists | Small |
| 1.2o | chat | own crate or hkask-services-core | Deep dependency, 1,401 lines | Large |

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
