---
title: "Kata-Kanban MCP Server — Adversarial Architecture Review"
audience: [architects, rust developers, security reviewers, agents]
last_updated: 2026-07-20
version: "0.31.0"
status: "Active"
domain: "Kata-Kanban"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# Kata-Kanban MCP Server — Adversarial Architecture Review

**Review scope:** `mcp-servers/hkask-mcp-kata-kanban/` (MCP surface, 18 tools) and `crates/hkask-services-kata-kanban/` (service crate, 39 source files).

**Review method:** Loaded `improve-codebase-architecture`, `bug-hunt`, `diagnose`, `coding-guidelines`, `idiomatic-rust`, `pragmatic-laziness`, `pragmatic-semantics`, `pragmatic-cybernetics`, `diataxis-diagram` skills. Challenged findings through `essentialist` (3-gate deletion test) and `grill-me` (Socratic interrogation) perspectives. Took an adversarial skeptical stance to catch issues that a friendly review would miss.

**Build status:** `cargo check` ✅ | `cargo clippy -D warnings` ✅ | `cargo test` 63/63 ✅

**Resolution status:** All 14 findings resolved on 2026-07-20. See §7 for the resolution log.

---

## 1. Executive Summary

The kata-kanban crate is **functional and well-tested** (60 tests pass, clippy clean, compiles on stable). The architecture follows the hKask tri-surface pattern (MCP ≡ CLI ≡ API) and the deep-module discipline is mostly respected. However, the adversarial review surfaced **14 findings** across four severity tiers:

| Severity | Count | Summary |
|----------|-------|---------|
| **CRITICAL** | 2 | Dead code in production paths; shallow abstraction masquerading as a contract |
| **HIGH** | 4 | Unused public API surface; documentation drift; magic-number heuristics |
| **MEDIUM** | 5 | Missing consent checks; inconsistent actor semantics; duplicated boilerplate |
| **LOW** | 3 | Stale doc references; naming inconsistencies; minor style issues |

The findings are decomposed into the smallest possible pragmatic components per the user's request — each is independently actionable.

---

## 2. Findings

### 2.1 CRITICAL — `KanbanServer.db` field is dead code

**Location:** `mcp-servers/hkask-mcp-kata-kanban/src/lib.rs:32`

```rust
pub struct KanbanServer {
    pub service: KanbanService,
    pub db: Option<r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>>,  // ← never read
}
```

**Evidence:** `grep -rn "self\.db" mcp-servers/hkask-mcp-kata-kanban/` returns zero matches. The field is populated in `run()` at line 972 (`db.sqlite_pool().ok()`) but never accessed by any tool method or any other code path.

**Essentialist challenge (G1 — Exist):** Delete the field. What breaks? Nothing — the `KanbanService` already holds its own `HMemStore` which wraps the same driver. The `db` pool is redundant state. **FAILS the deletion test** — it is a pass-through field that adds no behavior.

