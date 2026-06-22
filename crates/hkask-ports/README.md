# hkask-ports

Hexagonal architecture port traits for hKask. Defines the abstract interfaces that decouple domain logic from infrastructure adapters.

## Core Traits

- `InferencePort` — Abstract inference interface (provider-agnostic)
- `StoragePort` — Abstract storage interface (SQLCipher/Git CAS)
- `MCPPort` — Abstract MCP server dispatch interface

## Design

Ports are the "inner hexagon" boundary. Every port has one or more adapters (`hkask-adapter`) that implement it against concrete infrastructure. This enables:

- Provider swapping without domain changes
- Mock implementations for testing
- Clear dependency inversion: domain depends on ports, adapters depend on domain

## See Also

- [`hkask-adapter`](../hkask-adapter/README.md) — Adapter implementations
- [`PRINCIPLES.md`](../../docs/architecture/core/PRINCIPLES.md) §P7 — Evolutionary Architecture
