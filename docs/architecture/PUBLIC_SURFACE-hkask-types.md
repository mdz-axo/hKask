# Public Surface Justification — hkask-types

**Crate:** `hkask-types`  
**Public items in lib.rs:** 50  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-types` is the **foundational type system crate** for the entire hKask ecosystem. It serves as the shared vocabulary that all 16+ other crates depend on. Its public surface is large by design, not by accident:

1. **Cross-cutting domain types** — ID types (WebID, EventID, PodID, etc.), error taxonomy (InfrastructureError, McpErrorKind), event types (NuEvent, Span, Phase), capability types (DelegationToken, CapabilityChecker), wallet types, OCR types, voice types, template types, and loop architecture types. Each category serves a distinct domain with its own consumers.

2. **Single source of truth** — Per the Miller separation principle, types that appear in 3+ crates with identical semantics belong in `hkask-types`. Consolidating them here eliminates 87× repetition of type definitions across the codebase.

3. **Re-export consolidation** — Submodules (capability, loops, ports, event) define types that are re-exported at the crate root for ergonomic access. The re-exports are the public API; submodules are organizational.

## Mitigations

- **Submodule organization:** Types are grouped by domain (capability/, loops/, ports/) with clear module docs.
- **Non-exhaustive enums:** `InfrastructureError`, `McpErrorKind`, and capability enums use `#[non_exhaustive]` to allow future extension without breaking consumers.
- **Minimal trait surface:** Only essential traits (NuEventSink, Loop) are public; implementation details stay private.

## Deletion Test

If you delete `hkask-types`, the complexity of ID types, error taxonomy, event types, capability tokens, and loop architecture reappears scattered across 16+ crates with inevitable drift. The crate earns its existence.
