---
title: "Reference Documentation — Index"
audience: [developers, operators, agents]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain]
last-verified-against: "3d1a876f"
---

# Reference Documentation

Neutral, complete, descriptive-only documentation of the hKask system as it is. No procedures, no opinions, no explanations of why — only what.

## Crate API Reference

| Crate | Document | Description |
|-------|----------|-------------|
| `hkask-types` | [hkask-types.md](api/hkask-types.md) | ID types, NuEvent, ObservableSpan, visibility, error types (24 public modules) |
| `hkask-ports` | [hkask-ports.md](api/hkask-ports.md) | Hexagonal port traits: InferencePort, ToolPort, CnsObserver, FederationDispatch (8 traits) |
| `hkask-cns` | [hkask-cns.md](api/hkask-cns.md) | Cybernetic Nervous System: CnsRuntime, CyberneticsLoop, GovernedTool, SetPoints, GasBudget |
| `hkask-mcp` | [hkask-mcp.md](api/hkask-mcp.md) | MCP runtime, dispatch, DaemonClient, bootstrap_mcp_server, mcp_server! macro |
| `hkask-codegraph` | [hkask-codegraph.md](api/hkask-codegraph.md) | Code understanding engine: Symbol, Edge, IndexPipeline, AssembledContext |
| `hkask-agents` | [hkask-agents.md](api/hkask-agents.md) | Agent system: PodManager, CuratorAgent, ConsentManager, A2ARuntime |
| `hkask-memory` | [hkask-memory.md](api/hkask-memory.md) | Memory pipelines: EpisodicMemory, SemanticMemory, ConsolidationBridge |
| `hkask-inference` | [hkask-inference.md](api/hkask-inference.md) | Inference router: multi-provider dispatch, FusionOrchestrator |
| `hkask-templates` | [hkask-templates.md](api/hkask-templates.md) | Template system: Registry, ManifestExecutor, SkillLoader, Vocabulary |
| `hkask-capability` | [hkask-capability.md](api/hkask-capability.md) | OCAP: DelegationToken, CapabilityChecker, TokenRegistry |
| `hkask-guard` | [hkask-guard.md](api/hkask-guard.md) | Content safety: ContentGuard, GuardConfig, GuardResult |
| `hkask-database` | [hkask-database.md](api/hkask-database.md) | Database abstraction: DatabaseDriver, SqliteDriver, PostgresDriver |
| `hkask-storage` | [hkask-storage.md](api/hkask-storage.md) | Storage facade: HMemStore, NuEventStore, EmbeddingStore, sub-crates |
| `hkask-cli` | [hkask-cli.md](api/hkask-cli.md) | CLI: 33 subcommands, flags, environment variables |
| `hkask-api` | [hkask-api.md](api/hkask-api.md) | HTTP API: 26 route groups, request/response types |
| `hkask-keystore` | [hkask-keystore.md](api/hkask-keystore.md) | OS keychain + AES-256-GCM encryption |
| `hkask-wallet` | [hkask-wallet.md](api/hkask-wallet.md) | Wallet: WalletManager, ChainPort, ApiKeyIssuer, PriceFeed |
| `hkask-ledger` | [hkask-ledger.md](api/hkask-ledger.md) | Ledger: double-entry accounting, LedgerTransaction, Posting |
| `hkask-improv` | [hkask-improv.md](api/hkask-improv.md) | Improv: Plussing, Yes And, Yes But, Freestyling, Riffing |
| `hkask-condenser` | [hkask-condenser.md](api/hkask-condenser.md) | Condenser: CondenserEngine, CompressedOutput, health signals |
| `hkask-communication` | [hkask-communication.md](api/hkask-communication.md) | Communication: MatrixTransport, AgentRegistry, 7R7 listener |
| `hkask-federation` | [hkask-federation.md](api/hkask-federation.md) | Federation: FederationDispatch, CRDT sync, link lifecycle |
| `hkask-acp` | [hkask-acp.md](api/hkask-acp.md) | ACP: HkaskAcpAgent, AcpError, SessionState |

## Skill & Template Registry

- [Skill Registry Index](skills/README.md) — All 38 skills + 2 templates + 1 bundle with FlowDef parameters

## CNS Span Registry

- [CNS Span Registry](cns-spans.md) — Domain-specific ObservableSpan enums, emission points, algedonic thresholds

## Magna Carta

- [Magna Carta Reference](magna-carta.md) — P1-P4 with prohibition levels and enforcement traces

## MCP Servers

- [MCP Server Reference](mcp-servers/README.md) — All 15 MCP servers with tool tables and capability tiers

## Generated Documentation

- [CLI Reference](../generated/cli-reference.md) — Auto-generated from `kask --help`
- [OpenAPI Specification](../generated/openapi.json) — OpenAPI 3.1.0 spec for the HTTP API
- [Diagram Index](../DIAGRAMS_INDEX.md) — Registry of 55+ Mermaid diagrams
