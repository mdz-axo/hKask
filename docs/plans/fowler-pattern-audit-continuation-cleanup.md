# Fowler Pattern Audit Remediation — Cleanup, Documentation & Verification

## Session Purpose

The Fowler Pattern Audit remediation has progressed through P1 (7/7 ✅), P2 (8/8 ✅), P4.1 ✅, P4.2 ✅, and P4.3 ✅. The codebase compiles and all tests pass, but the work needs **cleanup, documentation, and verification** before P3 items begin.

Your job is to **audit the changes, fix any loose ends, update documentation, and verify constraint compliance**. You are NOT implementing P3 or P4.4/P4.5 — you are polishing what's already done.

---

## What Was Completed

### P1 (Complete — Prior Sessions)

| ID | Refactoring | Status |
|----|-------------|--------|
| P1.1 | Extract `verify_delegation_token()` + `VerificationOutcome` into `hkask-types/src/capability/verification.rs` | ✅ |
| P1.2 | `EscalationQueue` implements `Store` trait; `ConsentManager.load_from_store()` uses `lock_conn()`; `now_rfc3339()` helper added | ✅ |
| P1.3 | `From<DatabaseError>`, `From<EpisodicMemoryError>`, `From<SemanticMemoryError>` for `MemoryError`; 8 `.map_err()` calls replaced with `?` | ✅ |
| P1.4 | `in_memory_db()` helper in `hkask-storage/src/database.rs` and exported | ✅ |
| P1.5 | `StorageRequest` struct | ✅ |
| P1.6 | `DelegationAction::permits_write()/permits_read()`, `DelegationToken::allows_write()/allows_read()`, `require_write_access()`, `require_read_access()` in `hkask-types` | ✅ |
| P1.7 | Dead code removed: `Vault::new()` in keystore, `#[allow(dead_code)] health()` in exa.rs | ✅ |

### P2 (Complete — Prior Sessions + This Series)

| ID | Refactoring | Files | Status |
|----|-------------|-------|--------|
| P2.1 | Consolidate `AcpState` behind single lock | `hkask-agents/src/acp/mod.rs` | ✅ 5 `Arc<RwLock<...>>` → 1 `Arc<RwLock<AcpState>>` |
| P2.2 | Extract `Stores::init()` + `open_db()` from `ApiState::new()` | `hkask-api/src/lib.rs` | ✅ `Stores` struct with `init()`, `open_db()` returns `Result` |
| P2.3 | Domain newtypes for `GasCost`, `RBarThreshold`, `QueueDepth` | `hkask-cns/src/energy.rs` | ✅ Defined, `RBarThreshold` wired into `CurationConfidenceGate` |
| P2.4 | Template Method for `MemoryLoopAdapter` + `StorageRequest` struct | `hkask-agents/src/adapters/memory_loop_adapter.rs` | ✅ `triple_to_json()` helper, `StorageRequest` + `to_triple()` |
| P2.5 | Extract `github_api_url()` builder | `mcp-servers/hkask-mcp-github/src/main.rs` | ✅ |
| P2.6 | Extract `parse_webid()` API helper | `hkask-api/src/routes/acp.rs` | ✅ |
| P2.7 | Consolidate `MessageDispatch` priority queues | `hkask-agents/src/communication/dispatch.rs` | ✅ `HashMap<MessagePriority, VecDeque>` + single lock |
| P2.8 | Define token error constants | `hkask-types/src/capability/verification.rs` | ✅ `TOKEN_ERR_*` constants + `token_err_*()` helpers |

### P4 Items (Partially Complete)

| ID | Refactoring | Status |
|----|-------------|--------|
| P4.1 | Replace `.expect()` in production code with `Result` propagation | ✅ `ApiState::new()`, `Stores::init()`, `open_db()`, `build_loop_system()`, `with_defaults()`, `with_ensemble_inferencer()` all return `Result` |
| P4.2 | Use `AgentKind` methods instead of string literals | ✅ `AgentKind::as_russell_persona()` |
| P4.3 | Add `now_rfc3339()` helper | ✅ In `hkask-storage/src/store_macros.rs` |
| P4.4 | Audit `.to_string()` error conversions | ❌ Not started |
| P4.5 | Consider `tokio::sync::watch` for metacognition | ❌ Not started |

### Cross-cutting fix: `CuratorDirective` enum

