---
title: "ADR-040: Service Layer Extraction — Completing the Hub Decomposition"
audience: [architects, developers, agents]
last_updated: 2026-06-28
version: "0.30.0"
status: "Implemented"
domain: "Cross-cutting"
mds_categories: [lifecycle]
---

# ADR-040: Service Layer Extraction — Completing the Hub Decomposition

**Date:** 2026-06-27  
**Status:** Implemented (updated 2026-06-28)  
**Supersedes:** None

## Context

`hkask-services` had become a dependency hub, mixing re-exports with inline modules that overlapped with sister service crates. The extraction split the monolithic crate into focused `hkask-services-*` crates while preserving a thin facade for callers that prefer a single dependency.

## Decision

The decomposition is complete. `hkask-services` is now a **re-export facade** (option A from the original ADR) with a single remaining inline `skill` module and a test that ensures every `hkask-services-*` dependency is re-exported.

### Extracted crates

| Domain | Crate | Previously |
|--------|-------|------------|
| Core config/errors/verification/inference | `hkask-services-core` | `hkask-services::{config, error, verification, inference}` |
| Chat orchestration | `hkask-services-chat` | `hkask-services::chat` |
| Prompt composition | `hkask-services-compose` | `hkask-services::compose` |
| Agent service / context / build | `hkask-services-context` | `hkask-services::context` (now `AgentService`) |
| Corpus/embedding | `hkask-services-corpus` | `hkask-services::corpus` |
| Curator service | `hkask-services-curator` | `hkask-services::curator` |
| Kanban | `hkask-services-kanban` | `hkask-services::kanban` |
| Kata | `hkask-services-kata` | `hkask-services::kata` |
| Onboarding | `hkask-services-onboarding` | `hkask-services::onboarding` |
| Runtime/providers | `hkask-services-runtime` | `hkask-services::runtime` |
| Wallet | `hkask-services-wallet` | `hkask-services::wallet` |

`hkask-services` remains the stable facade: `hkask_services::AgentService`, `hkask_services::ChatService`, etc. Callers may also depend on the individual crates directly.

## Consequences

- **Locality:** Each service is independently understandable, testable, and versioned.
- **Compile time:** Callers depend only on the service crates they need.
- **Testability:** Each service crate has its own test surface.
- **Facade maintenance:** The `all_service_deps_are_reexported` test in `crates/hkask-services/src/lib.rs` catches drift between the facade and sister crate public APIs.

## Related

- `docs/plans/service-layer-extraction.md` — historical execution plan
- `docs/architecture/hKask-architecture-master.md` — architecture index (must reference the new crate map)
- `crates/hkask-services/src/lib.rs` — current facade crate
