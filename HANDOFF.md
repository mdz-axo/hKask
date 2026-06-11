# Handoff — hKask Core Crate Audit & Onboarding Fixes

**Date:** 2026-06-11
**Version:** 0.27.0
**Status:** Onboarding fixes complete. 50 findings from core crate scan ready for triage.

---

## 1. Session Context

This session reviewed the hKask onboarding flow and first-replicant creation against the Magna Carta design principles, fixed 4 bugs (ACP WebID mismatch, ReplSettings overwrite, onboarding path collapse, namespace inconsistency), then performed a systematic code scan across all 9 core crates. The scan produced 50 findings (18 must-fix, 21 should-fix, 11 cleanup). All onboarding fixes compile clean and pass tests. The scan findings are untouched — no code changes beyond the 4 onboarding fixes.

---

## 2. What Was Done

### Onboarding Fixes (all verified: `cargo clippy -D warnings`, `cargo test`)

- **ACP WebID namespace mismatch** — `crates/hkask-services/src/onboarding.rs` L84: Changed ACP state restoration from `WebID::from_persona()` (namespace `"hkask"`) to `WebID::from_persona_with_namespace(..., "replicant")`. Removed orphaned `use std::str::FromStr`.
- **ReplSettings overwrite** — `crates/hkask-cli/src/repl/init.rs` L221: Removed `let repl_settings = ReplSettings::default();` that shadowed the loaded settings from `~/.config/hkask/settings.json`. Removed orphaned `ReplSettings` import.
- **Onboarding path collapse** — `crates/hkask-cli/src/onboarding.rs`: Replaced three-scenario branching with two modes: **operating** (keys + replicants exist → return) and **setup** (create first replicant). Removed dead code: `sign_in_flow`, `try_list_existing_replicants`, `pick_or_default_replicant`, `InvalidPassphrase` variant. Renamed module doc and functions.
- **`ReplicantIdentity::derive_webid` namespace** — `crates/hkask-types/src/identity.rs` L61: Changed from `"hkask-replicant"` to `"replicant"`.

### Core Crate Scan

Full scan of `hkask-types`, `hkask-storage`, `hkask-agents`, `hkask-services`, `hkask-cns`, `hkask-memory`, `hkask-keystore`, `hkask-templates`, `hkask-mcp`. Findings documented below in §3.

---

## 3. What Remains

### HIGH — Production Panics (fix first, these crash the binary)

| # | Crate | Location | Bug |
|---|-------|----------|-----|
| 1 | `hkask-agents` | `curator/persona_filter.rs:32-38` | Byte-position mismatch after `to_lowercase()` — panics on non-ASCII input. Same bug at L63-67. |
| 2 | `hkask-agents` | `pod/mod.rs:192-199` | `.expect()` on user-supplied capability string, not the default. Panics on malformed input. |
| 3 | `hkask-storage` | `nu_event_store.rs:262` | `span_path[namespace.len() + 1..]` with no bounds check. Panics when fallback namespace doesn't match. |
| 4 | `hkask-memory` | `semantic.rs:189-197` | `compute_centroid` indexes `centroid[i]` without checking `emb.vector.len() == dim`. OOB panic. |

**Strategy:** Each is a 1-3 line fix (bounds check or char-boundary-safe slicing). Fix, add a regression test per fix, verify with `cargo test -p <crate>`.

### HIGH — Security (SSRF bypass, path traversal)

| # | Crate | Location | Bug |
|---|-------|----------|-----|
| 5 | `hkask-mcp` | `security.rs:95-98` | IPv6 ULA check only matches `fc00::` and `fd00::`, misses rest of `fc00::/7`. Fix: `(segments[0] & 0xfe00) == 0xfc00`. |
| 6 | `hkask-mcp` | `security.rs:86-100` | Missing IPv6 link-local (`fe80::/10`) check entirely. |
| 7 | `hkask-storage` | `security.rs:25-32` | `sanitize_path` bypass when parent dir doesn't exist — `canonicalize()` fails, check skipped. |

**Strategy:** Fix the bitmask, add `fe80::/10` check, make `sanitize_path` fail-closed when canonicalize fails. Add test cases for each bypass vector.

### HIGH — Data Corruption / Silent Loss

| # | Crate | Location | Bug |
|---|-------|----------|-----|
| 8 | `hkask-agents` | `consent.rs:92-101` | `to_stored()` generates new UUID per call — every `grant_consent` INSERTs a new row. Unbounded growth. |
| 9 | `hkask-storage` | `triples.rs:138-191` | Non-atomic triple update (UPDATE + SELECT + INSERT without transaction). Crash = data loss. |
| 10 | `hkask-cns` | `energy.rs:238,244,256` | `remaining + rate` uses `+` not `saturating_add`. Wraps u64 in release, budget shrinks. |
| 11 | `hkask-keystore` | `keychain.rs:53-55,84-86` | All keyring errors mapped to `NotFound`. Permission denied → generates new secret → overwrites real one. |