The `LoopPayload::CurationDirective` variant was changed from a struct variant `{ directive_type, target, parameters }` to a tuple variant wrapping `CuratorDirective`. This introduced:
- `CuratorDirective` enum in `hkask-types/src/loops/curation.rs` with `variant_name()`, `agent_target()`, `is_metacognitive()` methods
- Updated `Dampener::should_dampen_directive()` to accept `&CuratorDirective`
- Updated `CyberneticsLoop::handle_curation_directive()` and `apply_directive()` to destructure `CuratorDirective` variants
- Updated `ApiError` with `Display` impl, `std::error::Error` impl, and `From<EscalationError>`, `From<GitError>`, and many other `From` impls

---

## Key Files Modified (Complete List Since Origin)

```
crates/hkask-agents/src/adapters/memory_loop_adapter.rs  — StorageRequest, triple_to_json(), require_*_access, ? operator
crates/hkask-agents/src/adapters/mcp_runtime.rs          — unified verify_delegation_token(), TOKEN_ERR_* constants
crates/hkask-agents/src/acp/mod.rs                      — AcpState consolidated, single lock
crates/hkask-agents/src/communication/dispatch.rs        — HashMap<MessagePriority, VecDeque> + single lock
crates/hkask-agents/src/consent.rs                       — lock_conn() usage
crates/hkask-agents/src/curator/curation_gate.rs         — RBarThreshold newtype for thresholds
crates/hkask-agents/src/escalation.rs                    — Store impl, lock_conn(), removed private now_rfc3339
crates/hkask-agents/src/error.rs                          — From impls for MemoryError
crates/hkask-agents/src/lib.rs                            — re-exports updated
crates/hkask-agents/src/loop_system.rs                    — lock poisoning in start()
crates/hkask-agents/src/registry_loader.rs                — uses now_rfc3339()
crates/hkask-agents/src/adapters/russell_acp.rs           — uses AgentKind::as_russell_persona()
crates/hkask-api/src/error.rs                             — ApiError with Display, Error impl, 12+ From impls
crates/hkask-api/src/lib.rs                              — Stores::init(), open_db() returns Result, build_loop_system returns Result
crates/hkask-api/src/routes/acp.rs                       — parse_webid() helper
crates/hkask-cns/src/cybernetics_loop.rs                  — CuratorDirective match, handle_curation_directive
crates/hkask-cns/src/dampener.rs                          — takes &CuratorDirective, is_metacognitive()
crates/hkask-cns/src/energy.rs                            — GasCost, RBarThreshold, QueueDepth newtypes
crates/hkask-cns/src/lib.rs                              — exports updated
crates/hkask-cli/src/commands/serve.rs                    — ApiState::new()? with Result
crates/hkask-storage/src/database.rs                      — in_memory_db() added
crates/hkask-storage/src/goals.rs                         — uses now_rfc3339()
crates/hkask-storage/src/lib.rs                           — in_memory_db, now_rfc3339 exported
crates/hkask-storage/src/nu_event_store.rs                — uses now_rfc3339()
crates/hkask-storage/src/standing_session.rs              — uses now_rfc3339()
crates/hkask-storage/src/store_macros.rs                  — now_rfc3339() defined
crates/hkask-storage/src/triples.rs                       — uses now_rfc3339()
crates/hkask-types/src/capability/mod.rs                  — DelegationAction methods, DelegationToken methods
crates/hkask-types/src/capability/verification.rs         — VerificationOutcome, verify_delegation_token(), require_*_access, TOKEN_ERR_*
crates/hkask-types/src/lib.rs                             — re-exports updated
crates/hkask-types/src/loops/curation.rs                  — CuratorDirective enum with methods
crates/hkask-types/src/loops/dispatch.rs                  — CurationDirective wraps CuratorDirective
crates/hkask-types/src/loops/mod.rs                      — CuratorDirective re-exports
mcp-servers/hkask-mcp-github/src/main.rs                  — github_api_url() builder
mcp-servers/hkask-mcp-keystore/src/main.rs               — dead code removed
mcp-servers/hkask-mcp-spec/src/main.rs                    — TOKEN_ERR_* constants
mcp-servers/hkask-mcp-web/src/providers/exa.rs            — dead code removed
```

---

## Your Tasks

### 1. Verification — Build & Test Matrix

Run the full verification suite and report any failures:

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -A clippy::type_complexity -D warnings
```

Report any warnings, errors, or test failures. Fix only what the audit changes introduced — do not fix pre-existing unrelated issues.

### 2. Constraint Compliance Check

Per `AGENTS.md`, these constraints are **non-negotiable**:

```bash
# No visual UI
if grep -r "grafana\|prometheus\|dashboard\|visual.*ui" crates/ --include="*.rs"; then echo "VIOLATION: Headless"; exit 1; fi

# No dead code / stubs / deprecated
if grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"; then echo "VIOLATION: P6/P7"; exit 1; fi

