# hkask-services

Facade crate for all hKask service implementations. Provides the unified `ServiceRegistry` that wires together all service ports via dependency inversion.

**Version:** v0.31.0 | **Crate:** `hkask-services`

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
| `hkask-services-core` | Core service traits, config, error taxonomy, and port definitions |
| `hkask-services-context` | AgentService context, CNS runtime, cybernetic loops |
| `hkask-services-runtime` | Runtime orchestration (lifecycle, daemon, events) |
| `hkask-services-chat` | Chat session management, memory recall, turn handling |
| `hkask-services-compose` | Prompt composition with cognition configuration |
| `hkask-services-curator` | Curator daemon metacognition and escalation handling |
| `hkask-services-corpus` | Document corpus management and indexing |
| `hkask-services-kanban` | Kanban board coordination |
| `hkask-services-kata` | Toyota Kata coaching/improvement loops |
| `hkask-services-onboarding` | First-run and user onboarding |
| `hkask-services-skill` | Skill registry and discovery |
| `hkask-services-wallet` | Crypto wallet and chain port selection |

## See Also

- [`hkask-ports`](../hkask-ports/README.md) — Port trait definitions
- [`PRINCIPLES.md`](../../docs/architecture/core/PRINCIPLES.md) §P7 — Evolutionary Architecture
