---
title: "ADR-042: Port Trait Location — Promotion Rule"
audience: [architects, developers]
last_updated: 2026-06-30
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [curation]
---

# ADR-042: Port Trait Location — Promotion Rule

**Date:** 2026-06-30
**Status:** Active
**Supersedes:** None (codifies existing practice)

## Context

hKask uses hexagonal architecture with port traits defining boundaries between
domain logic and adapters. Port traits historically accumulated in `hkask-ports`
without a clear criterion for when a trait should live in a shared crate versus
its domain crate.

**Problem Statement:** Three port traits (`EpisodicStoragePort`,
`SemanticStoragePort`, `MCPRuntimePort`) lived in `hkask-agents/src/ports/`
rather than `hkask-ports`, forcing any crate needing memory storage ports to
depend on the entire `hkask-agents` crate.

**Stakeholders:** Service layer crates, adapter authors, crate maintainers.

**Constraints:**
- P5 (Essentialism): crates must earn their existence
- P7 (Evolutionary Architecture): types emerge from usage, not speculation
- Deep-module discipline (≤7 public items target)

## Decision

**Chosen Approach: Port traits live in the domain crate that first consumes
them. When a second consumer needs the trait, it is promoted to the crate
nearest to both consumers — either the natural domain crate or `hkask-ports`.**

The `hkask-ports` crate serves as the **promotion graveyard** — it only
contains traits that are consumed by at least two crates and don't have a
natural domain home closer to both consumers.

### Rule

| Consumers | Location |
|-----------|----------|
| 1 crate | Lives in consumer's crate |
| 2+ crates, shared domain | Lives in domain crate (e.g., `hkask-memory` for memory ports) |
| 2+ crates, cross-domain | Lives in `hkask-ports` (promotion graveyard) |

### Applied to current ports

| Trait | Consumers | Verdict | Location |
|-------|-----------|---------|----------|
| `EpisodicStoragePort` | `hkask-agents` + `hkask-services-context` | 2 consumers, memory domain | `hkask-memory` |
| `SemanticStoragePort` | `hkask-agents` + `hkask-services-context` | 2 consumers, memory domain | `hkask-memory` |
| `MCPRuntimePort` | `hkask-agents` | 1 consumer | Stays in `hkask-agents` |
| `InferencePort` | `hkask-services-chat` + `hkask-services-runtime` + `hkask-regulation` | 3 consumers, cross-domain | `hkask-ports` |
| `CnsObserver` | `hkask-regulation` + `hkask-services-runtime` | 2 consumers, cross-domain | `hkask-ports` |

### New type: `MemoryPortError`

The port traits now use `hkask_memory::MemoryPortError` instead of
`crate::error::MemoryError` (which lives in `hkask-agents`). This is a minimal
error type with two variants: `Storage(String)` and
`CapabilityDenied { resource, action }`. Conversion impls bridge between
`MemoryError` (internal to `hkask-agents`) and `MemoryPortError` (port boundary).

### Backward compatibility

`hkask-agents` re-exports the promoted traits from `ports/memory_storage.rs`
as a thin re-export shim. Existing `use hkask_agents::EpisodicStoragePort`
continues to work. New code should import from `hkask_memory::ports`.

**Alternatives Considered:**

1. **Move all three to `hkask-ports`** — Rejected: `MCPRuntimePort` has a single
   consumer; premature promotion violates P7. The current `hkask-ports` crate
   already has 13 traits that could benefit from the promotion rule.

2. **Leave everything where it is** — Rejected: `hkask-services-context`
   depends on `hkask-agents` solely for these two port traits. Evolving the
   architecture is the principled response.

3. **Define ports in `hkask-types`** — Rejected: Ports are behavioral
   interfaces, `hkask-types` is a passive data crate. Mixing them violates
   separation of concerns.

**Rationale:** Co-locating ports with their domain logic (memory ports in
`hkask-memory`, inference ports in `hkask-ports`) improves locality — the port
definition, its domain implementations, and its tests live in one crate. The
promotion threshold ensures `hkask-ports` stays small and only contains
genuinely cross-cutting abstractions.

## Consequences

**Positive:**
- `hkask-services-context` no longer imports memory port traits from
  `hkask-agents` (they come from `hkask-memory` which it already depends on)
- `hkask-ports` has a clear membership criterion — no more accumulation of
  single-consumer traits
- Port trait location is now governed by an explicit rule rather than
  historical accident

**Negative:**
- `MemoryPortError` is a new error type that must be maintained alongside
  `MemoryError` in `hkask-agents`
- Re-export shim in `hkask-agents/src/ports/memory_storage.rs` adds a thin
  indirection; should be removed once all consumers migrate

**Risks:**
- Future port promotions may miss the two-consumer check, causing traits to
  accumulate back in `hkask-ports`. Mitigation: this ADR documents the rule
  explicitly.

## Migration Path

1. ✅ `EpisodicStoragePort`, `SemanticStoragePort` → `hkask-memory/src/ports.rs`
2. ✅ `MemoryPortError` created in `hkask-memory/src/error.rs`
3. ✅ `hkask-agents` re-exports for backward compatibility
4. 🔜 Eventually remove re-export shim when all consumers import from
   `hkask-memory`
