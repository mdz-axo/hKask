# Handoff — Error Unification + Service Extraction

**Date:** 2026-06-14
**Session scope:** Unify error handling across hKask surfaces, extract duplicated business logic from CLI/API into `hkask-services`.
**Progress:** ~85% complete. Error unification is functionally complete. Service extraction started with 3 CLI commands refactored.

---

## 1. What Was Done

### Error Handling Unification (Core)

- **Deleted all 4 CLI surface error enums:** `AgentError`, `CuratorError`, `UserError`, `RegistryError`(CLI) — removed from `crates/hkask-cli/src/errors.rs` (file deleted) and `crates/hkask-cli/src/lib.rs` (`pub mod errors;` removed).
- **Deleted `HkaskError`** from `crates/hkask-types/src/error.rs` — enum, impl block (7 methods), 2 `From` impls, and re-export from `lib.rs` all removed. Updated doc comment to reflect current 3-layer architecture (`InfrastructureError` → domain enums → `ServiceError`).
- **Converted all CLI commands** to return `ServiceError` instead of surface error types:
  - `agent.rs` — 5 functions
  - `user.rs` — 12 functions
  - `goal.rs` — 3 functions
  - `curator.rs` — 4 functions
  - `template.rs` — 3 functions
  - `onboarding.rs` — `OnboardingError` reduced to `Cancelled` + `Service(#[from] ServiceError)`
- **Converted all API routes** to return `ServiceErrorResponse` (newtype in `crates/hkask-api/src/error.rs`):
  - Added `ServiceErrorResponse` newtype with `IntoResponse` (delegates to `ApiError` for HTTP mapping)
  - Added `From<ApiError> for ServiceErrorResponse` bridge (temporary — allows existing manual `ApiError::` constructions during migration)
  - Added 6 `From<DomainError> for ServiceErrorResponse` bridges (`AcpError`, `EscalationError`, `uuid::Error`, `AgentRegistryError`, `AgentPodError`, `RegistryError`)
  - All 11 route files: return types changed, `.map_err(ApiError::from)` → `?`
- **Added 3 methods to `ServiceError`** (`crates/hkask-services/src/error.rs`):
  - `is_retryable()` — CNS gas budget signal (retryable: `InferencePort(Connection/CircuitOpen)`, `Embedding(Connection/Api)`, `Infra(Io)`, `RateLimited`, `Matrix`, `Config`, `Keystore`, `McpTool`)
  - `nu_event()` — CNS ν-event emission (per-variant span mapping, user-input errors return `None`)
  - `message_key()` — i18n stable keys (`error.<domain>.<condition>`)
- **Added `McpTool` variant** to `ServiceError` with `McpErrorKind` for MCP error unification (F1).
- **Added `Backup` variant handling** to all 4 methods + API adapter (other work added this variant).

### Documentation

- `crates/hkask-services/docs/error-landscape.ttl` — RDF Turtle graph mapping 47 error types with `:livesIn`, `:wraps`, `:surfacesTo`, `:duplicates` predicates.
- Mermaid ER diagram (in session output) showing entities, relationships, cardinality, and duplication edges.
- Grill-me assessment (4 rounds, 11 questions) with per-area ratings.
- Essentialist elimination report (Gates 1-3) with 10 DELETE, 14 REDUCE, 23 PRESERVE verdicts.
- Future open questions F1-F6 documented.

### Service Extraction (Quick Wins)

- **`agent.rs`** — Eliminated 3× `Database::open()`, 2× `AcpRuntime::new()`, 3× `AgentRegistryStore::new()`, 1× `ServiceConfig::from_env()`. Now uses `build_service_context()` → `ctx.agent_registry_store()` and `ctx.identity()`.
- **`user.rs`** — Eliminated 1× `Database::open()`, 1× `UserStore::new()`, 1× `ServiceConfig::from_env()`. `build_store()` now returns `ctx.user_store().clone()`.
- **`embed_corpus.rs`** — Eliminated 1× `Database::open()`, 1× `UserStore::new()`, 1× `ServiceConfig::from_env()`. Auth now uses `build_service_context().user_store()`.

