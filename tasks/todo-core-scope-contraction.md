# TODO — `hkask-services-core` Scope Contraction

> Companion to `tasks/plan-core-scope-contraction.md`. The in-progress `tasks/todo.md`
> (corpus-ingest) is unchanged.

## Phase 1 — PoC: prove the strangler-fig pattern

- [x] **1.1** Extract `self_heal` → `hkask-services-self-heal` ✅ (verified 2026-07-17)
  - [x] CREATE: move `self_heal/{healer,types,strategies,tests}.rs` into new crate
  - [x] WIRE: update `hkask-services-context/build/reg_wallet.rs` import
  - [x] WIRE: update `hkask-test-harness/src/self_heal.rs` re-export (2nd consumer found during execution)
  - [x] DELETE: remove `pub mod self_heal` from core; drop `minijinja` + `dotenvy` from core deps (`dirs` kept — used by `settings.rs`)
  - [x] `cargo test -p hkask-services-self-heal` passes (13 tests)
  - [x] `cargo check -p hkask-services-context` passes
  - [x] `cargo clippy -p hkask-services-core -p hkask-services-self-heal -- -D warnings` clean
  - [x] added `README.md` (matches 11 sibling service crates)
  - [x] `rg 'pub mod self_heal' crates/hkask-services-core/src/` empty

- [x] **1.2** Extract `verification` → `hkask-services-verification` ✅ (verified 2026-07-17)
  - [x] CREATE: copy `verification.rs` (single file) into new crate as `lib.rs`
  - [x] WIRE: update `hkask-cli/commands/sovereignty.rs` (2 refs: `verify` + `verify_json`) + add dep
  - [x] DELETE: remove `pub mod verification` from core; drop `serde_yaml_neo` from core deps (only `verification.rs` used it; `serde`/`serde_json` kept)
  - [x] `cargo test -p hkask-services-verification` passes (4 tests; manifest path resolution relocation-safe via `CARGO_MANIFEST_DIR/../../`)
  - [x] `cargo check -p hkask-cli` passes
  - [x] added `README.md` (matches sibling convention)

**Checkpoint 1:** ✅ pattern proven (2 PoC slices done). `cargo check --workspace && cargo test` green. core pub-items 90 → 76.

## Phase 2 — Dependent: storage-coupled modules

- [x] **2.1** Extract `goal` → `hkask-goal` ✅ (verified 2026-07-17)
  - [x] CREATE: move `Goal`/`GoalCriterion`/`GoalArtifact`/`IllegalGoalTransition` (NOT `GoalState` — stays in `hkask-types`)
  - [x] WIRE 5 consumers (rigorous scan found 5, plan said 3): `hkask-storage/goals.rs`, `hkask-test-harness/strategies.rs`, `hkask-api/routes/goal.rs`, `hkask-cli/commands/goal.rs`, `hkask-types/tests/contract/types_contract.rs`
  - [x] `GoalState`-only consumers (api, cli) now source from `hkask_types::GoalState` directly; `hkask-test-harness` `core` dep DROPPED (goal was its only core usage); `hkask-types` dev-dep swapped `services-core` → `hkask-goal`
  - [x] DELETE: remove `pub mod goal` + `pub use goal::{...}` from core; `chrono`+`uuid` kept (used by `identity.rs`/`error/mod.rs`)
  - [x] `cargo test -p hkask-goal -p hkask-storage -p hkask-test-harness -p hkask-api -p hkask-cli --lib` + `cargo test -p hkask-types --test types_contract` all pass (0 failures)
  - [x] added `README.md` (the one net-new artifact — HEAD had the crate but not the README)

**Checkpoint 2 (partial):** goal extracted, workspace green, core pub-items 76 → 61.

- [x] **2.2** Extract `identity` → `hkask-identity` ✅ (verified 2026-07-17)
  - [x] CREATE: move `HumanUser`/`ReplicantIdentity`/`UserSession`/`RegistrationRequest`/`Invite`/`InviteStatus`/`RegistrationError` (`Role`/`OAuthProvider` re-exported from `hkask_types::identity`)
  - [x] WIRE 4 consumers (rigorous scan; `api/routes/auth.rs` was a false positive — uses `OAuthProvider` from `hkask_types::identity` directly): `api/middleware/admin.rs` (Role → `hkask_types::identity::Role`), `services-context/storage.rs` (split: ReplicantIdentity → `hkask-identity`, error types stay core), `cli/user.rs`, `storage/user_store.rs`
  - [x] DELETE: remove `pub mod identity` + `pub use identity::{...}` from core; drop `chrono` from core deps (only `identity.rs` used it; `thiserror`+`hkask-wallet-types` kept — used by `error`/`config`)
  - [x] `hkask-storage` `core` dep DROPPED entirely (identity was its last core usage)
  - [x] `cargo test` all pass (core 30, identity 0, storage 71, context 2, cli 24, api 35, storage-contract 5; 0 failures); `cargo check --workspace` clean; clippy clean on non-inference-dep crates
  - [x] added `README.md` (the one net-new artifact — HEAD had the crate but not the README)

**Checkpoint 2:** ✅ storage-coupled modules extracted (goal + identity). workspace green. core pub-items 90 → 47. `hkask-storage` is now core-free.

## Phase 3 — Cross-cutting: inference services

- [ ] **3.1** Extract `inference_svc` + `model_cache` → `hkask-services-inference`
  - [ ] Resolve Q1: new crate vs fold into `hkask-inference` (recommend: new crate)
  - [ ] CREATE: move `inference_svc.rs` + `model_cache.rs` (incl. poison-recovery test) into new crate
  - [ ] WIRE 8+ consumers: `hkask-api`, `hkask-cli`, `hkask-repl`, `hkask-services-chat`, `hkask-services-compose`, `hkask-services-context`
  - [ ] DELETE: remove `pub mod inference_svc`/`pub mod model_cache` + re-exports from core; drop `hkask-inference` from core deps
  - [ ] `cargo test -p hkask-services-inference` passes (incl. poison-recovery regression)
  - [ ] `cargo check -p hkask-api -p hkask-cli -p hkask-repl -p hkask-services-chat -p hkask-services-compose -p hkask-services-context` passes
  - [ ] (If too large) split into 3.1a CREATE / 3.1b WIRE / 3.1c DELETE; `cargo check --workspace` after each

**Checkpoint 3:** `hkask-services-core` = `error` + `config` + `settings` only (≤ 15 pub items), workspace green

## Final verification

- [ ] `rg -c '^\s*pub ' crates/hkask-services-core/src/` ≤ 15
- [ ] `core`'s `Cargo.toml` description matches contents (`ServiceError, ServiceConfig, HkaskSettings`)
- [ ] `cargo check --workspace && cargo test --workspace` green
- [ ] `bash docs/ci/verify-docs.sh` reports 0 errors
- [ ] `cargo clippy -p hkask-services-core -- -D warnings` clean