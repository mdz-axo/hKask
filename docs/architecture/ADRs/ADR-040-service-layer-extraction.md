---
title: "ADR-040: Service Layer Extraction — Completing the Hub Decomposition"
audience: [architects, developers, agents]
last_updated: 2026-06-27
version: "0.1.0"
status: "Draft"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# ADR-040: Service Layer Extraction — Completing the Hub Decomposition

**Date:** 2026-06-27
**Status:** Draft
**Supersedes:** None

## Context

**Problem Statement:** `hkask-services` has become a dependency hub — it depends on 27 other hkask crates and contains 15 inline modules that duplicate or overlap with already-extracted sister service crates. The crate's `lib.rs` is a mixture of re-exports (from sister crates) and inline module definitions, creating confusion about where each capability lives.

**Stakeholders:** All consumers of `hkask-services` — CLI, API, MCP servers, TUI, agent pods.

**Constraints:**
- P5 (Essentialism): Every module must earn its existence. Pass-through modules violate the deletion test.
- P5 deep-module discipline: Public surface ≤7 items per module unless justified.
- P7 (Strangler-Fig): Migration must leave the system fully functional at every intermediate step. No big-bang rewrites.
- The `cargo test` suite (204k LoC, 39 integration test files) must continue passing at each atomic commit.
- Sister crates already exist for 8 of the 15 inline modules, suggesting this extraction was started but not finished.

### Current State

| Module | Lines | Fns (pub) | Sister Crate Exists? | Notes |
|--------|------:|----------:|---------------------|-------|
| `chat` | 1,401 | 33 (13) | No | Core chat orchestration. Deepest module. |
| `skills` | 642 | 16 (7) | `hkask-services-skill` | Skill audit logic. Should live in sister crate. |
| `verification` | 535 | 14 (2) | No | Manifest verification. 2 pub fns, mostly internal. |
| `compose` | 480 | 9 (2) | No | Prompt composition. 2 pub fns. |
| `archival` | 339 | 7 (4) | No | Snapshot/archive operations. Leaf. |
| `bundle` | 329 | 7 (7) | No | Bundle composition. 7 pub — all justified? |
| `curator` | 313 | 8 (4) | No | Curator service wrapper. |
| `lifecycle` | 292 | 14 (3) | No | Server lifecycle. 3 pub fns. |
| `cloud` | 259 | 8 (7) | No | Cloud provisioning. 7 pub — all justified? |
| `memory` | 210 | 6 (6) | `hkask-memory` | Memory service. Wraps types from sister crate. |
| `federation` | 142 | 8 (8) | `hkask-federation` | Federation lifecycle. All 8 pub. |
| `cns` | 137 | 9 (6) | `hkask-cns` | CNS health wrapper. Convenience pass-through. |
| `experience` | 128 | 3 (2) | No | CLI experience recorder. Small. |
| `consolidation` | 115 | 4 (4) | No | Data consolidation. Small. |

**Total:** 5,322 lines, 155 functions (83 public), 15 modules.

## Decision

We will complete the decomposition of `hkask-services` using the **strangler-fig pattern** — extract one module at a time, atomic commits, system fully functional at every step.

### Extraction Order (Leaf-First DAG)

1. **Pass-through / wrapper modules** (move to sister crates or inline at call site):
   - `cns` (137 lines) → merge into `hkask-cns` as a convenience layer, or evaluate if callers can use `CnsRuntime` directly
   - `federation` (142 lines) → evaluate merge with `hkask-federation`
   - `memory` (210 lines) → merge into `hkask-memory` or evaluate if it's a genuine service layer
   - `skills` (642 lines) → move to `hkask-services-skill`

2. **Leaf modules with no sister crate** (extract to new or existing crate):
   - `archival` (339 lines) → new `hkask-services-archival` or merge into `hkask-services-core`
   - `consolidation` (115 lines) → merge into `hkask-services-core`
   - `experience` (128 lines) → evaluate: is this CLI concern? If so, move to CLI.
   - `cloud` (259 lines) → evaluate: provisioning is infrastructure, not a service. May belong in CLI or API.
   - `bundle` (329 lines) → evaluate: is bundle composition a skill concern? If so, move to `hkask-services-skill`.

3. **Medium-complexity modules** (genuine services, extract to own crate):
   - `verification` (535 lines) → new `hkask-services-verification` or merge into `hkask-services-core`
   - `compose` (480 lines) → new `hkask-services-compose`
   - `curator` (313 lines) → move to `hkask-agents` (curator logic belongs with agent orchestration)
   - `lifecycle` (292 lines) → new `hkask-services-lifecycle` or merge into core

4. **Deep module** (last, most coupled):
   - `chat` (1,401 lines) → new `hkask-services-chat`. This is the core chat orchestration — it earns its existence but needs its own crate for locality.

### End State

`hkask-services` becomes one of:
- **(A) A pure re-export facade** — zero inline modules, only `pub use` statements pointing to sister crates. Callers who want the convenience of a single dependency can use it; callers who want precise deps can depend on individual service crates.
- **(B) Deleted entirely** — if all callers can be updated to depend on individual service crates directly. The CLI already depends on many service crates individually (23 hkask-* deps); this may already be the case.

We choose **(A) as the initial target** with a path to (B) if the facade proves unnecessary.

### Rollback Plan

Every extraction is an atomic commit. If any commit breaks the build or tests, we revert that single commit and adjust the extraction strategy (e.g., merge two modules into the same crate if they have circular dependencies). The system is never in a broken state across commits.

## Consequences

### Positive
- **Locality:** Each service is independently understandable, testable, and versioned.
- **Compile time:** Callers depend on only the service crates they need, reducing transitive compilation.
- **Testability:** Each service crate gets its own test surface. Contract tests are written against simplified, narrow interfaces.
- **Clarity:** No confusion about "is this in hkask-services or hkask-services-foo?"
- **P5 compliance:** No pass-through modules. No god crate with 27 dependencies.

### Negative
- **Crate count increases:** 3–7 new service crates. This adds Cargo.toml boilerplate but each crate is small and focused.
- **Migration churn:** ~40 commits over the extraction. Mechanical, but requires review attention.
- **Facade maintenance:** If we keep `hkask-services` as a facade, it needs to stay in sync with sister crate public APIs. A CI test (`all_service_deps_are_reexported`) already exists to catch drift.

## Related

- `docs/plans/service-layer-extraction.md` — full execution plan
- `docs/architecture/hKask-architecture-master.md` — architecture index (will be updated)
- `crates/hkask-services/src/lib.rs` — current hub crate (source of truth for extraction)
