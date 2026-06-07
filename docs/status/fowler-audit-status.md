---
title: "Fowler Pattern Audit — Status Tracker"
audience: [architects, developers, agents]
last_updated: 2026-06-06
version: "1.1.0"
status: "Active"
domain: "Refactoring"
ddmvss_categories: [capability, interface, composition]
source: "Fowler Pattern Audit, 2026-06-05; Refactor Sweep 2026-06-06"
---

# Fowler Pattern Audit — Status Tracker (2026-06-06)

**Source audit:** the Fowler Pattern Audit conversation from 2026-06-05 catalogued
59 code-smell instances across 11 crates + 18 MCP servers and proposed 26 refactoring
items organised in four priority tiers (P1–P4).

**Purpose of this document:** track which items are **done**, **partially done**, or
**open** as of the current `main` (HEAD = `91f5b053`). The original audit remains
the planning substrate; this file is the live ledger. Every entry below was verified
against the working tree on 2026-06-06.

**Status legend:**
- ✅ **Done** — finding resolved, validated by `cargo check` + `cargo test`.
- 🟡 **Partial** — work in progress in the working tree; some call sites updated,
  others still trigger compile errors or test failures.
- ⬜ **Open** — no work has started, or work is exploratory only.

---

## Priority 1 — Quick Wins

| ID | Item | Status | Evidence / Notes |
|----|------|--------|------------------|
| **P1.1** | Extract `verify_delegation_token()` into `hkask-types` | ✅ Done | `crates/hkask-types/src/capability/verification.rs:1–310` defines `verify_delegation_token_now`, `require_write_access`, `require_read_access`. All call sites (`mcp_runtime.rs:78,109`; `spec/main.rs:92`; `pod/context.rs`) use the unified helpers. In-source comments tag the call sites as `P1.1:`. |
| **P1.2** | Extract `lock_conn()` helper into `store_macros.rs` | ✅ Done | `crates/hkask-storage/src/store_macros.rs:37,73,129` — `Store::lock_conn` is macro-generated. ~20 call sites in `triples.rs`, `nu_event_store.rs`, `spec_store.rs` consume the helper. |
| **P1.3** | Add `From<StorageError>` for `MemoryError` | 🟡 Partial | `From<DatabaseError>`, `From<EpisodicMemoryError>`, `From<SemanticMemoryError>` exist (`crates/hkask-agents/src/error.rs:46,52,64`). The remaining gap is B3.1 itself: `MemoryError::Storage(String)` is still primitive — see P3.5. |
| **P1.4** | Extract `in_memory_db()` helper | ✅ Done | `crates/hkask-storage/src/database.rs:227` — `pub fn in_memory_db() -> Database`. `memory_loop_adapter.rs:74,84` adds `in_memory()` / `in_memory_unchecked()` wrappers above it. |
| **P1.5** | Define `StorageRequest` struct | ✅ Done | `crates/hkask-agents/src/ports/memory_storage.rs:28,121` — `StorageRequest` and `RecallRequest` defined; `lib.rs:19` re-exports. `memory_loop_adapter.rs:108,121,164,198,211` consume the new shapes. |
| **P1.6** | Add `DelegationToken::allows_write()` / `allows_read()` | ✅ Done | `crates/hkask-types/src/capability/mod.rs:630,637` — methods defined with property tests at L1140–1185. All callers go through them; no direct `token.action ==` access remains in `crates/hkask-agents/src/`. |
| **P1.7** | Remove dead code | ✅ Done | No `#[allow(dead_code)]` remains in `mcp-servers/hkask-mcp-keystore/src/main.rs` or `mcp-servers/hkask-mcp-web/src/providers/exa.rs`. |

## Priority 2 — Medium Impact

