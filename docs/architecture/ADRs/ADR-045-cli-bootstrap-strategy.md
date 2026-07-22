---
title: "ADR-045: CLI Bootstrap Strategy"
audience: [architects, developers]
last_updated: 2026-07-03
version: "0.31.0"
status: "Active"
domain: "Application"
mds_categories: [lifecycle]
---

# ADR-045: CLI Bootstrap Strategy

**Date:** 2026-07-03
**Status:** Active

## Context

During the v0.31.0 architecture audit, `hkask-cli` was identified as having
the highest fan-out in the crate graph (28+ internal dependencies). The audit
proposed extracting bootstrap/wiring logic into a separate bootstrap crate
(planned but never created) to reduce coupling.

Investigation revealed the bootstrap code in `main.rs` is thin:
- 70 lines of setup (dotenv, CLI parsing, logging, tokio runtime, registry init,
  platform manifest loading)
- 200 lines of command dispatch (thin routing to command handlers)

The REPL subsystem (`repl/init.rs`, 1,011 lines) contains the heavy wiring
(MCP server startup, service initialization, TUI bridge wiring).

**Problem Statement:** Should the CLI's assembly/wiring logic be extracted into
a separate bootstrap crate?

## Decision

**Chosen Approach:** Do NOT extract a bootstrap crate at this time.
The CLI's bootstrap code is not the source of its high dependency count.

**Rationale:**
- A hypothetical bootstrap crate (never created) would need the same 28+ dependencies as `hkask-cli`
  (it wires the same services), providing no compilation improvement
- The 70-line bootstrap in `main.rs` is already thin — extraction would trade
  70 lines for a new crate with identical dependencies
- The real wiring complexity (1,011 lines in `repl/init.rs`) is part of the
  REPL system, not a separable bootstrap concern

**Alternatives Considered:**
1. **Extract a bootstrap crate (planned, never created)** — Rejected. Would create a 28-dependency
   crate for 70 lines of code. No reduction in dependency count or compilation
   time.
2. **Extract `hkask-repl`** — Deferred. The 9,542-line REPL subsystem could
   be extracted into a standalone crate, reducing CLI's direct dependency on
   REPL-specific concerns. Left as future work; requires API stabilization
   on the REPL boundary.
3. **Extract TUI bridge implementations** — Deferred. `repl/tui_bridges.rs`
   (1,074 lines) could become a separate TUI bridge crate (planned, never
   created). Same dependency problem as bootstrap — needs TUI traits + all
   services.

## Consequences

### Positive
- No unnecessary crate proliferation
- Bootstrap code remains co-located with the entry point for discoverability
- CLI dependency count accurately reflects its role as the assembly point

### Negative
- CLI remains the highest fan-out crate in the graph
- Adding a new service requires touching CLI's Cargo.toml (but this is
  inherent to the assembly role)

### Neutral
- Future REPL extraction (when the REPL subsystem API stabilizes) would
  naturally reduce CLI's dependency count from 28+ to ~20

## Compliance

| Principle | Compliance | Evidence |
|-----------|-----------|----------|
| **P5** (Essentialism) | ✅ | Not creating a crate that would not reduce complexity |
| **P7** (Prefer deletion over deprecation) | ✅ | No code deleted; decision to not add new code |
| **C4** (Repetition is missing primitive) | ✅ | The 70-line bootstrap is not repetitive across crates |

## Verification

```bash
# Verify bootstrap is thin
wc -l crates/hkask-cli/src/main.rs
# Expected: ~311 lines (70 bootstrap, 200 dispatch, 40 boilerplate)

# Verify no separate bootstrap crate exists
find crates -maxdepth 1 -type d -name '*bootstrap*'
# Expected: no matches
```

## Related Documents

- [ADR-042: Port Promotion Rule](ADR-042-port-promotion-rule.md) — Rules for when to promote code to a separate crate
- [ADR-046: REPL Extraction Path](ADR-046-repl-extraction-path.md) — REPL subsystem extraction plan

## Extension: Context Service Fan-Out (2026-07-03)

The same bootstrap analysis was applied to `hkask-services-context` (18 deps).
Its sub-modules are too small to extract:

| Module | Lines | Deps | Extractable? |
|--------|-------|------|-------------|
| `reg.rs` | 59 | 3 | No — pass-through |
| `reg_store_slo_provider.rs` | 70 | 2 | No — pass-through |
| `governance.rs` | 289 | 7 | No — would create 7-dep crate for 289 lines |
| `infra.rs` | 73 | 7 | No — pass-through |
| `storage.rs` | 82 | 4 | No — pass-through |
| `context_impl/build/` | 1,359 | all 18 | No — this IS the DI container |

The context service's high fan-out is inherent to its role as the system's
orchestration layer. The `context_impl/build/` module is the dependency
injection container for the entire agent runtime — it legitimately needs
access to every service.