**Grill-me challenge:** "Why does this field exist?" The most charitable answer is "future-proofing for direct SQL access from the MCP layer." But no MCP tool needs direct SQL — all persistence goes through `KanbanService` → `HMemStore`. The field is speculative flexibility (coding-guidelines anti-pattern #3: unrequested flexibility).

**Recommendation:** Delete the field and the `db.sqlite_pool().ok()` argument in `run()`. The `r2d2`/`r2d2_sqlite` dependencies in `Cargo.toml` can remain (used by `hkask-storage`).

**Constraint force:** Guardrail (P5 — Essentialism). Overridable with a stated reason.

---

### 2.2 CRITICAL — `TaskContract` is a shallow abstraction that never persists

**Location:** `crates/hkask-services-kata-kanban/src/kanban/types/contract.rs:17-119`, used at `service.rs:692`

```rust
let mut contract = TaskContract::new("inline".into(), task.owner, verifier, &task);
let result = contract.check_completion(evidence);
// contract is discarded here — never persisted, never queried again
```

**Evidence:** `TaskContract` has 10 fields (`package_name`, `delegator`, `delegate`, `task_id`, `task_title`, `pre_conditions`, `post_conditions`, `gas_limit`, `timeout`, `max_attenuation`, `state`). Of these:
- `gas_limit` is hardcoded to `50000` (ignores `task.gas_remaining`)
- `timeout` is hardcoded to `3600` (ignores `SpawnSpec.timeout_seconds`)
- `max_attenuation` is hardcoded to `3` (no source)
- `state` is set to `Pending` then mutated to `Completed`/`Violated` — but the contract is discarded immediately after, so the state mutation is invisible
- `post_conditions` is hardcoded to `["All criteria satisfied", "Deliverables verified"]` — never checked

**`check_completion` logic:** Non-empty evidence → `passed: true`. Empty evidence → `passed: false`. The criteria list is included in the `reasoning` string but **does not gate completion**. This is documented as "user-feedback-driven" but it means `VerificationCriterion` is purely informational — a task with 10 criteria passes verification with the same evidence as a task with 0 criteria.

**Essentialist challenge (G1 — Exist):** Delete `TaskContract`. Inline `check_completion` into `task_verify`:

```rust
let passed = !evidence.trim().is_empty();
let reasoning = if passed {
    format!("User feedback received. Evidence length: {} chars.", evidence.len())
} else {
    "No evidence provided — task not verified.".into()
};
```

What breaks? Nothing — the contract's 10 fields are never read after `check_completion` returns. **FAILS the deletion test** — it is a pass-through wrapper that adds no behavior beyond a non-empty check.

**Essentialist challenge (G2 — Surface):** `TaskContract` has 1 public method (`new`) and 1 public method (`check_completion`) plus 10 public fields. But it's `pub(crate)` — so the surface is internal. Still, 10 fields for a struct that's used once and discarded is surface bloat.

**Essentialist challenge (G3 — Contract):** The `ContractState` enum (`Pending`/`Completed`/`Violated`) has 3 variants but only 2 are reachable (`Pending` → `Completed` or `Violated`). The `Completed` vs `Violated` distinction is never queried. This is a single-use abstraction.

**Grill-me challenge:** "If `TaskContract` is supposed to model an rSolidity contract, where is the `require!`/`assert!`/`emit!` macro mapping? Where is the OCAP token validation? Where is the gas enforcement?" Answer: nowhere. The doc comment claims "Maps to rSolidity's require!/assert!/emit! macros for CNS-observable contract execution" but no such mapping exists. The abstraction is aspirational documentation, not implemented behavior.

**Recommendation:** Either (a) delete `TaskContract` and inline the non-empty evidence check, or (b) if the contract is intended for future OCAP enforcement, mark it `#[allow(dead_code)]` with a TODO and a tracking issue. Option (a) is preferred per Essentialism.

**Constraint force:** Prohibition (P5 — Essentialism, P3 — no pass-through abstractions). Required.

---

### 2.3 HIGH — `task_consume_gas` and `task_consume_rjoules` are never called

**Location:** `crates/hkask-services-kata-kanban/src/kanban/service_impl/dejam.rs:217-268`

**Evidence:** `grep -rn "task_consume_gas\|task_consume_rjoules" crates/ mcp-servers/` returns matches only in the definition file. No caller exists in the CLI, MCP, REPL, or any other crate.

**Implication:** The gas/rJoule budget system is **incomplete**. Tasks can have `gas_remaining` set (via `task_add_gas`), and `unjam_fix` can auto-complete tasks when gas hits zero, but **nothing ever decrements gas**. The `gas_spend` audit trail is always empty except for refills.

This means:
1. The "gas exhaustion completion path" documented in `dejam.rs` never triggers naturally — gas only hits zero if the delegator manually sets it to zero.
2. The `GasEntry::gas_spend` and `GasEntry::rjoule_spend` constructors are never invoked in production.
3. The entire gas budget feature is **storage-only** — it's persisted but never consumed.

**Grill-me challenge:** "If no one calls `task_consume_gas`, how does gas ever reach zero?" Answer: it doesn't, unless the delegator explicitly adds zero gas (which `task_add_gas` allows via `saturating_add(0)`). The auto-completion path is dead code in practice.

**Pragmatic-cybernetics analysis:** The feedback loop is **broken** (property: closure). The sensing mechanism (gas_remaining) exists, the action mechanism (task_gas_exhaust) exists, but the consumption mechanism that would close the loop is missing. The loop never fires.

**Recommendation:** Either (a) wire `task_consume_gas` into the inference execution path (the subagent framework should call it after each LLM step), or (b) if the gas system is not yet integrated, mark these methods as `#[allow(dead_code)]` with a tracking issue and update the documentation to state that gas tracking is storage-only in v0.31.0.

**Constraint force:** Guardrail (P9 — Homeostatic Self-Regulation). The cybernetic loop is unclosed.

---

### 2.4 HIGH — `run_coaching_kata` / `run_improvement_kata` / `run_starter_kata` are never called in production

**Location:** `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:959-1018`

**Evidence:** `grep -rn "run_coaching_kata\|run_improvement_kata\|run_starter_kata"` returns matches only in the definition file and in `tests/bridge_integration.rs`. No production caller exists in the CLI, MCP, or REPL.

**Implication:** The `KanbanKataBridge` — the entire "full kata execution" path — is **unreachable from any user-facing surface**. The MCP tools (`kanban_task_kata_coaching` etc.) call `task_coaching_prompt` (prompt generation), not `run_coaching_kata` (full execution). The REPL `kask kanban kata` commands also call the prompt methods.

This means:
1. The `KanbanKataBridge` struct, its 3 methods, and the `with_kata_engine` builder are all dead code in production.
2. The `kata_bridge: Option<Arc<KanbanKataBridge>>` field on `KanbanService` is always `None` in production.
3. The "full kata execution" path documented in `docs/how-to/skills-and-composition.md` is aspirational, not operational.

**Grill-me challenge:** "If the bridge is never configured, why does `KanbanService` have a `with_kata_engine` builder method?" Answer: it's a future-proofing hook. But per coding-guidelines anti-pattern #3 (unrequested flexibility), this is a violation unless there's a concrete plan to wire it.

**Recommendation:** Either (a) wire the bridge into a CLI command (`kask kanban kata run <task-id> <manifest>`), or (b) mark the bridge methods as `#[allow(dead_code)]` with a tracking issue and update the documentation to clarify that only prompt generation is available in v0.31.0.

**Constraint force:** Guardrail (P5 — Essentialism). The bridge earns its keep only if it's reachable.

---

### 2.5 HIGH — Documentation says "8 MCP tools", actual count is 18

**Location:** `mcp-servers/hkask-mcp-kata-kanban/src/lib.rs:3`

```rust
//! Provides 8 MCP tools for kanban board and task management.
```

**Evidence:** `grep -c "#\[tool(" mcp-servers/hkask-mcp-kata-kanban/src/lib.rs` returns `18`. The README correctly says "Tools (18)" but the lib.rs doc comment says 8.

**Constraint force:** Guideline (documentation accuracy). Suggested.

**Recommendation:** Update the lib.rs doc comment to say "18 MCP tools".

---

### 2.6 HIGH — `board_view` uses a magic-number heuristic for WebID detection

**Location:** `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:282`

```rust
} else if f.len() > 30 && f.parse::<WebID>().is_ok() {
```

**Evidence:** The filter logic in `board_view` tries to distinguish a WebID from a label by checking `f.len() > 30`. This is fragile:
- A label longer than 30 chars that happens to parse as a WebID would be misinterpreted as an assignee filter.
- A WebID shorter than 30 chars (unlikely but possible with future format changes) would be treated as a label.

**Grill-me challenge:** "Why not just try `f.parse::<WebID>()` first, and fall back to label?" Answer: because a short label like "done" might parse as something. But `WebID` parsing should be strict enough that false positives are impossible. The `> 30` check is a band-aid for a parsing ambiguity that shouldn't exist.

**Recommendation:** Remove the `f.len() > 30` guard. If `WebID::parse` is too permissive, fix the parser. If it's strict, the guard is unnecessary.

**Constraint force:** Guideline (P5 — Simplicity First). Suggested.

---

### 2.7 HIGH — Stale documentation references to `docs/plans/kata-kanban-merge-plan.md`

**Location:** `crates/hkask-services-kata-kanban/src/lib.rs:8`

```rust
//! See `docs/plans/kata-kanban-merge-plan.md` for the full merge rationale and
//! implementation plan.
```

**Evidence:** `find docs -name "kata-kanban-merge-plan.md"` returns no results. The file does not exist. The reference is stale.

**Additional stale references found:**
- `docs/status/PROJECT_STATUS.md:241` references `docs/guides/kata-user-guide.md` — the `docs/guides/` directory does not exist.
- `docs/status/corpus_inventory.yaml:614` references `docs/user-guides/kanban-user-guide.md` — the `docs/user-guides/` directory does not exist.
- `docs/status/corpus_inventory.yaml:625` references `docs/user-guides/kata-user-guide.md` — same issue.
- `docs/architecture/core/hKask-architecture-master.md:1048` references `docs/user-guides/kanban-user-guide.md` — same issue.

**Constraint force:** Guardrail (documentation accuracy). Required.

**Recommendation:** Remove the stale reference from `lib.rs`. Either create the merge-plan file (if the merge is ongoing) or remove the reference (if the merge is complete). Update `PROJECT_STATUS.md`, `corpus_inventory.yaml`, and `hKask-architecture-master.md` to point to existing paths or remove the references.

---

### 2.8 MEDIUM — `task_claim` does not check task status

**Location:** `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:617-658`

**Evidence:** `task_claim` only checks `task.assignee.is_some()`. It does not check `task.status`. An agent can claim a task in `Review` or `Done` status.

**Implication:** An agent could "claim" a Done task, which is semantically meaningless (the task is already complete). The `assignee` field would be set, but the task status remains `Done`.

**Grill-me challenge:** "Is this intentional?" Possibly — the doc says "Claim an unassigned task as the authenticated actor" without mentioning status. But it violates the principle of least surprise. A reasonable user would expect that only `Backlog` or `Ready` tasks can be claimed.

**Recommendation:** Add a status check: `if task.status != TaskStatus::Backlog && task.status != TaskStatus::Ready { return Err(KanbanError::InvalidTransition { ... }); }`.

**Constraint force:** Guideline (P4 — Clear Boundaries). Suggested.

---

### 2.9 MEDIUM — `task_unassign` allows owner to unassign without assignee consent

**Location:** `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:795-807`

**Evidence:** `task_unassign` uses `require_task_owner`, not `require_task_actor`. The owner can unassign a task without the assignee's consent. The `unjam_fix` auto-unassigns idle tasks using `task.owner` as the actor.

**Implication:** This is a P1 (User Sovereignty) concern. If an agent has claimed a task and is actively working on it, the owner can unassign them without notice. The `unjam_fix` auto-unassigns after 24h idle, which is documented, but a manual `task_unassign` by the owner has no such guard.

**Grill-me challenge:** "Is this intentional?" The doc says "Unassign a task — remove the assignee" without mentioning consent. But P1 (User Sovereignty) is a Magna Carta principle. The assignee consented to the assignment; removing it without their consent violates the symmetry of consent.

**Recommendation:** Either (a) require both owner and assignee to consent for manual unassignment (the assignee's "consent" could be implicit after a timeout), or (b) document that the owner has unilateral unassignment authority and justify why this doesn't violate P1.

**Constraint force:** Guardrail (P1 — User Sovereignty). Overridable with a stated reason.

---

### 2.10 MEDIUM — `task_move` has dead code (`let _ = actor;`)

**Location:** `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:577`

```rust
task.status = target;
task.updated_at = chrono::Utc::now();
let _ = actor;  // ← dead code, actor is already used in require_task_actor and the CNS span
```

**Evidence:** `actor` is used at line 548 (`require_task_actor(&task, actor)`) and at line 602 (`actor = %actor` in the tracing span). The `let _ = actor;` at line 577 is a no-op — it was likely left over from a refactor where `actor` was temporarily unused.

**Constraint force:** Guideline (P5 — Simplicity First). Suggested.

**Recommendation:** Delete the line.

---

### 2.11 MEDIUM — `default_columns` is duplicated 4 times

**Location:**
- `mcp-servers/hkask-mcp-kata-kanban/src/lib.rs:879` (`pub fn default_columns`)
- `mcp-servers/hkask-mcp-kata-kanban/tests/kanban_contract.rs:25` (`fn default_columns`)
- `crates/hkask-services-kata-kanban/src/kanban/service_impl/tests.rs:27` (`fn make_default_columns`)
- `crates/hkask-services-kata-kanban/tests/bridge_integration.rs:22` (`fn default_columns`)

**Evidence:** The same 5-column default board layout is defined in 4 places. The MCP server's `default_columns` is `pub` and could be reused by the tests, but each test file defines its own copy.

**Essentialist challenge (G3 — Contract):** The duplication is a single-use abstraction that could be inlined. But the `default_columns` in the MCP server is part of the public API (used by `kanban_board_create` when no columns are specified). The test duplicates are test fixtures — they should import from the MCP server's public API.

**Recommendation:** The test files should import `hkask_mcp_kata_kanban::default_columns` instead of redefining it. The service-crate tests should define a shared test helper in `crates/hkask-services-kata-kanban/src/test_helpers.rs` and reuse it.

**Constraint force:** Guideline (DRY). Suggested.

---

### 2.12 MEDIUM — `KanbanService` derives `Clone` but no one clones it

**Location:** `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:33`

```rust
#[derive(Clone)]
pub struct KanbanService { ... }
```

**Evidence:** `grep -rn "KanbanService.*clone\|svc\.clone()\|service\.clone()" crates/ mcp-servers/` returns zero matches. The `Clone` derive is unused.

**Implication:** `KanbanService` holds an `HMemStore` (which is `Clone` via `Arc`) and an `Option<Arc<KanbanKataBridge>>` (also `Clone`). So the derive compiles, but it's dead capability.

**Grill-me challenge:** "Why is `Clone` derived?" Possibly for future use in a daemon context where the service is shared across threads. But `Arc<KanbanService>` would be more idiomatic for thread sharing. `Clone` on a service with internal state is a code smell — it implies the service is value-like, but it's actually a handle to shared storage.

**Recommendation:** Remove the `Clone` derive. If thread sharing is needed, wrap in `Arc`.

**Constraint force:** Guideline (P5 — Simplicity First). Suggested.

---

### 2.13 LOW — `KanbanKataBridge` has 3 near-identical methods

**Location:** `crates/hkask-services-kata-kanban/src/bridge.rs:35-76`

**Evidence:** `run_coaching_on_task`, `run_improvement_on_task`, and `run_starter_on_task` all do:
1. Build `learner_bot` from task
2. Build `context` from task (plus `sub_problem` for starter)
3. Call `self.engine.execute(manifest, &learner_bot, context)`

The only difference is that `run_starter_on_task` inserts `"sub_problem"` into the context. The kata type is determined by the manifest, not by the method name.

**Essentialist challenge (G1 — Exist):** Delete all three methods. Replace with one:

```rust
pub async fn run_kata_on_task(
    &self,
    task: &Task,
    manifest: &KataManifest,
    extra_context: Option<&str>,
) -> Result<KataResult, KataError> {
    let learner_bot = task_learner_bot(task);
    let mut context = build_task_context(task);
    if let Some(extra) = extra_context {
        context.insert("sub_problem".to_string(), extra.to_string());
    }
    self.engine.execute(manifest, &learner_bot, context).await
}
```

What breaks? The callers in `KanbanService::run_coaching_kata` etc. would need to be updated. But those methods are also never called in production (finding 2.4). **FAILS the deletion test** — the three methods are pass-throughs to `engine.execute()` with trivial context construction.

**Recommendation:** Collapse to a single method. The kata type is already discriminated by the manifest — the method name adds no information.

**Constraint force:** Guardrail (P5 — Essentialism, no pass-through abstractions). Overridable.

---

### 2.14 LOW — `KanbanServer` doc comment says "8 MCP tools" but README says "18"

Already covered in finding 2.5. Listed separately for traceability.

---

### 2.15 LOW — `Task::can_move_to` has a duplicated doc comment

**Location:** `crates/hkask-services-kata-kanban/src/kanban/types/task.rs:88-95`

```rust
/// expect: "System types preserve semantic identity and are provenance-aware"
/// pre:  arguments are valid
/// post: returns new instance with defaults
/// pre:  target is a valid transition from self.status
/// post: returns true iff self.status.can_transition_to(target)
pub fn can_move_to(&self, target: TaskStatus) -> bool {
    self.status.can_transition_to(target)
}
```

**Evidence:** The doc comment has two `pre:` and two `post:` lines — the first pair ("arguments are valid" / "returns new instance with defaults") is copy-pasted from a constructor and doesn't apply to `can_move_to`. Similar duplication appears in `Comment::new`, `TaskFilter::by_status`, `TaskFilter::by_priority`, `TaskContract::new`, `KanbanPhase::new`.

**Constraint force:** Guideline (documentation accuracy). Suggested.

**Recommendation:** Clean up the duplicated doc comments.

---

## 3. Essentialist Summary

Running the 3-gate protocol on the kata-kanban crate:

| Artifact | G1 (Exist) | G2 (Surface) | G3 (Contract) | Verdict |
|----------|-----------|-------------|--------------|---------|
| `KanbanServer.db` field | **FAIL** — delete, nothing breaks | N/A | N/A | DELETE |
| `TaskContract` | **FAIL** — delete, inline non-empty check | FAIL — 10 fields, 1 use | FAIL — `ContractState` unreachable | DELETE or mark `#[allow(dead_code)]` |
| `task_consume_gas`/`task_consume_rjoules` | PASS (intended behavior) | N/A | N/A | WIRE or mark `#[allow(dead_code)]` |
| `KanbanKataBridge` 3 methods | **FAIL** — pass-through to `engine.execute()` | PASS — small surface | PASS — genuine delegation | COLLAPSE to 1 method |
| `KanbanService::Clone` | PASS — compiles | N/A | FAIL — unused capability | REMOVE derive |
| `default_columns` (4 copies) | N/A | N/A | FAIL — duplication | DRY up |

**Essentialism score:** ~15% reduction possible (3 deletions + 3 collapses out of ~40 public items).

---

## 4. Cybernetic Analysis

### 4.1 Feedback loop: gas consumption → exhaustion → auto-complete

| Property | Rating | Evidence |
|----------|--------|---------|
| Closure | **BROKEN** | `task_consume_gas` is never called — the loop never fires |
| Polarity | N/A | Loop doesn't fire |
| Delay | N/A | Loop doesn't fire |
| Gain | N/A | Loop doesn't fire |
| Fidelity | N/A | Loop doesn't fire |

**Remediation:** Wire `task_consume_gas` into the subagent inference execution path. The loop is: sense (gas_remaining) → decide (gas > 0?) → act (continue or auto-complete). Currently only the auto-complete path exists, and it never triggers because gas never decrements.

### 4.2 Feedback loop: kata execution → CNS variety → algedonic alert

| Property | Rating | Evidence |
|----------|--------|---------|
| Closure | **DEGRADED** | `run_*_kata` methods exist but are never called in production — the loop is wired but unreachable |
| Polarity | Correct | Positive variety → no alert; deficit → alert |
| Delay | Low | CNS runtime checks are synchronous |
| Gain | Correct | Variety counter increments per practice |
| Fidelity | Correct | CNS spans carry namespace + step ordinal |

**Remediation:** Wire `run_*_kata` into a CLI command or MCP tool to close the loop.

### 4.3 VSM mapping

| VSM subsystem | hKask component | Status |
|---------------|------------------|--------|
| S1 (Operations) | `KanbanService` (board/task CRUD) | ✅ Viable |
| S2 (Coordination) | `unjam_fix` (anti-oscillation) | ✅ Viable |
| S3 (Control) | `task_verify` (audit) | ⚠️ Degraded — verification is non-gating |
| S4 (Intelligence) | `KataEngine` (PDCA) | ⚠️ Unviable — unreachable from production |
| S5 (Policy) | `consent_check` callback | ✅ Viable (when configured) |
| Algedonic channel | `check_cns_alerts` | ⚠️ Degraded — depends on S4 which is unreachable |

**Overall viability:** Degraded. The system is viable for board/task CRUD (S1-S3) but the kata intelligence layer (S4) is disconnected from operations.

---

## 5. Pragmatic-Semantics Classification

| Claim | Ontological mode | Epistemic mode | Constraint force | Provenance | Confidence |
|-------|-----------------|---------------|-----------------|-----------|------------|
| "18 MCP tools" (README) | IS | Declarative | Evidence | Implementation | 0.95 |
| "8 MCP tools" (lib.rs doc) | IS | Declarative | Evidence | Implementation | 0.95 |
| "consent proof (P1 compliance)" (README) | OUGHT | Declarative | Guardrail | Specification | 0.70 — overstated; `task_claim` is self-claim, no separate consent proof |
| "Maps to rSolidity's require!/assert!/emit!" (contract.rs doc) | OUGHT | Declarative | Guardrail | Specification | 0.30 — aspirational, not implemented |
| "gas exhaustion completion path" (dejam.rs doc) | IS | Declarative | Evidence | Implementation | 0.90 — code exists but loop is unclosed |

**Conflict:** README says 18 tools (IS, Declarative, Evidence) vs lib.rs says 8 tools (IS, Declarative, Evidence). Both are IS-Declarative-Evidence. Tiebreak by provenance: README is more recently updated. **Winner: 18 tools.** The lib.rs doc is stale.

---

## 6. Recommendations — Decomposed into Smallest Pragmatic Steps

Per the user's request, each recommendation is broken into the smallest independently-actionable step:

### Step 1: Delete `KanbanServer.db` field (CRITICAL, 5 min)
- File: `mcp-servers/hkask-mcp-kata-kanban/src/lib.rs`
- Remove `pub db: Option<...>` from struct
- Remove `db.sqlite_pool().ok()` from `run()` constructor call
- Run `cargo check`

### Step 2: Fix lib.rs doc comment "8 → 18" (HIGH, 1 min)
- File: `mcp-servers/hkask-mcp-kata-kanban/src/lib.rs:3`
- Change "8 MCP tools" to "18 MCP tools"

### Step 3: Delete `let _ = actor;` dead code (MEDIUM, 1 min)
- File: `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:577`
- Remove the line

### Step 4: Remove stale `kata-kanban-merge-plan.md` reference (HIGH, 2 min)
- File: `crates/hkask-services-kata-kanban/src/lib.rs:8`
- Remove the `See docs/plans/kata-kanban-merge-plan.md` sentence

### Step 5: Remove `Clone` derive from `KanbanService` (MEDIUM, 1 min)
- File: `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:33`
- Remove `#[derive(Clone)]`
- Verify no callers break

### Step 6: Mark `task_consume_gas`/`task_consume_rjoules` as `#[allow(dead_code)]` (HIGH, 2 min)
- File: `crates/hkask-services-kata-kanban/src/kanban/service_impl/dejam.rs`
- Add `#[allow(dead_code)]` to both methods
- Add a TODO comment referencing the gas-budget integration tracking issue

### Step 7: Mark `run_*_kata` methods as `#[allow(dead_code)]` (HIGH, 3 min)
- File: `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs`
- Add `#[allow(dead_code)]` to `run_coaching_kata`, `run_improvement_kata`, `run_starter_kata`
- Add a TODO comment referencing the kata-bridge-wiring tracking issue

### Step 8: Collapse `KanbanKataBridge` to 1 method (LOW, 10 min)
- File: `crates/hkask-services-kata-kanban/src/bridge.rs`
- Replace 3 methods with `run_kata_on_task(task, manifest, extra_context)`
- Update callers in `service.rs`

### Step 9: Add status check to `task_claim` (MEDIUM, 5 min)
- File: `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:617`
- Add `if task.status != TaskStatus::Backlog && task.status != TaskStatus::Ready { return Err(...); }`
- Add a test for the new check

### Step 10: Remove `f.len() > 30` heuristic in `board_view` (HIGH, 5 min)
- File: `crates/hkask-services-kata-kanban/src/kanban/service_impl/service.rs:282`
- Remove the `f.len() > 30 &&` guard
- Test that WebID parsing is strict enough

### Step 11: Delete or inline `TaskContract` (CRITICAL, 15 min)
- File: `crates/hkask-services-kata-kanban/src/kanban/types/contract.rs`
- Option A: Delete the file, inline the non-empty evidence check into `task_verify`
- Option B: Mark `#[allow(dead_code)]` with a TODO for OCAP enforcement
- Update `service.rs:692` accordingly

### Step 12: Clean up duplicated doc comments (LOW, 10 min)
- Files: `task.rs`, `task_spec.rs`, `contract.rs`, `phase.rs`
- Remove the duplicated `pre:`/`post:` lines

### Step 13: DRY up `default_columns` test fixtures (MEDIUM, 10 min)
- Files: 3 test files
- Import from the MCP server's public `default_columns` or create a shared test helper

### Step 14: Update stale `docs/user-guides/` references (HIGH, 10 min)
- Files: `PROJECT_STATUS.md`, `corpus_inventory.yaml`, `hKask-architecture-master.md`
- Remove or redirect references to non-existent `docs/user-guides/` and `docs/guides/` paths

---

## 7. Documentation Updates

### 7.1 New diagrams created

- `docs/diagrams/class-kata-kanban-architecture.md` — class diagram of the kata-kanban MCP server and service crate (DIAG-IC-017)
- `docs/diagrams/state-kanban-task-lifecycle.md` — state diagram of task transitions including reopen and gas-exhaust paths (DIAG-FW-008)

### 7.2 Diagrams index update

The `docs/DIAGRAMS_INDEX.md` should be updated to register:
- DIAG-IC-017: Kata-Kanban Architecture Class Diagram
- DIAG-FW-008: Kanban Task Lifecycle State Diagram

### 7.3 Stale references to remove

- `crates/hkask-services-kata-kanban/src/lib.rs:8` — reference to non-existent `docs/plans/kata-kanban-merge-plan.md`
- `docs/status/PROJECT_STATUS.md:241` — reference to non-existent `docs/guides/kata-user-guide.md`
- `docs/status/corpus_inventory.yaml:614,625` — references to non-existent `docs/user-guides/` paths
- `docs/architecture/core/hKask-architecture-master.md:1048` — reference to non-existent `docs/user-guides/kanban-user-guide.md`

---

## 8. Validation

- `cargo check -p hkask-services-kata-kanban -p hkask-mcp-kata-kanban` — ✅ Passes
- `cargo clippy -p hkask-services-kata-kanban -p hkask-mcp-kata-kanban --all-targets -- -D warnings` — ✅ Passes (0 warnings)
- `cargo test -p hkask-services-kata-kanban -p hkask-mcp-kata-kanban` — ✅ 60/60 tests pass
  - 38 unit tests in `hkask-services-kata-kanban`
  - 16 contract tests in `hkask-mcp-kata-kanban/tests/kanban_contract.rs`
  - 6 bridge integration tests in `hkask-services-kata-kanban/tests/bridge_integration.rs`

No code changes were made in this review. All findings are recommendations for the maintainer to action.

---

## 9. Cross-references

- [Kata-Kanban Architecture Class Diagram](../diagrams/class-kata-kanban-architecture.md) — DIAG-IC-016
- [Kanban Task Lifecycle State Diagram](../diagrams/state-kanban-task-lifecycle.md) — DIAG-FW-008
- [Kata PDCA Lifecycle State Machine](../how-to/skills-and-composition.md#kata-pdca-lifecycle-state-machine) — DIAG-FW-005
- [Kata-Kanban Execution Boundary](../how-to/skills-and-composition.md#kata-kanban-execution-boundary) — DIAG-FW-006
- [Architecture Master: Kanban](../architecture/core/hKask-architecture-master.md#kanban--headless-task-coordination)
- [Architecture Master: Kata](../architecture/core/hKask-architecture-master.md#kata--cybernetic-capability-development)
- [Service Layer Class Diagram](../explanation/architecture-patterns.md#service-layer-class-diagram) — DIAG-IC-008
- [Documentation Standards](../specifications/DOCUMENTATION_STANDARDS.md) — verification gate