### Pre-existing Issues Fixed

- **`hkask-improv`** — Added missing `ImprovProtocol` trait to `protocol.rs`. Added `PartialEq, Eq` derives to `ImprovCascade` and `ImprovMode`. Fixed 2 cascade test failures.
- **`hkask-services/src/context.rs`** — Fixed `Keychain::default()` call (was wrapped in `if let Ok()` but returns `Keychain` directly, not `Result`).
- **`hkask-mcp-training`** — Added `#[allow(clippy::should_implement_trait)]` to `TrainingProviderId::from_str`.
- **Stale doc comments** — Updated `hkask-types/src/error.rs` and `hkask-services/src/error.rs` headers to reflect current architecture.

### Build Status

| Crate | Check | Tests |
|-------|-------|-------|
| `hkask-types` | ✅ | ✅ |
| `hkask-storage` | ✅ | ✅ |
| `hkask-improv` | ✅ | ✅ |
| `hkask-services` | ✅ | ✅ |
| `hkask-api` | ✅ | ✅ |
| `hkask-cli` | ✅ | ✅ |
| Full workspace | ✅ | ✅ (31 suites, 0 failures) |

**Known pre-existing issues (unrelated):**
- `hkask-mcp` — 20 errors from gix/git API changes (`ObjectId::from_bytes`, `null_sha1`, `index_from_worktree`, `try_into_commit`)
- `hkask-mcp-communication` — 1 dead_code warning on `queue` field

---

## 2. What Remains

### HIGH — Complete Service Extraction

**1. `registry.rs` — 2 remaining `Database::open()` calls**

File: `crates/hkask-cli/src/commands/registry.rs` (lines 81, 131)

These use custom DB paths for `rm` commands (styles, templates). Current `AgentService` only opens the default DB. Options:
- Add `AgentService::open_database(path, passphrase)` method
- Or accept that `rm` commands legitimately need custom DB paths (they operate on arbitrary databases)

**2. CNS set-points through `CnsService`**

Files: `crates/hkask-cli/src/commands/cns.rs` (lines 104, 136, 144, 184), `crates/hkask-cli/src/commands/kata.rs` (line 222)

`CnsService` (`crates/hkask-services/src/cns.rs`) exists but only has `new()`. Needs:
```rust
impl CnsService {
    pub fn get_set_points(&self) -> SetPoints { ... }
    pub fn update_set_points(&self, config: &SetPointsConfig) -> SetPoints { ... }
}
```
Then CLI commands call `CnsService` instead of `hkask_cns::SetPoints` directly.

**3. `spec.rs` through `SpecService`**

File: `crates/hkask-cli/src/commands/spec.rs`

`SpecService` (`crates/hkask-services/src/spec.rs`) exists with `capture`, `cultivate`, `validate` methods. CLI currently calls `hkask_storage::spec_types::SpecId::from_string()` directly. Route through `SpecService`.

### MEDIUM — API Route Cleanup

**4. Delete temporary `From<ApiError> for ServiceErrorResponse` bridge**

File: `crates/hkask-api/src/error.rs` (lines 108-120)

Once all route files have manual `ApiError::` constructions converted to `ServiceError` variants, delete this bridge. Currently ~40 manual constructions remain across route files (they auto-convert via the bridge, so no urgency).

**5. Convert remaining manual `ApiError::` constructions**

Files: `crates/hkask-api/src/routes/bundles.rs`, `consolidation.rs`, `episodic.rs`, `git.rs`, `pods.rs`, `sovereignty.rs`, `backup.rs`, `mcp.rs`

Pattern to apply:
- `ApiError::BadRequest { message }` → `ServiceError::ValidationError(message)`
- `ApiError::NotFound { resource, id }` → domain-specific `ServiceError::AgentNotFound(id)` etc.
- `ApiError::Internal { message }` → `ServiceError::Infra(InfrastructureError::Database(message))`
- `ApiError::Forbidden { reason }` → `ServiceError::Acp(AcpError::CapabilityDenied(...))`

