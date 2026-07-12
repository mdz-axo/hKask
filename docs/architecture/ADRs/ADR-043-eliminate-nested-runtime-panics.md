---
title: "ADR-043: Eliminate Nested Runtime Panics in CLI Context Construction"
audience: [architects, developers]
last_updated: 2026-07-02
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# ADR-043: Eliminate Nested Runtime Panics in CLI Context Construction

**Date:** 2026-07-02
**Status:** Active

## Context

The `kask chat` startup path panicked with `Cannot start a runtime from within a runtime`
when a prior failed setup left an orphaned database with a passphrase mismatch. The root
cause was `build_service_context_inner()` in `helpers.rs`, which detected an active tokio
runtime via `Handle::try_current()` and then called `Handle::block_on(rx)` — a call that
tokio explicitly forbids from within a worker thread or `block_on` context.

**Problem Statement:** CLI command handlers and REPL initialization both construct
`AgentService` instances, but the construction path is called from two fundamentally
incompatible contexts: outside any runtime (one-shot CLI) and inside `rt.block_on`
(REPL init and REPL turn handlers).

**Stakeholders:** CLI developers, REPL users, anyone calling `kask chat` after a failed
prior setup.

**Constraints:**
- Multiple call sites (~40) across 17 files in `crates/hkask-cli/src/commands/`
- Chat path needs graceful error handling (maps to `ChatTurnResponse`)
- One-shot CLI commands have no recovery path (must exit on failure)
- Cannot change the `AgentService::build()` constructor (it's in `hkask-services-context`)

## Decision

**Chosen Approach:** Replace the three `build_service_context*` functions with a layered
helper architecture in `helpers.rs`:

```
build_agent_service_inner(from_secrets) → Result<AgentService, String>  (private)
    ├── Uses block_in_place when inside a runtime, Runtime::new() otherwise
    │
build_agent_service() → AgentService                                 (public, panics)
    └── Wraps build_agent_service_inner(None) with or_exit
    │
build_agent_service_from_secrets(from_secrets) → Result<...>        (public, for chat)
    └── Passes through to build_agent_service_inner
```

The key fix: `build_agent_service_inner` detects whether it's inside a tokio runtime
via `Handle::try_current()`. When inside, it uses `tokio::task::block_in_place` to move
the blocking `Runtime::new().block_on(...)` call to the blocking thread pool. When
outside, it creates a fresh runtime and calls `block_on` normally.

Additionally, REPL handlers (agent, escalation, pods) were refactored to use the
existing `ReplState.service_context: Arc<AgentService>` directly rather than
constructing a new `AgentService` per command.

**Alternatives Considered:**
1. **Spawn on separate OS thread** — Correct but creates a thread per call. Rejected:
   `block_in_place` achieves the same result using tokio's existing thread pool.
2. **Make all callers accept `&AgentService`** — Would require threading context
   through ~40 call sites and `main()`. Rejected as too invasive for the scope.
3. **Delete the helpers entirely, build in `main()`** — Infeasible because the chat
   path needs onboarding secrets that aren't available until onboarding completes.

**Rationale:** The layered approach preserves backward compatibility (all existing
callers work unchanged), fixes the panic (via `block_in_place`), and provides a
clean `Result`-returning path for callers that need graceful error handling.

## Consequences

### Positive
- `kask chat` startup no longer panics with nested runtime errors
- Single implementation (`build_agent_service_inner`) shared by all paths
- REPL handlers avoid redundant `AgentService` construction (use `ReplState`)
- Regression test added: `build_agent_service_from_within_block_on`
- Deleted the bug-prone `Handle::block_on` pattern entirely

### Negative
- `build_agent_service()` panics via `or_exit` — callers must know this
- `block_in_place` requires a multi-threaded runtime (satisfied by hkask's setup)
- Two public wrappers with different error contracts could confuse new callers

### Neutral
- `block_on!` macro added to `lib.rs` — reconstructed from call-site patterns
  because the original definition site could not be located
- `init_repl_state` safety comments updated to reflect actual thread model

## Compliance

### Constraint-Driven Design Principles

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P1** (No trait without two consumers) | ✅ | No new traits introduced |
| **P2** (No generic without two instantiations) | ✅ | `print_item_list<T>` has multiple callers |
| **P3** (No module directory without encapsulation) | ✅ | `build_agent_service_inner` is private |
| **P7** (Prefer deletion over deprecation) | ✅ | `build_service_context_inner` deleted, not deprecated |

### Constraints

| Constraint | Compliance | Evidence |
|-----------|-----------|----------|
| **C4** (Repetition is missing primitive) | ✅ | Three `build_service_context*` functions merged into one inner + two public wrappers |
| **C7** (Divergence must yield) | ✅ | REPL path now uses `state.service_context` directly, CLI path uses helpers — no divergence in context construction |

## Verification

```bash
# Verify no build_service_context references remain
grep -rn "build_service_context" crates/hkask-cli/src/ --include="*.rs"
# Expected: only historical references in comments/docstrings

# Verify the regression test passes
cargo test -p hkask-cli --lib -- helpers::tests
# Expected: build_agent_service_from_within_block_on passes

# Full test suite
cargo test -p hkask-cli --lib
# Expected: 57 passed
```

## Related Documents

- `crates/hkask-cli/src/commands/helpers.rs` — The implementation
- `crates/hkask-cli/src/repl/init.rs` — REPL initialization that triggers the original bug
- `crates/hkask-cli/src/repl/handlers/` — REPL handlers refactored to use `ReplState`
- [Tokio documentation: `block_in_place`](https://docs.rs/tokio/latest/tokio/task/fn.block_in_place.html)