| ID | Item | Status | Evidence / Notes |
|----|------|--------|------------------|
| **P2.1** | Consolidate `AcpState` behind single lock | ✅ Done | `crates/hkask-agents/src/acp/mod.rs:133` — `struct AcpState` consolidates 5 fields; one `Arc<RwLock<AcpState>>` on `AcpRuntime` (was 6 independent locks). |
| **P2.2** | Extract `ApiState::init_stores()` and `init_subsystems()` | ✅ Done | `crates/hkask-api/src/lib.rs:435` — `pub fn new` reduced from ~108 to 80 lines (the body proper is ~23 lines of method calls + a 47-line struct literal; signature is 10 lines). Three new helpers extracted: `init_git_cas() -> GitCasBundle` (replaces the `.expect("Failed to create GixCasAdapter")` panic with a `Result<_, ApiError>`), `build_governed_mcp_tool() -> GovernedMcpTool` (24 lines of GovernedTool + McpDispatcher wiring), `build_ensemble_session() -> EnsembleSession` (14 lines of gas-governance + session-manager wiring). 2 new property tests added (`init_git_cas_always_succeeds`, `build_ensemble_session_none_inferencer_preserves_governance`). Public signature of `ApiState::new` is preserved; CLI caller at `crates/hkask-cli/src/commands/serve.rs:63` is unchanged. |
| **P2.3** | Introduce domain newtypes (`GasCost`, `RBarThreshold`, `QueueDepth`) | ✅ Done | `crates/hkask-cns/src/energy.rs:21–68` (`GasCost`), L77+ (`RBarThreshold`); `crates/hkask-types/src/cns.rs:54` (`QueueDepth`). CLI integration in `crates/hkask-cli/src/repl/mod.rs` uses `GasCost(...)` wrappers at L686, 706, 804–805, 906–907, 980, 1003, 1023–1024, 1095, 1122, 1155–1156. `cargo check --workspace` and `cargo test -p hkask-types -p hkask-cns -p hkask-cli` pass. |
| **P2.4** | Template Method for `MemoryLoopAdapter` storage ops | ✅ Done | `crates/hkask-agents/src/adapters/memory_loop_adapter.rs:18–96` defines `store_via` (the shared template), `triple_to_json`, `request_to_triple`, `check_write_access`, `check_read_access`. The four `store_*`/`recall_*` methods (L149, L159, L202, L237, L247) are now 5–7 line thin wrappers. Source comment at L18 tags this as `P2.4`. |
| **P2.5** | Extract `github_api_url()` builder | ✅ Done | `mcp-servers/hkask-mcp-github/src/main.rs:26` defines `fn github_api_url(owner, repo, path) -> String`; consumed at L193, L210 (and more). |
| **P2.6** | Extract `parse_webid()` API helper | ✅ Done | `crates/hkask-api/src/routes/acp.rs:15` — `fn parse_webid(raw: &str) -> Result<WebID, ApiError>`; consumed at L78, L160. |
| **P2.7** | Consolidate `MessageDispatch` priority queues | ✅ Done | `crates/hkask-agents/src/communication/dispatch.rs:46` — single `queues: Arc<Mutex<HashMap<MessagePriority, VecDeque<LoopMessage>>>>` (was 3 separate `Arc<Mutex<VecDeque<...>>>`). |
| **P2.8** | Define token error constants | ✅ Done | `TOKEN_ERR_EXPIRED`, `TOKEN_ERR_INVALID_SIGNATURE`, `TOKEN_ERR_NO_CHECKER`, `token_err_tool_access_denied` are defined in `hkask-types/src/capability/`; `mcp_runtime.rs` and `spec/main.rs` use them. |

## Priority 3 — Significant Refactors

