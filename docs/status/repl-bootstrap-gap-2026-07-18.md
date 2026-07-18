---
title: "REPL Bootstrap Gap Post-Mortem — 2026-07-18"
audience: [developers, administrators]
last_updated: 2026-07-18
version: "0.31.0"
status: "Active"
domain: "Core"
mds_categories: [lifecycle, trust, composition]
---

# REPL Bootstrap Gap Post-Mortem

**Date:** 2026-07-18
**Severity:** Critical (system was non-functional for tool-augmented chat)
**Duration:** ~3 hours (diagnosis to fix)
**Root cause:** `tool_calls` field was on `ChatChoice` instead of `ChatResponseMessage`

## Summary

The hKask REPL tool-use loop was terminating after a single inference call
instead of iterating when the model requested tools. The model returned
`finish_reason=tool_calls` but the `tool_calls` array was silently dropped
during deserialization, causing the loop to see zero tool calls and break.

Compounding this were four bootstrap gaps that made diagnosis difficult:
missing daemon, missing UserStore session, wrong keychain key, and wrong
`.env` passphrase.

## Timeline

1. **Symptom observed:** User reported the REPL stalled after one tool call
   with 142 completion tokens and a prose preamble ("Let me start by
   exploring...").
2. **Initial diagnosis (wrong):** Attributed to the model not emitting
   `<<tool:...>>` directives. This was the proximate cause but not the root.
3. **Bootstrap gap discovered:** The daemon wasn't running. `kask daemon
   status` reported "running" because it only checked file existence, not
   socket liveness.
4. **Passphrase mismatch discovered:** The `.env` file had
   `HKASK_DB_PASSPHRASE=hkask-default-passphrase-2024` but the actual DB was
   encrypted with `allostery`.
5. **Keychain key bug discovered:** `kask init` stored the master passphrase
   under `KEY_MASTER_KEY` ("HKASK_MASTER_KEY") but `resolve_db_passphrase()`
   looked for `KEY_DB_PASSPHRASE` ("hkask-db-passphrase"). The keychain
   lookup never found it.
6. **Mixed passphrases discovered:** The curator agent's per-agent DBs were
   encrypted with a different passphrase than the main DB. This was caused
   by the daemon falling back to in-memory config and re-encrypting DBs.
7. **Root cause found:** The `tool_calls` field was on `ChatChoice` instead
   of `ChatResponseMessage`. The OpenAI Chat Completions API spec puts
   `tool_calls` on the `message` object, not the `choice` object. KiloCode
   (OpenAI-compatible) was returning `finish_reason=tool_calls` with the
   `tool_calls` array on `message.tool_calls`, but hKask was looking for it
   on `choice.tool_calls` — so it deserialized as `None`.
8. **Fix applied:** Moved `tool_calls` from `ChatChoice` to
   `ChatResponseMessage` (non-streaming) and from `StreamChoice` to
   `StreamDelta` (streaming). Verified the tool-use loop now iterates.

## Root Cause

`crates/hkask-inference/src/chat_protocol.rs` had this struct layout:

```rust
// WRONG — tool_calls on ChatChoice
pub struct ChatChoice {
    pub message: ChatResponseMessage,
    pub finish_reason: String,
    pub tool_calls: Option<Vec<RawToolCall>>,  // ← should be on message
}

pub struct ChatResponseMessage {
    pub role: String,
    pub content: String,
    // ← missing tool_calls
}
```

The OpenAI Chat Completions API spec puts `tool_calls` at
`choices[0].message.tool_calls`, not `choices[0].tool_calls`. When
KiloCode returned a response with `tool_calls` on the message, serde
silently ignored it (unknown field on `ChatResponseMessage`) and the
`ChatChoice.tool_calls` field deserialized as `None`.

The fix:

```rust
// CORRECT — tool_calls on ChatResponseMessage per OpenAI spec
pub struct ChatChoice {
    pub message: ChatResponseMessage,
    pub finish_reason: String,
}

pub struct ChatResponseMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<RawToolCall>>,  // ← per spec
}
```

## Bootstrap Gaps Fixed

1. **`kask init` keychain key bug:** Was storing the master passphrase under
   `KEY_MASTER_KEY` (reserved for the derived master key hex). Now stores
   under both `KEY_DB_PASSPHRASE` and `KEY_MASTER_PASSPHRASE`.

2. **`kask daemon status` false positive:** Was checking file existence only.
   Now pings the socket by sending a sentinel `auth_query` and verifying a
   valid JSON response.

3. **`run_onboarding` operating mode missing session:** Was returning the
   agent name without calling `UserStore::login()`. Now creates a session
   via `create_user_session()` and resolves secrets from the keychain via
   `resolve_secrets_from_keychain()`.

4. **`kask chat` missing daemon auto-start:** Was assuming the daemon was
   already running. Now auto-starts it in Phase 7.5 of REPL init via
   `ensure_daemon_running()`, with proper process detachment and stderr
   capture.

## What Would Have Prevented This

1. **A contract test for OpenAI response deserialization** that includes a
   `tool_calls` field on the `message` object. The existing test
   (`chat_response_deserializes_openai_format`) only tested a simple
   text response with no tool calls.

2. **A `kask doctor --bootstrap` command** that checks the full chain:
   daemon running, socket live, keychain entries present, DB passphrase
   correct, session exists, MCP servers connect.

3. **The getting-started doc including the daemon + session bootstrap steps**
   instead of going straight from `kask init` to `kask chat`.

## Remaining Open Issues

1. **Fusion orchestrator discards tool_calls** — `algo_merge` and
   `algo_vote` hardcode `tool_calls: Vec::new()`. Not a regression (chat
   bypasses fusion) but a latent bug.

2. **`UserStore` vs `AgentRegistryStore` mismatch** — onboarding writes to
   `agent_registry`, daemon checks `replicant_identities`. The
   `create_user_session` workaround fails with "replicant not found" for
   replicants created via onboarding.

3. **Keychain entries disappear between sessions** — root cause unknown.
   The HKDF fallback in `resolve_secrets_from_keychain` mitigates this.

4. **Daemon in-memory fallback can corrupt disk DBs** — when the daemon
   can't resolve the DB passphrase, it falls back to in-memory config but
   still touches disk databases.

5. **`supply-chain-sentinel` test failure** — the skill manifest references
   template files that don't exist.

## Files Changed

- `crates/hkask-inference/src/chat_protocol.rs` — tool_calls deserialization fix
- `crates/hkask-mcp/src/daemon/mod.rs` — shared `ping_daemon` + `DaemonPingError`
- `crates/hkask-cli/src/commands/daemon.rs` — socket-pinging status, removed duplicate
- `crates/hkask-cli/src/commands/init.rs` — correct keychain keys
- `crates/hkask-cli/src/onboarding.rs` — session creation + keychain resolution
- `crates/hkask-repl/src/init.rs` — daemon auto-start (Phase 7.5)
- `docs/how-to/getting-started.md` — bootstrap steps

## Verification

- `cargo build --workspace` — pass
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo fmt --check` — pass
- `cargo test --workspace` — 222 pass, 1 pre-existing failure (supply-chain-sentinel)
- All CI invariant scripts pass
- Functional: `kask chat` tool-use loop iterates with `codegraph_structure` tool
