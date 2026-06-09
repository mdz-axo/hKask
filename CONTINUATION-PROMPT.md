# CONTINUATION PROMPT — Session 26: Complete Remaining Service Extractions

**Read these files first (in this order):**

1. **`HANDOFF.md`** — Full session history, key decisions (#1–#76), service module inventory (§3), file reference map (§6), open questions (§7).
2. **`CONTINUATION.md`** — Current task status.
3. **`OPEN_QUESTIONS.md`** — Structured F1–F10 entries with force classifications.

**Load these skills before any code changes:**

1. `refactor-service-layer` — Required. Governing methodology.
2. `coding-guidelines` — Required. Surgical changes only.
3. `constraint-forces` — Required. Classify every design decision.
4. `zoom-out` — Required before each extraction.

---

## Context

Session 25 completed F10 (RecalledSemantic typed DTO), created OPEN_QUESTIONS.md,
and refreshed the test inventory. The user then clarified that the remaining
extraction items are **not speculative** — the mandate is to extract ALL business
logic to services and delete legacy code.

Session 25 began this work with three successful extractions before the session
ended mid-test-run:

| Extraction | Status | Service |
|-----------|--------|---------|
| `EnsembleService::get_chat` + `list_deliberations` | ✅ Done | `EnsembleService` |
| `SovereigntyService::grant_consent_and_fetch` | ✅ Done | `SovereigntyService` |
| `ConsolidationService::check_rate_limit` + `db_path_for_agent` | ✅ Done | `ConsolidationService` |

Build and clippy are clean. **Tests were not verified** — the test run was
interrupted. Start by verifying tests, then continue with remaining extractions.

---

## Remaining Extractions (Priority-Ordered)

### 1. `ensemble.rs` `standing_start` — HIGH

**File:** `crates/hkask-api/src/routes/ensemble.rs` L486-565

~80 lines of orchestration that must move to `EnsembleService`:
1. Build `StandingSessionConfig` from request DTOs (surface-specific mapping — stays in route)
2. `StandingSession::from_config(config)` → service
3. MCP tool discovery + `with_available_tools()` → service (needs `mcp_runtime`)
4. `with_store(standing_session_store)` → service (needs `standing_session_store`)
5. `with_gas_governance(gas_governance)` → service (needs gas governance port)
6. `persist_session()` + `post_initial_messages()` → service
7. Store in `standing_sessions` HashMap → stays in route (surface-specific live state)

**What needs to change:**
- Add `mcp_runtime`, `standing_session_store`, and `gas_governance` fields to `EnsembleContext`
- Add `EnsembleService::start_standing_session()` method that does steps 2-6
- Route calls service, then stores result in `standing_sessions`
- Gas governance currently lives only on `ApiState`. It must be constructible from `ServiceContext` (the `CyberneticsLoop` is already there). Either add `gas_governance` to `EnsembleContext` or construct it inside the service method.

**Constraint classification:**
- Standing session orchestration is domain logic → Evidence (measured by duplicated steps if deleted)
- `gas_governance` is surface-derived from `ServiceContext::cybernetics_loop` → Guideline (can be moved)
- `standing_sessions` live state map is surface-specific → Guideline (stays in route)

### 2. `sovereignty.rs` consent enforcement — HIGH

**File:** `crates/hkask-api/src/routes/sovereignty.rs` L219-226

```rust
if !access.has_consent && access.classification != "PUBLIC" {
    return Err(ApiError::Forbidden { ... });
}
```

This P1 Prohibition enforcement (deny access without consent for non-public data)
is inline in the API. The CLI would need to duplicate it. Extract to
`SovereigntyService::check_access_and_authorize()` that returns a `Result` denying
access when consent is missing for non-public categories.

**What needs to change:**
- Add `SovereigntyService::check_access_and_authorize()` that calls `check_access`
  and returns `Err(ServiceError::ConsentDenied)` when `!has_consent && classification != "PUBLIC"`
- Route calls this method instead of doing inline enforcement
- Add `ServiceError::ConsentDenied` variant
- Add `ApiError` mapping for `ServiceError::ConsentDenied`

### 3. `chat.rs` PromptStrategy framing — MEDIUM

**File:** `crates/hkask-api/src/routes/chat.rs` L68-76

`PromptStrategy::from_input`, template_id prefix formatting, strategy.name()
fallback are domain logic currently only in the API. The CLI does its own framing
differently (in the REPL's `chat_turn`).

**Depth test:** Deleting this from the route would make the API unable to determine
which prompt strategy to use without re-implementing the logic. However, the CLI
has different framing. This is **Divergent** — both surfaces do different things for
the same intent. Consider whether unification adds more complexity than it removes.

**Recommendation:** If CLI and API framing diverge meaningfully, add a
`ChatService::resolve_prompt_strategy()` method that both surfaces can call with
their own parameters. If the CLI doesn't need this, leave it surface-specific.

### 4. `episodic.rs` — MEDIUM (new service assessment)

**File:** `crates/hkask-api/src/routes/episodic.rs`

All 3 handlers construct `StorageRequest`/`RecallRequest` from HTTP DTOs, resolve
confidence defaults, and map `MemoryError → ApiError`. The request construction
uses domain types (`Confidence::new()`, `StorageRequest::episodic()`) that are
already properly typed (F9). The error mapping is surface-specific (HTTP status codes).

**Depth test:** An `EpisodicService` would wrap the port calls with error
normalization, but both surfaces (API and potential CLI) would still need their own
error→presentation mapping. The request construction uses domain types already.
A service layer would be a shallow adapter — fails depth test per #71.

**Recommendation:** Leave as-is. The typed DTOs (F9) already eliminated the fragile
destructuring. Error mapping is surface-specific. No CLI counterpart exists.

### 5. `cns.rs` — LOW (depth-test skip)

Already evaluated in Session 23 (#65). Variety aggregation and SSE subscription
are shallow operations. The SSE wiring is deeply tied to HTTP response streaming.
Creating a `CnsService` would be a shallow adapter. **Skip.**

### 6. `git.rs` — LOW (depth-test skip)

Archive handler's dual-source assembly (template crate + SHA resolution) is a
minor data composition. `ArchivalService` already exists for the CLI path. The API
route uses different sources (`git_cas` vs `GitCasAdapter`). **Divergent — skip.**

---

## Per-Task Discipline

1. **Zoom out** before each extraction — read the route, the service, and the context
2. **Apply depth test** — would deleting the new method force callers to duplicate logic?
3. **One extraction per commit** — no cross-domain refactors
4. **Verify after each** — `cargo check -p hkask-services -p hkask-api && cargo test -p hkask-services -p hkask-api`
5. **Update HANDOFF.md** after all extractions — add Session 26 entry, key decisions, file reference map

## Build Commands

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

**Start by verifying the interrupted test run from Session 25.**

---

*ℏKask - A Minimal Viable Container for Agents — v0.23.0*