### LOW — Deeper Architecture

**6. Guardrail error type deletions (11 types blocked)**

Types blocked by `impl_from_rusqlite!` macro (provides `From<rusqlite::Error>` which `InfrastructureError` can't — hkask-types doesn't depend on rusqlite):
`NuEventError`, `TripleError`, `ConsentStoreError`, `SovereigntyStoreError`, `EscalationError`, `GoalRepositoryError`, `AgentRegistryError`, `UserStoreError`, `SpecError`, `KataHistoryError`, `GalleryStoreError`

To unblock: add `From<rusqlite::Error> for InfrastructureError` in `hkask-storage` (not `hkask-types`, since storage depends on rusqlite). Then these types can be deleted/reduced.

**7. `GitError` deletion**

Used in `hkask-mcp/src/git_cas/mod.rs` (12 references) + `AgentPodError::CrateLoadError(#[from] GitError)`. Requires refactoring git_cas adapter to use `InfrastructureError` + `ServiceError`.

**8. Error context chaining (F3)**

Add `#[source]` fields to String sentinel variants in `ServiceError` so original errors aren't lost. Example:
```rust
Consolidation {
    #[source]
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
    message: String,
}
```
This is a breaking enum change — all construction sites need updating.

---

## 3. Key Decisions to Preserve

1. **`ServiceErrorResponse` newtype pattern** — `ServiceError` cannot implement `IntoResponse` directly (orphan rule: both trait and type are foreign). The newtype in `hkask-api/src/error.rs` bridges this. Route handlers return `Result<Json<T>, ServiceErrorResponse>`. Do NOT implement `IntoResponse for ServiceError` in `hkask-services` — that would leak Axum types into the service layer.

2. **`From<DomainError> for ServiceErrorResponse` bridges** — The `?` operator needs direct `From<E> for ServiceErrorResponse` (Rust doesn't compose `From` impls). These bridges in `hkask-api/src/error.rs` are necessary for `?` to work in route handlers. Each new domain error type used in routes needs a bridge.

3. **CLI commands return `ServiceError` directly** — No wrapper type needed. `or_exit()` and `block_on!()` only need `Display`, which `ServiceError` provides via `#[error("...")]`.

4. **Surface error enums are deleted, not adapted** — `AgentError`, `CuratorError`, `UserError`, `RegistryError`(CLI) are permanently removed. Do NOT recreate them. All new CLI commands return `ServiceError`.

5. **`OnboardingError::Cancelled` is preserved** — This is a flow-control signal (user chose to cancel), not an error. It's the only surface-specific error variant that survived.

6. **`HkaskError` is permanently deleted** — Its `is_retryable()` and `to_mcp_kind()` logic now lives in `ServiceError`. The `From<std::io::Error>` and `From<serde_json::Error>` impls were for `HkaskError` only (unused elsewhere) and were deleted with it.

7. **`InfrastructureError` does NOT depend on `rusqlite`** — `hkask-types` is a foundation crate without database dependencies. Domain error types in `hkask-storage` use `impl_from_rusqlite!` macro to bridge this gap. Any solution for deleting pass-through domain errors must work within this constraint (e.g., add `From<rusqlite::Error> for InfrastructureError` in `hkask-storage`, not `hkask-types`).

---

## 4. Recommended Skills and Commands

**Skills to load:**
- `coding-guidelines` — before any code changes
- `refactor-service-layer` — for extracting more CLI logic into services
- `essentialist` — when auditing error types for deletion

**Verification commands:**
```bash
cargo check -p hkask-services -p hkask-cli -p hkask-api
cargo test --workspace
cargo clippy -p hkask-services -p hkask-cli -p hkask-api -- -D warnings
```

**File watcher note:** Some files in this repo are watched/reverted by an external process. If `edit_file` changes don't persist, use `sed -i` via terminal or `write_file` for complete file rewrites. Files in `crates/hkask-api/src/routes/` are particularly affected.
