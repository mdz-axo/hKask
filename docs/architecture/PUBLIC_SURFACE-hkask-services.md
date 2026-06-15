# Public Surface Justification — hkask-services

**Crate:** `hkask-services`  
**Public items in lib.rs:** 66  
**Deep-module threshold:** ≤7 public functions (Ousterhout)

## Why This Surface Is Large

`hkask-services` is the **shared service layer** — the strangler fig extraction target for business logic duplicated across CLI, API, and MCP surfaces. Its public surface is large because it consolidates what was previously scattered:

1. **Domain service modules** — Each submodule (chat, goal, wallet, pods, sovereignty, backup, settings, etc.) exposes a focused service with 3–7 public functions. The crate root re-exports all of them for surface-level consumers (CLI, API, REPL).

2. **Strangler fig consolidation** — Per the refactor-service-layer discipline, business logic that appeared in both CLI and API (settings persistence, goal management, wallet operations) has been extracted here. The surface size reflects the breadth of domains served, not shallow design.

3. **Single error vocabulary** — `ServiceError` is the unified error enum composing all domain errors. Its 30+ variants replace what would otherwise be per-crate error enums with duplicated variants.

## Mitigations

- **Submodule depth:** Each domain module (e.g., `wallet.rs`, `goal.rs`) has ≤7 public functions — they pass the deep-module test individually.
- **Re-export is organizational:** The 66 pub items in lib.rs are `pub use` or `pub mod` declarations, not 66 distinct functions in one file.
- **ServiceError is exhaustive:** Despite 30+ variants, each maps to a distinct recovery path — no catch-all, no `Other(String)`.

## Deletion Test

If you delete `hkask-services`, the settings persistence logic, goal management, wallet operations, sovereignty verification, backup orchestration, and CNS service adapters reappear duplicated across CLI, API, and MCP server surfaces. The crate earns its existence.