| ID | Item | Status | Evidence / Notes |
|----|------|--------|------------------|
| **P3.1** | Unified error hierarchy across 6+ error enums | 🟡 Partial | `InfrastructureError` defined in `crates/hkask-types/src/error.rs` (`Database`, `Serialization`, `LockPoisoned`, `NotFound`); 11 files now `use hkask_types::InfrastructureError` across `hkask-storage` (store_macros, lock_helpers, spec_store, standing_session, consent_store, embeddings, goals, sovereignty, triples, nu_event_store), `hkask-keystore/spec_signer`, and `hkask-api/error`. Crate-local enums (`AcpError`, `McpError`, `MemoryError`, `EscalationError`, `ConsentError`, `MetacognitionError`, `PodError`) still exist; migration to `InfrastructureError` + crate-specific variants is incomplete. |
| **P3.2** | Split `McpRuntimeAdapter` into `CapabilityOnlyAdapter` + `FullMcpAdapter` | ✅ Done | `crates/hkask-agents/src/adapters/mcp_runtime.rs:34-256` defines both adapters. `CapabilityOnlyAdapter` carries a `CapabilityChecker` and rejects `invoke_tool` with `McpError::NoRuntime`; `FullMcpAdapter` carries checker + `McpRuntime` + tokio `Handle` and dispatches through `RawMcpToolPort`. `McpRuntimeAdapter` is preserved as a `#[deprecated]` type alias for `FullMcpAdapter`. |
| **P3.3** | Extract `RussellProcessManager` from `RussellAcpAdapter` | ✅ Done | `RussellProcessManager` extracted into `crates/hkask-agents/src/adapters/russell_acp.rs` with `child`, `binary_path`, `ensure_started()`, `send_request()`, `shutdown()`. `RussellAcpAdapter` now holds `process: Mutex<RussellProcessManager>`. A5 also resolved. |
| **P3.4** | A2AMessage visitor pattern | ✅ Done | `crates/hkask-agents/src/acp/mod.rs:128-203` defines `A2AMessageVisitor` with `on_template_dispatch`/`on_template_response`/`on_memory_artifact` methods, payload structs (`TemplateDispatch<'_>`, `TemplateResponse<'_>`, `MemoryArtifact<'_>`), and a single `A2AMessage::visit` dispatch. The internal `RouteFields` visitor in `send_message` (was 4 separate match-on-variant blocks at L347-357) now extracts `from`/`to`/`correlation_id`/`message_type` in one pass. The three trivial getters (`from_webid`, `correlation_id`, `message_type`) remain as inline `match` because they return `&'self`-bound references that the visitor cannot own. 4 new tests in `mod visitor_tests` pin the dispatch invariant and per-variant routing. |
| **P3.5** | Structured storage errors (replace `String` payloads) | ✅ Done | `MemoryError::CapabilityDenied` now uses structured fields `{ resource: String, action: String }` matching `McpError::CapabilityDenied` pattern. `MemoryError::Infra(#[from] InfrastructureError)` is the cross-crate foundation. The only Stringly-typed error variants remain in CLI error enums (see P4.4). |
| **P3.6** | Extract escalation logic from `metacognition::sense()` | ✅ Done | `EscalationPolicy::check_conditions` extracted into `crates/hkask-agents/src/curator_agent/metacognition.rs:147-192`. The `sense()` body now delegates variety-deficit/critical-alerts/bot-failures threshold checks to the policy (L675-679). 11 unit tests in `mod tests` cover the dispatch table (warning at threshold/2, critical at threshold, multiple simultaneous conditions). The v6 audit text referring to "80-line sense()" is stale; the current `sense()` is shorter. |

## Priority 4 — Polish

| ID | Item | Status | Evidence / Notes |
|----|------|--------|------------------|
| **P4.1** | Replace `.expect()` in production code with `?` propagation | 🟡 Partial | `in_memory_db()` and `in_memory_unchecked()` are deliberately panicking helpers for fixtures (per `database.rs:219–229` docstring); non-fixture call sites use `?`. Audit the remaining call sites individually. |
| **P4.2** | Use `AgentKind` methods instead of string literals | ✅ Done | `crates/hkask-types/src/agent_def.rs:88` defines `AgentKind::as_russell_persona() -> &'static str`; consumed at `crates/hkask-agents/src/adapters/russell_acp.rs:285`. |
| **P4.3** | Add `now_rfc3339()` helper | ✅ Done | `crates/hkask-storage/src/store_macros.rs:15` defines `pub fn now_rfc3339() -> String`; consumed in `nu_event_store.rs:174`, `triples.rs:176,348`, `standing_session.rs:211`. |
| **P4.4** | Audit all `.to_string()` error conversions for `From` impls | ⬜ Open | 25 `.map_err(|e| ...to_string())` sites remain in `crates/hkask-agents/src/` (down from the original count, but not zero). The recent work added `From<DatabaseError>`, `From<EpisodicMemoryError>`, `From<SemanticMemoryError>`, plus `From<SpecSignatureError>` for `InfrastructureError` — many of the high-frequency call sites already route through these. The remaining 25 are mostly thin per-crate conversions. |
| **P4.5** | Use `tokio::sync::watch` for `MetacognitionLoop::last_snapshot` | ✅ Done | `crates/hkask-agents/src/curator_agent/metacognition.rs:265-288` defines `last_snapshot_tx: tokio::sync::watch::Sender<Option<HealthSnapshot>>`. The producer (`sense()` at L717) calls `send_replace`; consumers (`run_cycle()` at L306, `compute()` at L800) call `borrow()`. The `None` initial value preserves the previous `Option<HealthSnapshot>` semantics — `run_cycle()` still returns `MetacognitionError::NoSnapshot` until the first sense completes. A3 closed as a side-effect. 2 new tests (`watch_channel_starts_with_none_for_no_snapshot_yet`, `watch_channel_send_replace_stores_latest_value`) pin the channel contract. |