# No #[allow(dead_code)]
if grep -r "#\[allow(dead_code)\]" crates/ --include="*.rs"; then echo "VIOLATION: dead_code allow"; exit 1; fi
```

Run each check and report results.

### 3. Remaining `.expect()` Audit

The P4.1 goal was to replace `.expect()` in **production** code. Some remain:

**In `hkask-agents/src/pod/manager.rs`** (builder methods):
- L142: `.expect("In-memory storage initialization should never fail")` — `PodManager::new_mock`
- L272: `.expect("In-memory storage initialization should never fail")` — `with_in_memory_storage`
- L288: `.expect("Storage path must be valid UTF-8")` — `with_encrypted_storage`
- L291: `.expect("Encrypted storage initialization should succeed")` — `with_encrypted_storage`
- L308: `.expect("In-memory storage initialization should never fail")` — `build`

**In `hkask-agents/src/loop_system.rs`**:
- L257: `.expect("dispatch_rx lock poisoned during LoopSystem::start")`

**In `hkask-agents/src/acp/mod.rs`**:
- L412: `.expect("ACP secret not available...")` — `Default` impl

**In `hkask-agents/src/communication/dispatch.rs`**:
- L59: `.expect("dispatch queue initialized with all priorities")` — invariant assertion

**In `hkask-agents/src/pod/mod.rs`**:
- L167: `.expect("Default capability 'tool:execute' must always parse")` — static invariant

Categorize each:
- **Should fix now** → builder methods that should return `Result` (manager.rs L272, L288, L291, L308)
- **Acceptable invariant** → true invariant assertions that cannot fail in practice (dispatch.rs, pod/mod.rs)
- **Deliberate panic** → `Default` impls that deliberately panic on missing preconditions (acp/mod.rs)
- **Needs graceful handling** → lock poisoning (loop_system.rs)

Fix the "should fix now" items. Leave acceptable invariants and deliberate panics with a `// SAFETY:` comment explaining why the `.expect()` is valid.

### 4. Stale Comments & Doc Audit

Search for these patterns in the modified files and clean up:
- Comments that reference "P2.2" or "P1.5" or "P2.4" by ID number — these are internal tracking tags and should be removed or rewritten as plain English doc comments
- The old `/// Capture-common-parameters struct for memory storage operations (P2.4/P1.5).` double doc on `MemoryLoopAdapter` — remove the duplicate
- Any `// ── Persistent stores (P2.2: extracted init_stores) ──` style comments — keep the section headers but drop the P-number tags

### 5. Documentation Updates

**Update `docs/plans/fowler-pattern-audit-continuation-P3.md`**:
- Mark P4.1 as ✅ in the P4 items table
- Update the status of any findings that were resolved (e.g., `ApiState::new()` B1.1, `.expect()` removals)
- Add a note about remaining `.expect()` items that were categorized as acceptable

**Update `docs/refactoring/fowler-pattern-audit.md`** if needed:
- Mark resolved findings with ✅
- Ensure the findings table accurately reflects the current state

### 6. Import Hygiene

Check for unused imports introduced by the refactoring:
- `Database` was removed from `hkask-api/src/lib.rs` imports — verify nothing else references it
- `CuratorDirective` import in `dampener.rs` — the warning about unused import was mentioned; verify it's actually used
- Check for any `use` statements that became dead after the refactoring

### 7. `ApiError` Completeness Check

The `ApiError` type gained many `From` impls. Verify:
- Every `From` impl corresponds to an error type that actually appears in route handlers
- No `From` impl is dead code (check by temporarily removing each and seeing if it compiles)
- The `Display` impl matches the `IntoResponse` behavior (they should produce consistent messages)

### 8. `CuratorDirective` Completeness Check

The new `CuratorDirective` enum has these methods:
- `variant_name()` — returns `&'static str`
- `agent_target()` — returns `Option<WebID>`
- `is_metacognitive()` — returns `bool`

Verify these are used consistently in `cybernetics_loop.rs` and `dampener.rs`. Check if `apply_directive` in `cybernetics_loop.rs` covers all `CuratorDirective` variants — if there's a new variant not handled, flag it.

---

## Design Constraints (from AGENTS.md)

- **No visual UI** — CLI/MCP/API only
- **No monitoring stacks** — CNS provides programmatic observability
- **No excess complexity** — No unused traits, stubs, `#[allow(dead_code)]`, feature flags

---

## Build Verification Command

```bash
cargo check --workspace && cargo test --workspace
```

---

*Status: P1 (7/7 ✅), P2 (8/8 ✅), P4.1 ✅, P4.2 ✅, P4.3 ✅. Remaining: P3 (0/6), P4.4, P4.5.*