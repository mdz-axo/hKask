# hKask Remediation Session — Handoff

**Date:** 2026-06-05
**Status:** Partial — 3 of 8 fixes applied, git working tree has pre-existing dirty files

---

## What Was Done (Applied Successfully)

### 1. `Goal::transition()` — Now validates state machine (P0 bug fix)
**File:** `crates/hkask-types/src/goal.rs`
- `Goal::transition()` was a void method that assigned any state without validation, bypassing `can_transition_to()`. The persistence layer validated, but in-memory mutations were unchecked.
- **Fix:** Changed `transition()` to return `Result<(), IllegalGoalTransition>`, validating via `can_transition_to()` before mutating. Added `IllegalGoalTransition` error type with `Display` and `Error` impls.
- Convenience methods (`activate`, `complete`, `block`, `abandon`) now use `let _ = self.transition(...)` — they silently ignore errors for backward-compat call sites that only transition to legal states.
- **Exported** `IllegalGoalTransition` from `hkask_types` via `lib.rs`.
- `cargo check --workspace` passes. `cargo clippy` has pre-existing errors in `hkask-cns` (not caused by this change).

### 2. OCAP server — Removed duplicate HMAC secret (P0 security)
**File:** `mcp-servers/hkask-mcp-ocap/src/main.rs`
- `OcapServer` stored the same HMAC secret twice: once inside `CapabilityChecker` and once as `self.secret: Zeroizing<Vec<u8>>`. Two copies of a cryptographic secret can drift; `verify()` used one copy, `DelegationToken::new()` used the other.
- **Fix:** Removed `secret` field from `OcapServer`. `ocap_delegate` now uses `self.checker.grant_tool()` instead of `DelegationToken::new()`. `ocap_verify` already used `self.checker.verify()`.
- Removed duplicate `parse_capability()` method — replaced with direct `CapabilitySpec::parse()` calls with proper error mapping.
- Removed unused `DelegationResource`, `DelegationAction` imports and `Zeroizing` import.
- **Borrow fix:** `capabilities` was moved into `grant_tool()` then used again in JSON response. Added `let capabilities_str = capabilities.clone()` before the move.
- `cargo check --workspace` passes.

### 3. Clippy fix — Misleading doc comment on `caveat_ids()`
**File:** `crates/hkask-types/src/capability/mod.rs`
- The doc comment above `caveat_ids()` described `add_caveat()` instead, causing `empty_line_after_doc_comments` clippy error.
- **Fix:** Replaced the misleading 8-line doc comment with an accurate one-liner: `/// Return the caveat IDs attached to this token.`

---

## What Was NOT Done (Not Yet Applied)

### 4. CnsServer.threshold shadow removal
**File:** `mcp-servers/hkask-mcp-cns/src/main.rs`
- I made the edit locally but it was lost in a git stash conflict. The plan was:
  - Remove `threshold: AtomicU64` field from `CnsServer`
  - Add `CnsRuntime::default_threshold()` getter (I did add this to `crates/hkask-cns/src/runtime.rs` — it may or may not have survived)
  - Replace `self.threshold.load(Ordering::Relaxed)` with `self.runtime.default_threshold()`
  - Remove `self.threshold.store(new_threshold, Ordering::Relaxed)` from `cns_calibrate`
  - Remove `use std::sync::atomic::{AtomicU64, Ordering}` import

### 5. AuditLog dead `store` field removal
**File:** `crates/hkask-agents/src/acp/audit.rs`
- Remove `store: Option<Arc<dyn AuditLogPort>>` from `AuditLog` struct
- Remove `if let Some(ref store) = self.store { store.log(entry.clone()); }` from `log()` method
- Remove `store: None` from `new()`
- Remove `pub use hkask_types::AuditLogPort` from `audit.rs`
- Remove `pub use hkask_types::AuditLogPort` from `acp/mod.rs`
- Remove `AuditLogPort` from `ports/audit_log.rs` re-export
- Remove `AuditLogPort` from `ports/mod.rs` re-export

### 6. `HkaskError::CapabilityDenied` / `PermissionDenied` dedup — NOT STARTED
- These two variants are semantically identical (both map to `McpErrorKind::PermissionDenied`). Would require updating all call sites.

### 7. Database opening helper — NOT STARTED
- 20+ call sites each independently do `match db_config { ... Database::open() ... }`. Would add `open_database_or_in_memory()` to `hkask-storage`.

### 8. Memory infrastructure factory — NOT STARTED
- 6-line `TripleStore + EpisodicMemory + SemanticMemory` wiring block copy-pasted 6+ times. Would add `MemoryInfra::from_database()` to `hkask-memory`.

---

## Git Working Tree State

The working tree has **pre-existing uncommitted changes** that were NOT made by this session:

```
M crates/hkask-agents/src/adapters/mcp_runtime.rs   (84 lines)
M crates/hkask-agents/src/escalation.rs               (66 lines)
M crates/hkask-storage/src/database.rs                (12 lines)
M crates/hkask-storage/src/lib.rs                     (2 lines)
M mcp-servers/hkask-mcp-keystore/src/main.rs          (8 lines)
M mcp-servers/hkask-mcp-spec/src/main.rs             (2 lines)
M mcp-servers/hkask-mcp-web/src/providers/exa.rs     (5 lines)
```

**My changes** (apply cleanly on top of origin/main):
- `crates/hkask-types/src/goal.rs` — IllegalGoalTransition + transition validation
- `crates/hkask-types/src/lib.rs` — Export IllegalGoalTransition
- `crates/hkask-types/src/capability/mod.rs` — Fixed misleading doc comment on caveat_ids()
- `mcp-servers/hkask-mcp-ocap/src/main.rs` — Removed duplicate secret, deduped parse_capability

**⚠️ DO NOT `git stash` or `git checkout` on the pre-existing dirty files** — they contain other work. My changes should be applied as a commit on top, or the diffs extracted manually.

---

## Pre-existing Clippy Errors (NOT caused by my changes)

`cargo clippy --workspace -- -D warnings` fails on `hkask-cns` with 7 errors:
1. `empty_line_after_doc_comments` (1 occurrence)
2. `redundant_closure` (1)
3. `map_or` simplification (1)
4. `returning let binding from block` (3)
5. `if` statement can be collapsed (1)

These existed before my session started.

---

## Key Findings from Assessment (for reference)

| Priority | Finding | Status |
|----------|---------|--------|
| P0 | `Goal::transition()` bypassed state machine | ✅ Fixed |
| P0 | OcapServer duplicate HMAC secret | ✅ Fixed |
| P0 | Clippy: empty_line_after_doc_comments | ✅ Fixed |
| P0 | `parse_capability()` duplicated in 2 crates | ✅ Fixed (ocap only) |
| P1 | CnsServer.threshold shadows CnsRuntime | ❌ Edit lost |
| P1 | AuditLog dead `store` field | ❌ Not applied |
| P1 | `CapabilityDenied` defined 7 times across 4 crates | ❌ Not started |
| P1 | 0 MCP server tests, 0 integration tests | ❌ Not started |
| P1 | `HkaskError` legacy wrapper | ❌ Not started |
| P1 | `SovereigntyPort` single pass-through impl | ❌ Not started (decided to keep — docs reference it) |
| P2 | Database opening copy-pasted 20+ times | ❌ Not started |
| P2 | Memory wiring copy-pasted 6+ times | ❌ Not started |
| P2 | NuEventSink creates independent in-memory DBs | ❌ Not started (documented only) |