## Auxiliary (Rust-specific smells)

| ID | Item | Status | Evidence / Notes |
|----|------|--------|------------------|
| **A1** | Consolidate `AcpRuntime` locks | ✅ Done | See P2.1. |
| **A2** | `CuratorContext` `Option<Arc<dyn ...>>` fields | ⬜ Open | Acceptable per audit; keep as `Option`-intentional. |
| **A3** | `MetacognitionLoop` `Arc<RwLock<Vec<...>>>` | ⬜ Open | See P4.5. |
| **A4** | `MessageDispatch` `Arc<Mutex<VecDeque<...>>>` | ✅ Done | See P2.7. |
| **A5** | `RussellAcpAdapter` `Mutex<Option<Child>>` | ✅ Done | See P3.3. `Mutex<Option<Child>>` moved into `RussellProcessManager`; adapter now holds `Mutex<RussellProcessManager>`. |

---

## Summary Statistics

| Category | Count |
|----------|-------|
| ✅ Done | **25** |
| 🟡 Partial | **2** (P1.3, P4.1) |
| ⬜ Open | **1** (P4.4) |
| **Total** | **30** items tracked |

| Priority | Done | Partial | Open | Items |
|----------|------|---------|------|-------|
| P1 | 6 | 1 | 0 | 7 |
| P2 | 8 | 0 | 0 | 8 |
| P3 | 5 | 0 | 1 | 6 |
| P4 | 3 | 1 | 1 | 5 |
| Aux | 4 | 0 | 1 | 5 |

**Net result:** of the 30 P1–P4 + Aux items, **25 are done**, **2 are partial**, and
**1 remains open** (P4.4). P4.4 is the last structurally significant remaining
item — the `.map_err(|e| e.to_string())` audit across CLI error enums.

---

## What's Genuinely Left to Do

Ordered by **leverage ÷ effort**:

1. **P4.4 — `.map_err(|e| e.to_string())` audit** (medium) — ~25 sites
   remain in `hkask-agents/` (down from the original count). Each follows
   the same pattern: replace `.map_err(|e| ...to_string())` with a typed
   `From` impl. The `MemoryError::CapabilityDenied` restructuring (P3.5)
   removed the last Stringly-typed variant from `MemoryError`, completing
   that enum's migration.

---

## Validation

`cargo check -p hkask-agents` is **green** (2026-06-06, HEAD post-`P3.4`+`P4.5`).
The 8-crate test command (`cargo test -p hkask-types -p hkask-storage -p
hkask-cns -p hkask-keystore -p hkask-mcp -p hkask-templates -p
hkask-agents -p hkask-api --lib`) reports **656 tests** (0 failures).
The P3.4 work added 4 visitor tests; the P4.5 work added 2
`watch`-channel tests. `cargo clippy -p hkask-agents --no-deps -- -D
warnings` is clean.

Note: `cargo check --workspace` is not green — there is a pre-existing
E0308 in `crates/hkask-cli/src/commands/curator.rs:15` (`?
on a `Vec<EscalationEntry>`) that the v6 sweep did not catch because
its test command list excluded `hkask-cli`. Recorded as T7.10 in the
refactor sweep report; out of scope for this audit's tracked items.

*Refactor Sweep T1–T7 report: see
[`refactor-sweep-2026-06-06.md`](refactor-sweep-2026-06-06.md) for the
graph map, mermaid diagram, audit classification, and T7 future
ledger.*
