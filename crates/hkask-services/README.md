# hkask-services

Facade crate for all hKask service implementations. Provides the unified `ServiceRegistry` that wires together all service ports via dependency inversion.

## Architecture

`hkask-services` is the **single entry point** for all service access. It does not contain implementation logic — it re-exports and composes:

- Service traits (from `hkask-services-core`)
- Service implementations (from `hkask-services-*` crates)
- Port adapters (from `hkask-adapter`)

## Core Component

- `ServiceRegistry` — Singleton registry that composes all service implementations and provides them through their port interfaces

## Service Crates

| Crate | Purpose |
|-------|---------|
| `hkask-services-core` | Core service traits and port definitions |
| `hkask-services-backup` | Backup policy layer on Git CAS |
| `hkask-services-classify` | Content classification (Qwen3 MoE via KiloCode) |
| `hkask-services-cloud` | Cloud deployment primitives |
| `hkask-services-context` | Context window management |
| `hkask-services-daemon` | Background daemon services |
| `hkask-services-discover` | Content discovery and search |
| `hkask-services-embed` | Embedding generation and storage |
| `hkask-services-inference-svc` | Inference service orchestration |
| `hkask-services-kanban` | Kanban board coordination |
| `hkask-services-kata` | Toyota Kata coaching/improvement loops |
| `hkask-services-lifecycle` | Agent lifecycle management |
| `hkask-services-onboarding` | First-run and user onboarding |
| `hkask-services-skill` | Skill registry and discovery |
| `hkask-services-sovereignty` | Magna Carta enforcement |
| `hkask-services-verification` | Capability verification |
| `hkask-services-wallet` | Crypto wallet and chain port selection |

## See Also

- [`hkask-ports`](../hkask-ports/README.md) — Port trait definitions
- [`PRINCIPLES.md`](../../docs/architecture/core/PRINCIPLES.md) §P7 — Evolutionary Architecture
