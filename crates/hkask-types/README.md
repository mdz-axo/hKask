# hkask-types

Foundation types for the hKask agent platform.

Foundation crate — all other crates depend on this. Defines the core domain vocabulary shared across all loops.

**LOC:** ~3,900 (down from 12,500 — kanban, wallet, capability, ports, and domain types extracted to dedicated crates)

## Key Types

| Module | Purpose |
|--------|---------|
| `cns` | `CnsSpan` registry — canonical CNS span definitions |
| `id` | `WebID`, `AgentID`, `PodID`, and other identity types |
| `event` | ν-event types (`NuEvent`, `Span`, `Phase`) for CNS observability |
| `visibility` | `Visibility`, `Confidence`, `AccessControl` |
| `error` | `InfrastructureError`, `McpErrorKind` — cross-cutting error taxonomy |
| `template` | `LLMParameters` — shared inference parameter config |
| `curation` | `DataCategory`, `CurationDecision`, sovereignty boundary types |
| `agent` | `AgentKind` — canonical agent kind enum (with SQL impls) |
| `loops` | `LoopId` — loop identifier enum (needed by hkask-ports) |
| `crypto` | `Ed25519PublicKey` — shared cryptographic value type |

## Extracted Crates

Types that outgrew the foundation crate:

| Crate | Former Module | Purpose |
|-------|--------------|---------|
| `hkask-capability` | `capability/` | OCAP delegation tokens, Ed25519 signing, verification |
| `hkask-ports` | `ports/` | Hexagonal port traits (InferencePort, ToolPort, etc.) |
| `hkask-wallet-types` | `wallet/` | Wallet value types (RJoule, WalletConfig, ChainId, etc.) |
| `hkask-services-kanban` | `kanban.rs` | Kanban board, task, and column types |
| `hkask-cns` | `loops/` | Cybernetic loop channels and curation directives |
| `hkask-templates` | `bundle/` | Skill bundle manifests and composition |
| `hkask-agents` | `agent/`, `audit.rs`, `voice.rs` | Rich agent definition and profile types |
| `hkask-services-sovereignty` | `sovereignty.rs` | Sovereignty boundary classification |
| `hkask-services-core` | `goal.rs` (Goal), `identity.rs` | Goal types, identity/OAuth types |
| `hkask-mcp-docproc` | `ocr/` | OCR pipeline configuration types |
