# Architecture Improvement Plan — `hkask-services-core` Scope Contraction

**Date:** 2026-07-17  
**Skills applied:** task-breakdown, strangler-fig, improve-codebase-architecture, essentialist, pragmatic-semantics, coding-guidelines  
**Supersedes (for this scope):** the core-scope-contraction recommendation from the 2026-07-17 service-crate review (Recommendation #2).  
**Preserves:** the in-progress `tasks/plan.md` (corpus-ingest elimination) is untouched; this plan is scoped to `hkask-services-core` only.

---

## Overview

`hkask-services-core` claims to be the service-layer foundation (`ServiceError`, `ServiceConfig`, `HkaskSettings`), but it holds 90 public items across 9 modules — including `self_heal`, `model_cache`, `verification`, `goal`, `identity`, and `inference_svc`. A crate needing only `ServiceError` links a self-healing engine, a model cache, an identity subsystem, and a verification service. The essentialist G2 surface gate fails (90 > 7 by 12×).

**Goal:** Contract `hkask-services-core` to its named foundation (`error` + `config` + `settings`, ~15 pub items) by extracting the six non-foundation modules into sibling crates, one domain at a time, keeping the system fully functional at every step (strangler-fig).

**Target condition (measurable):**
- `hkask-services-core` public surface ≤ 15 items.
- `cargo check --workspace` + `cargo test --workspace` green after every slice.
- `core`'s `Cargo.toml` description matches its contents.
- Each extracted module has exactly one home crate.

**Out of scope:** `hkask-services-research`'s independent `WebError` (settled by ADR-054). Re-splitting `hkask-services-kata-kanban` (its merge ADR is load-bearing).

## Architecture Decisions

1. **Strangler-fig, one module per vertical slice.** Each slice = CREATE new crate → WIRE all consumers → DELETE from core. A half-extracted module leaves the system in a worse state, so a slice is the full extraction of one module, not a horizontal layer.
2. **Risk-ordered sequencing** (strangler-fig: PoC → Independent → Dependent → Cross-cutting):
   - Phase 1: `self_heal`, `verification` (1 consumer each — prove the pattern).
   - Phase 2: `goal`, `identity` (storage-coupled — 3-4 consumers each).
   - Phase 3: `inference_svc` + `model_cache` (8+ consumers — cross-cutting, moved as a pair).
3. **Re-export bridge during WIRE.** While migrating, `core` may temporarily `pub use new_crate::X` so uncritical `use hkask_services_core::X` sites keep compiling; the re-export is removed in the DELETE sub-step. This keeps each WIRE sub-step small and reversible.
4. **`GoalState` stays in `hkask-types`.** Only `Goal`/`GoalCriterion`/`GoalArtifact`/`IllegalGoalTransition` move; `GoalState` is already in `hkask-types` (orphan rule — SQL impls live there).
5. **No new public traits.** Each extracted crate re-homes existing types; the extraction introduces zero new abstractions (P1 compliance).

## Phased Task List

### Phase 1 — PoC: prove the strangler-fig pattern (fail fast)

**Task 1.1: Extract `self_heal` → `hkask-services-self-heal`**
- **Slice:** `extract-self-heal`
- **Files:** new `crates/hkask-services-self-heal/` (Cargo.toml + `src/lib.rs` moving `crates/hkask-services-core/src/self_heal/*`); edit `crates/hkask-services-core/src/lib.rs` (drop `pub mod self_heal` + re-export); edit `crates/hkask-services-context/src/context_impl/build/reg_wallet.rs` (the 1 consumer: `use hkask_services_core::self_heal::…` → `use hkask_services_self_heal::…`); edit workspace `Cargo.toml` (add member).
- **CREATE:** move `self_heal/{healer,types,strategies,tests}.rs` into the new crate; the new crate depends on `hkask-types`, `hkask-inference` (for `HealInferenceFn`), `tracing`, `serde`, `thiserror`, `tokio`.
- **WIRE:** update `reg_wallet.rs` import; add `hkask-services-self-heal` dep to `hkask-services-context/Cargo.toml`.
- **DELETE:** remove `pub mod self_heal;` from core's `lib.rs`; remove `self_heal/` dir from core; drop `hkask-inference` from core's deps if now unused there.
- **Acceptance:** (a) `cargo test -p hkask-services-self-heal` passes (moved tests); (b) `cargo check -p hkask-services-context` passes; (c) `rg 'pub mod self_heal' crates/hkask-services-core/src/` returns nothing.
- **Verification:** `cargo test -p hkask-services-self-heal -p hkask-services-context --lib`
- **Dependencies:** None
- **Scope:** S

**Task 1.2: Extract `verification` → `hkask-services-verification`**
- **Slice:** `extract-verification`
- **Files:** new `crates/hkask-services-verification/`; edit `crates/hkask-services-core/src/lib.rs`; edit `crates/hkask-cli/src/commands/sovereignty.rs` (the 1 consumer).
- **CREATE:** move `crates/hkask-services-core/src/verification/*` into the new crate; depends on `hkask-types`, `serde`, `serde_json`, `tracing`.
- **WIRE:** update `sovereignty.rs` (`hkask_services_core::verification::VerificationService` → `hkask_services_verification::VerificationService`); add dep to `hkask-cli/Cargo.toml`.
- **DELETE:** remove `pub mod verification;` from core; remove `verification/` dir.
- **Acceptance:** (a) `cargo test -p hkask-services-verification` passes; (b) `cargo check -p hkask-cli` passes; (c) `rg 'pub mod verification' crates/hkask-services-core/src/` returns nothing.
- **Verification:** `cargo test -p hkask-services-verification -p hkask-cli --lib`
- **Dependencies:** None (independent of 1.1)
- **Scope:** XS

**Checkpoint 1:** Pattern proven. `core` shed 2 modules. `cargo check --workspace && cargo test --workspace` green. `core` pub-item count dropped (~15 items).

### Phase 2 — Dependent: storage-coupled modules

**Task 2.1: Extract `goal` → `hkask-goal`**
- **Slice:** `extract-goal`
- **Files:** new `crates/hkask-goal/`; edit `crates/hkask-services-core/src/lib.rs`; edit consumers: `crates/hkask-storage/src/goals.rs`, `crates/hkask-api/src/routes/goal.rs`, `crates/hkask-cli/src/commands/goal.rs`.
- **CREATE:** move `Goal`, `GoalCriterion`, `GoalArtifact`, `IllegalGoalTransition` from `crates/hkask-services-core/src/goal.rs` into `hkask-goal`; re-export `GoalState` from `hkask-types` (do NOT move it — orphan rule). Depends on `hkask-types`, `chrono`, `serde`, `uuid`.
- **WIRE:** update `hkask-storage/goals.rs` (`use hkask_services_core::{Goal, GoalArtifact, GoalCriterion}` → `use hkask_goal::{…}`; keep `GoalState` from `hkask_types`); update `hkask-api` + `hkask-cli` goal routes; add `hkask-goal` dep to each consumer's Cargo.toml. Add `hkask-goal` dep to `hkask-services-core` only if a re-export bridge is needed during migration (prefer direct consumer wiring to avoid re-introducing the coupling).
- **DELETE:** remove `pub mod goal;` + `pub use goal::{…}` from core; remove `goal.rs`.
- **Acceptance:** (a) `cargo test -p hkask-goal` passes; (b) `cargo test -p hkask-storage -p hkask-api -p hkask-cli --lib` passes; (c) `rg 'pub mod goal' crates/hkask-services-core/src/` returns nothing.
- **Verification:** `cargo test -p hkask-goal -p hkask-storage --lib`
- **Dependencies:** Checkpoint 1 (pattern proven; avoids concurrent Cargo.toml churn)
- **Scope:** S

**Task 2.2: Extract `identity` → `hkask-identity`**
- **Slice:** `extract-identity`
- **Files:** new `crates/hkask-identity/`; edit `crates/hkask-services-core/src/lib.rs`; edit consumers: `crates/hkask-cli/src/commands/user.rs`, `crates/hkask-services-context/src/storage.rs`, `crates/hkask-api/src/routes/auth.rs`, `crates/hkask-storage/src/user_store.rs`.
- **CREATE:** move `HumanUser`, `ReplicantIdentity`, `UserSession`, `RegistrationRequest`, `Invite`, `InviteStatus`, `OAuthProvider`, `RegistrationError`, `Role` from `crates/hkask-services-core/src/identity.rs` into `hkask-identity`. Depends on `hkask-types`, `hkask-wallet-types` (for `WalletId` on `ReplicantIdentity`), `chrono`, `serde`.
- **WIRE:** update the 4 consumers' imports; add `hkask-identity` dep to each. `hkask-services-context/storage.rs` uses `ReplicantIdentity` + `ServiceError` — after wiring it imports `ReplicantIdentity` from `hkask-identity` and `ServiceError` stays from `core`.
- **DELETE:** remove `pub mod identity;` + `pub use identity::{…}` from core; remove `identity.rs`; drop `hkask-wallet-types` from core's deps if now unused there.
- **Acceptance:** (a) `cargo test -p hkask-identity` passes; (b) `cargo check -p hkask-cli -p hkask-services-context -p hkask-api -p hkask-storage` passes; (c) `rg 'pub mod identity' crates/hkask-services-core/src/` returns nothing.
- **Verification:** `cargo test -p hkask-identity --lib && cargo check -p hkask-services-context`
- **Dependencies:** Task 2.1 (sequential Cargo.toml edits in storage avoid merge conflicts)
- **Scope:** S

**Checkpoint 2:** Storage-coupled modules extracted. `cargo check --workspace && cargo test --workspace` green. `core` pub-item count ~30 (down from 90).

### Phase 3 — Cross-cutting: inference services (many consumers)

**Task 3.1: Extract `inference_svc` + `model_cache` → `hkask-services-inference`**
- **Slice:** `extract-inference-services`
- **Files:** new `crates/hkask-services-inference/` (moving `inference_svc.rs` + `model_cache.rs` together — they are coupled: `InferenceService::list_models` delegates to `ModelCache`); edit `crates/hkask-services-core/src/lib.rs`; edit 8+ consumers: `hkask-api` (models.rs, bundles.rs), `hkask-cli` (models.rs, bundle.rs, skill.rs, chat.rs), `hkask-repl` (handlers/model.rs), `hkask-services-chat` (chat/service.rs), `hkask-services-compose` (lib.rs), `hkask-services-context` (build — `From<&AgentService> for InferenceContext`).
- **CREATE:** move `inference_svc.rs` + `model_cache.rs` (incl. the poison-recovery `lock_cache` + regression test from the 2026-07-17 fix) into `hkask-services-inference`. Depends on `hkask-inference` (`InferenceRouter`, `InferenceConfig`), `hkask-types`, `hkask-ports` (`InferencePort`), `hkask-services-core` (`ServiceError` — the one remaining foundation dep). Re-export `InferenceContext`, `InferenceService`, `ModelInfo`, `ModelCache`.
- **WIRE:** update all 8+ consumers `hkask_services_core::{InferenceContext, InferenceService, ModelCache}` → `hkask_services_inference::{…}`; add `hkask-services-inference` dep to each consumer's Cargo.toml. The `From<&AgentService> for InferenceContext` impl in `hkask-services-context` stays in context (context depends on the new crate for `InferenceContext`).
- **DELETE:** remove `pub mod inference_svc; pub mod model_cache;` + re-exports from core; remove the two files; drop `hkask-inference` from core's deps.
- **Acceptance:** (a) `cargo test -p hkask-services-inference` passes (incl. the poison-recovery test); (b) `cargo check -p hkask-api -p hkask-cli -p hkask-repl -p hkask-services-chat -p hkask-services-compose -p hkask-services-context` passes; (c) `rg 'pub mod (inference_svc|model_cache)' crates/hkask-services-core/src/` returns nothing.
- **Verification:** `cargo test -p hkask-services-inference --lib && cargo check --workspace`
- **Dependencies:** Checkpoint 2
- **Scope:** M (split into 3.1a CREATE, 3.1b WIRE, 3.1c DELETE if it exceeds one session — see Open Question Q2)

**Task 3.2 (optional sub-split of 3.1):** If 3.1 is too large for one session, split into:
- 3.1a CREATE `hkask-services-inference` with both modules + tests; `cargo test -p hkask-services-inference`.
- 3.1b WIRE the 4 inference-CLI/repl/api consumers (models listing); then WIRE the 3 service consumers (chat/compose/context).
- 3.1c DELETE from core.
Each sub-step runs `cargo check --workspace`.

**Checkpoint 3:** `hkask-services-core` now contains only `error` + `config` + `settings` (+ the `goal`/`identity` re-exports removed). `cargo check --workspace && cargo test --workspace` green. `core` pub-item count ≤ 15.

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Re-export bridge re-introduces coupling (core keeps depending on extracted crates) | High | Prefer direct consumer wiring over re-exports; only use a re-export transiently within one slice, removed in that slice's DELETE step |
| `hkask-storage` breaks when `goal`/`identity` move (it imports both from core) | Medium | Wire storage first in each WIRE step; run `cargo test -p hkask-storage` before touching api/cli |
| `inference_svc` extraction touches 8+ crates — large blast radius | High | Phase 3 last; sub-split 3.1 into CREATE/WIRE/DELETE; workspace check after each sub-step |
| `From<&AgentService> for InferenceContext` lives in `hkask-services-context` and references `InferenceContext` | Medium | Context gains a dep on `hkask-services-inference`; the impl stays in context (it's about `AgentService`, not inference) |
| `GoalState` orphan-rule confusion (lives in `hkask-types`, not core) | Low | Explicitly documented in Decision #4; only the core-local goal types move |
| Docs verifier (`docs/ci/verify-docs.sh`) flags stale crate refs during migration | Low | Avoid bare `hkask-services` tokens in any new docs/ADRs (regex extracts `hkask-services` from `hkask-services-*` globs); reference full crate names |

## Open Questions

| # | Question | Recommendation |
|---|----------|----------------|
| Q1 | Should `inference_svc` + `model_cache` fold into the existing `hkask-inference` crate instead of a new `hkask-services-inference`? | Prefer a new `hkask-services-inference`. `hkask-inference` is a port/adapter (router, backends); the service layer (`InferenceService`, `ModelCache`, TTL policy) is a different ontology tier. Mixing them violates the hexagonal port/adapter separation. Decide before Task 3.1 CREATE. |
| Q2 | Is Task 3.1 one slice or three (3.1a/b/c)? | Start as one; split only if a single session can't hold the 8+ consumer edits. The CREATE+tests sub-step is independently verifiable, so 3.1a can land first. |
| Q3 | Does `hkask-services-core` keep a `pub use` re-export of the extracted types for the entire migration, or drop per-slice? | Drop per-slice (Decision #3). A long-lived re-export defeats the contraction goal and keeps core's compile fan-in high. |
| Q4 | Should `self_heal`'s `SetEnv` action (the cybernetic smell flagged in the review) be fixed as part of the extraction or separately? | Separately. Extraction is a move, not a behavior change. The `SetEnv` → local-config reroute is its own diagnose-PDCA (Recommendation #4 of the review), tracked independently. |