**Strategy:** #8 needs upsert logic or lookup-before-insert. #9 needs a transaction wrapper. #10 is a one-word fix (`saturating_add`). #11 needs proper error variant mapping using the existing `From<KeyringError>` impl.

### HIGH — Project Violation

| # | Crate | Location | Bug |
|---|-------|----------|-----|
| 12 | `hkask-agents` | `adapters/mcp_runtime.rs:266-269` | `#[deprecated]` attribute on `McpRuntimeAdapter`. P6/P7 violation. Delete the type alias and the `#[allow(deprecated)]` at `adapters/mod.rs:11`. |

**Strategy:** Grep for callers of `McpRuntimeAdapter`, replace with `FullMcpAdapter`, delete the alias.

### HIGH — Broken Logic

| # | Crate | Location | Bug |
|---|-------|----------|-----|
| 13 | `hkask-types` | `cns.rs:96` | `RetryConfig::delay_for_attempt` truncates `f64` multiplier to `u64` before exponentiation. Backoff broken for non-integer multipliers. |
| 14 | `hkask-services` | `context.rs:612` + `curator.rs:111` | Curator metacognition uses detached/fresh CNS runtime — always sees zero alerts, zero variety. |
| 15 | `hkask-services` | `goal.rs:116-121` | `set_goal_state` returns empty `text` and `visibility` despite having the fetched goal. |
| 16 | `hkask-memory` | `consolidation.rs:185-189` | `consolidation_candidate_count` calls `storage_usage()` (total count) not candidate count. |
| 17 | `hkask-keystore` | `master_key.rs:45-55` | `InternalSecrets` derives `Debug` — key material in logs/panics. Custom `Debug` impl needed. |
| 18 | `hkask-mcp` | `runtime.rs:117,156` | TOCTOU race in `start_server` — concurrent calls orphan child processes. |

### MEDIUM — Swallowed Errors (21 instances across 5 crates)

The most impactful cluster:
- `hkask-storage`: 5 locations map all DB errors to `NotFound`, 7 locations silently corrupt timestamps with `Utc::now()`.
- `hkask-templates`: 6 `.ok()` calls on INSERT/DELETE — silent data loss on restart.
- `hkask-services`: `sovereignty.rs` returns `ConsentError` instead of `ServiceError`.
- `hkask-agents`: `escalation.rs` `resolve()`/`dismiss()` don't check `rows_affected`.

**Strategy:** Work crate-by-crate. Start with `hkask-storage` (most impactful), then `hkask-templates`, then `hkask-agents`.

### LOW — Cleanup (11 items)

Dead code, naming inconsistencies, empty test modules. See scan results in conversation history for full list. Non-urgent but accumulates debt.

---

## 4. Recommended Skills and Tools

| Skill | When |
|-------|------|
| `coding-guidelines` | Before every fix — surgical changes, simplicity first |
| `tdd` | For panic fixes (#1-4) — write failing test first, then fix |
| `diagnose` | For #14 (curator CNS) — needs tracing to confirm the detached runtime theory |

```bash
# Verify after each fix
cargo check -p <crate>
cargo clippy -p <crate> -- -D warnings
cargo test -p <crate>

# Full workspace gate before any PR
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace

# Project constraint verification
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"
```

---

## 5. Key Decisions to Preserve

1. **"Operating mode" vs "setup" — not "fast path" vs "interactive".** The onboarding module has two states: setup (zero replicants → create first) and operating (replicants exist → sign in). There is no "fast path" — that name was misleading. The operating mode returns immediately when keys work and replicants exist; setup runs the full creation flow.

2. **WebID namespace is `"replicant"` everywhere.** Registration, ACP restoration, chat, REPL init, and `ReplicantIdentity::derive_webid` all use `"replicant"`. The old `"hkask-replicant"` and `"hkask"` namespaces were bugs. Any new code creating replicant WebIDs must use `"replicant"`.

3. **ReplSettings loads from disk once at REPL init.** The loaded settings are the canonical source for the session. No re-initialization with defaults after load. The `repl_settings` variable from `load_settings()` flows into `ReplState` and is mutable via `/repl` during the session.

4. **Dead code from removed paths was deleted, not commented out.** `sign_in_flow`, `try_list_existing_replicants`, `pick_or_default_replicant`, `InvalidPassphrase` — all removed. Recovery/sign-in is a future feature, not dead code to preserve.

5. **Scan findings are reports, not prescriptions.** The 50 findings describe what exists. The agent doing the fixes should verify each finding before changing code — some may have context not visible in the scan (e.g., the "dead" `TurnRequest` fields may be planned for imminent use).

---

*Recommended work order: #1-4 (panics) → #12 (violation) → #5-7 (security) → #8-11 (data corruption) → #13-18 (logic) → MEDIUM → LOW*
