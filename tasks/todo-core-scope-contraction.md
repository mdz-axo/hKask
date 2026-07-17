# TODO — `hkask-services-core` Scope Contraction

> Companion to `tasks/plan-core-scope-contraction.md`. The in-progress `tasks/todo.md`
> (corpus-ingest) is unchanged.

## Phase 1 — PoC: prove the strangler-fig pattern

- [ ] **1.1** Extract `self_heal` → `hkask-services-self-heal`
  - [ ] CREATE: move `self_heal/{healer,types,strategies,tests}.rs` into new crate
  - [ ] WIRE: update `hkask-services-context/build/reg_wallet.rs` import
  - [ ] DELETE: remove `pub mod self_heal` from core; drop unused core deps
  - [ ] `cargo test -p hkask-services-self-heal` passes
  - [ ] `cargo check -p hkask-services-context` passes
  - [ ] `rg 'pub mod self_heal' crates/hkask-services-core/src/` empty

- [ ] **1.2** Extract `verification` → `hkask-services-verification`
  - [ ] CREATE: move `verification/*` into new crate
  - [ ] WIRE: update `hkask-cli/commands/sovereignty.rs` import
  - [ ] DELETE: remove `pub mod verification` from core
  - [ ] `cargo test -p hkask-services-verification` passes
  - [ ] `cargo check -p hkask-cli` passes

**Checkpoint 1:** pattern proven, `cargo check --workspace && cargo test --workspace` green

## Phase 2 — Dependent: storage-coupled modules

- [ ] **2.1** Extract `goal` → `hkask-goal`
  - [ ] CREATE: move `Goal`/`GoalCriterion`/`GoalArtifact`/`IllegalGoalTransition` (NOT `GoalState` — stays in `hkask-types`)
  - [ ] WIRE: `hkask-storage/goals.rs`, `hkask-api/routes/goal.rs`, `hkask-cli/commands/goal.rs`
  - [ ] DELETE: remove `pub mod goal` + re-export from core
  - [ ] `cargo test -p hkask-goal -p hkask-storage --lib` passes

- [ ] **2.2** Extract `identity` → `hkask-identity`
  - [ ] CREATE: move `HumanUser`/`ReplicantIdentity`/`UserSession`/`RegistrationRequest`/`Invite`/`Role`/etc.
  - [ ] WIRE: `hkask-cli/commands/user.rs`, `hkask-services-context/storage.rs`, `hkask-api/routes/auth.rs`, `hkask-storage/user_store.rs`
  - [ ] DELETE: remove `pub mod identity` + re-export from core; drop `hkask-wallet-types` from core if unused
  - [ ] `cargo test -p hkask-identity` passes
  - [ ] `cargo check -p hkask-cli -p hkask-services-context -p hkask-api -p hkask-storage` passes

**Checkpoint 2:** storage-coupled modules extracted, workspace green

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