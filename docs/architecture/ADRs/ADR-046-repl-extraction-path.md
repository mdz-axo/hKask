---
title: "ADR-046: REPL Extraction Path"
audience: [architects, developers]
last_updated: 2026-07-03
version: "0.31.0"
status: "Active"
domain: "Application"
mds_categories: [lifecycle]
---

# ADR-046: REPL Extraction Path

**Date:** 2026-07-03
**Status:** Active

## Context

During the v0.31.0 architecture audit, `hkask-cli/src/repl/` was identified as a
9,542-line subsystem with 12+ direct hKask crate imports. The REPL is the
primary source of CLI's 28-dependency fan-out. Extracting it into a standalone
`hkask-repl` crate would improve separation of concerns.

**Problem Statement:** How should the REPL subsystem be extracted from the CLI
crate to reduce coupling and enable independent testing?

## Decision

**Chosen Approach:** Implemented. The REPL was extracted into `hkask-repl` 
using a `ReplHost` trait to bridge cross-cutting concerns (WebID resolution,
onboarding, template listing, transcript viewing, sovereignty status).
The `CliHost` struct in `hkask-cli` implements the trait.

**Completed phases:**

### Phase 1: Stabilize REPL public API ✅
- Converted 109 `pub(crate)` items to `pub` across 23 files
- `ReplState`, `run()`, `run_tui()`, `TalkConfig`, `ManifestState` now public

### Phase 2: Extract `hkask-repl` crate ✅
- Moved all `repl/` files to `crates/hkask-repl/src/` (31 files)
- Created `hkask-repl/Cargo.toml` with 17 internal dependencies
- Designed `ReplHost` trait in `host.rs` bridging 5 CLI cross-cuts
- CLI's old `repl/` directory deleted (strangler fig complete)

### Phase 3: TUI bridges ✅
- `tui_bridges.rs` lives in `hkask-repl` (behind `tui` feature)
- Bridges implement traits from `hkask-repl` (tui module)
- `CliHost` implements `open_transcript_viewer` for the TUI feature

### Extraction plan (original estimate):

### Dependency reduction estimate

| Crate | Before | After |
|-------|--------|-------|
| `hkask-cli` | 28 deps | ~16 deps |
| `hkask-repl` (new) | — | ~12 deps |
| Separate REPL TUI bridge crate (deferred, not created) | — | ~2 deps |

## Consequences

### Positive (when extracted)
- REPL becomes independently testable
- CLI reduces from 28 to ~16 dependencies
- TUI bridges become independently testable with mock services
- Each layer has a single responsibility

### Negative
- Three-phase extraction is non-trivial (estimated 4 days)
- Risk of API drift during extraction
- Public API commitments for REPL session lifecycle

## Verification

The extraction is complete. The `crates/hkask-cli/src/repl/` directory no longer exists — all REPL code lives in `crates/hkask-repl/`. The original baseline command below is preserved for historical reference but will return zero results.

```bash
# Historical baseline (extraction complete — directory no longer exists)
# find crates/hkask-cli/src/repl -name "*.rs" | wc -l
# Expected: 0 (extraction complete)

# Current REPL size (authoritative)
find crates/hkask-repl/src -name "*.rs" | wc -l && \
  wc -l crates/hkask-repl/src/**/*.rs crates/hkask-repl/src/*.rs 2>/dev/null | tail -1
# Expected: ~40 files (15 src + 25 handlers), ~5,000 lines
```

The `ReplHost` trait in `crates/hkask-repl/src/host.rs` is implemented by `CliHost` in `crates/hkask-cli/src/repl_host.rs`. The `deps` module (`crates/hkask-repl/src/deps.rs`) exposes the turn-loop dependency injection traits (`TurnExecutor`, `GasGovernor`, `ToolInvoker`, `ThreadMemory`) for independent testability.

## Related Documents

- [ADR-045: CLI Bootstrap Strategy](ADR-045-cli-bootstrap-strategy.md) — Related CLI architecture decision
