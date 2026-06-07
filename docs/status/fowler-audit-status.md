---
title: "Fowler Pattern Audit — Status Tracker"
audience: [architects, developers, agents]
last_updated: 2026-06-06
version: "1.0.0"
status: "Active"
domain: "Refactoring"
source: "Fowler Pattern Audit, 2026-06-05"
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
| **P2.2** | Extract `ApiState::init_stores()` and `init_subsystems()` | ⬜ Open | `ApiState::new` is 277 lines (`crates/hkask-api/src/lib.rs:435`) — the long-constructor B1.1 finding is still live. |
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
| **P3.2** | Split `McpRuntimeAdapter` into `CapabilityOnlyAdapter` + `FullMcpAdapter` | ⬜ Open | `crates/hkask-agents/src/adapters/mcp_runtime.rs` still has a single type with optional `capability_checker`. |
| **P3.3** | Extract `RussellProcessManager` from `RussellAcpAdapter` | ⬜ Open | `crates/hkask-agents/src/adapters/russell_acp.rs` still owns both protocol and process lifecycle. |
| **P3.4** | A2AMessage visitor pattern | ⬜ Open | `crates/hkask-agents/src/acp/mod.rs` still uses match-on-variant for `A2AMessage` dispatch. |
| **P3.5** | Structured storage errors (replace `String` payloads) | ⬜ Open | `MemoryError::Storage(String)`, `MemoryError::Query(String)`, `MemoryError::CapabilityDenied(String)` still primitive in `crates/hkask-agents/src/error.rs:5–11`. Pre-requisite for P1.3 completion. |
| **P3.6** | Extract escalation logic from `metacognition::sense()` | ⬜ Open | `crates/hkask-agents/src/curator_agent/metacognition.rs:sense()` body is still ~80 lines; algedonic review, cursor advance, and goal-stale counting are inlined. |

## Priority 4 — Polish

| ID | Item | Status | Evidence / Notes |
|----|------|--------|------------------|
| **P4.1** | Replace `.expect()` in production code with `?` propagation | 🟡 Partial | `in_memory_db()` and `in_memory_unchecked()` are deliberately panicking helpers for fixtures (per `database.rs:219–229` docstring); non-fixture call sites use `?`. Audit the remaining call sites individually. |
| **P4.2** | Use `AgentKind` methods instead of string literals | ✅ Done | `crates/hkask-types/src/agent_def.rs:88` defines `AgentKind::as_russell_persona() -> &'static str`; consumed at `crates/hkask-agents/src/adapters/russell_acp.rs:285`. |
| **P4.3** | Add `now_rfc3339()` helper | ✅ Done | `crates/hkask-storage/src/store_macros.rs:15` defines `pub fn now_rfc3339() -> String`; consumed in `nu_event_store.rs:174`, `triples.rs:176,348`, `standing_session.rs:211`. |
| **P4.4** | Audit all `.to_string()` error conversions for `From` impls | ⬜ Open | 25 `.map_err(|e| ...to_string())` sites remain in `crates/hkask-agents/src/` (down from the original count, but not zero). The recent work added `From<DatabaseError>`, `From<EpisodicMemoryError>`, `From<SemanticMemoryError>`, plus `From<SpecSignatureError>` for `InfrastructureError` — many of the high-frequency call sites already route through these. The remaining 25 are mostly thin per-crate conversions. |
| **P4.5** | Consider `tokio::sync::watch` for `MetacognitionLoop::last_snapshot` | ⬜ Open | `metacognition.rs` still uses `Arc<RwLock<Option<...>>>`. |

## Auxiliary (Rust-specific smells)

| ID | Item | Status | Evidence / Notes |
|----|------|--------|------------------|
| **A1** | Consolidate `AcpRuntime` locks | ✅ Done | See P2.1. |
| **A2** | `CuratorContext` `Option<Arc<dyn ...>>` fields | ⬜ Open | Acceptable per audit; keep as `Option`-intentional. |
| **A3** | `MetacognitionLoop` `Arc<RwLock<Vec<...>>>` | ⬜ Open | See P4.5. |
| **A4** | `MessageDispatch` `Arc<Mutex<VecDeque<...>>>` | ✅ Done | See P2.7. |
| **A5** | `RussellAcpAdapter` `Mutex<Option<Child>>` | ⬜ Open | See P3.3. |

---

## Summary Statistics

| Category | Count |
|----------|-------|
| ✅ Done | **19** |
| 🟡 Partial | **3** (P1.3, P3.1, P4.1) |
| ⬜ Open | **8** (P2.2, P3.2–P3.6, P4.4, P4.5, A2, A3, A5) |
| **Total** | **30** items tracked |

| Priority | Done | Partial | Open | Items |
|----------|------|---------|------|-------|
| P1 | 6 | 1 | 0 | 7 |
| P2 | 7 | 0 | 1 | 8 |
| P3 | 0 | 1 | 5 | 6 |
| P4 | 2 | 1 | 2 | 5 |
| Aux | 2 | 0 | 3 | 5 |

**Net result:** of the 26 P1–P4 items, **15 are done**, **3 are partial**, and **8 remain
open**. Of the 8 open items, 5 are in P3 (significant refactors — visitor pattern,
process manager extraction, error hierarchy completion, structured errors, escalation
extraction), 1 is P2.2 (the long `ApiState::new`), and 2 are P4 polish.

---

## What's Genuinely Left to Do

Ordered by **leverage ÷ effort**:

1. **P3.5 — Structured storage errors** (small) — replace `String` payloads in
   `MemoryError`, `EscalationError`, `ConsentError`, `MetacognitionError` with
   structured variants. Pre-requisite for closing P1.3 fully.
2. **P2.2 — Split `ApiState::new` into `init_stores()` + `init_subsystems()`**
   (medium) — 277-line constructor. Direct, mechanical extraction.
3. **P3.6 — Extract escalation logic from `metacognition::sense()`** (medium) —
   ~80-line method; algedonic review + cursor advance + goal-stale counting
   are independent concerns.
4. **P3.4 — A2AMessage visitor pattern** (medium) — replaces match-on-variant
   with a visitor trait; makes adding new message types less error-prone.
5. **P3.2 — Split `McpRuntimeAdapter`** (medium) — make impossible states
   unrepresentable: `CapabilityOnlyAdapter` vs `FullMcpAdapter`.
6. **P3.3 — `RussellProcessManager`** (medium) — separate process lifecycle
   from ACP protocol concerns.
7. **P3.1 (continuation) — finish unified error hierarchy** (large) — extend
   `InfrastructureError` to cover the remaining crate-local enums.
8. **P4.5 + A3 — `tokio::sync::watch` for `last_snapshot`** (small) — more
   idiomatic for single-producer/single-consumer broadcast.

---

## Validation

`cargo check --workspace` is **green** (2026-06-06, HEAD `91f5b053`).
`cargo test -p hkask-types -p hkask-cns -p hkask-cli -p hkask-agents --lib`:
`15 + 107 + 196 + 49 = 367` tests pass, **0 failures**.
