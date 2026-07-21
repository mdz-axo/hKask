---
title: "Explanation — Architecture and Design Decisions"
audience: [architects, developers]
last_updated: 2026-07-20
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, curation]
---

# Explanation — Architecture and Design Decisions

Background, context, and reasoning for hKask's design. "This design exists because…"

Seven guides cover architecture patterns, the terminal UI boundary, cybernetic regulation, sovereignty enforcement, federation, resource economics, and cognition.

| Guide | Topics | Domain Tier |
|-------|-------|-------------|
| [Architecture Patterns](architecture-patterns.md) | Hexagonal ports and adapters (17 traits), loom-and-thread philosophy, Good Regulator theorem (Conant-Ashby), Viable System Model mapping (S1–S5), dual-axis ontology (PKO + DC+BIBO), API surface equivalence (P3). | Core |
| [Terminal UI Architecture](tui-architecture.md) | Ratatui workspace ownership, CLI→REPL→TUI bridge wiring, current implementation status, adversarial architecture findings, and componentized improvement order. | Domain supplement |
| [CNS and Loops](cns-and-loops.md) | CNS homeostatic loop (sense→compare→compute→act→verify), skill PDCA model (FlowDef manifests, convergence contracts), Curator metacognition (CurationLoop + MetacognitionLoop), bug hunting methodology (Weinberg, Beizer, Hendrickson), QA system (YAML manifests, LLM classification, CNS spans). | Core |
| [Sovereignty and OCAP](sovereignty-and-ocap.md) | Object Capability MCP dispatch (DelegationToken, GovernedTool 6-step membrane, fail-closed semantics), Diataxis quality review (diagram audit gates, OWASP anchoring). | Core |
| [Federation and Transport](federation-and-transport.md) | Cross-instance federation protocol, FederationDispatch lifecycle (register→invite→accept→pause→revoke→leave→dissolve), CRDT-based sync, merged registries, sovereignty guarantees. | Core |
| [Energy and Economy](energy-and-economy.md) | Gas system (GasBudget, Well, WalletManager, rJoule), double-entry ledger, database driver abstraction (SQLite/PostgreSQL, SQLCipher, column-level encryption), LoRA adapter store lifecycle (AdapterStore, AdapterRouter, EndpointLifecycle, provider selection). | Core |
| [Cognition and Replica](cognition-and-replica.md) | Fusion system design recommendations (multi-model deliberation), scenario forecasting (Schwartz + Tetlock + Chermack pipeline), ν-event semantics (ObservableSpan, NuEvent, CANONICAL_NAMESPACES, decay-weighted replay), Companies MCP server (41 tools, DCF valuation, forecast feedback, portfolio ledger). | Core |

## ADR Archive

Design decisions recorded as Architecture Decision Records:

- [ADR-031: Consolidation Authorization](../architecture/ADRs/ADR-031-consolidation-authorization.md)
- [ADR-035: Replicant Server Mode](../architecture/ADRs/ADR-035-replicant-server-mode.md)
- [ADR-043: Database Driver](../architecture/ADRs/ADR-043-database-driver.md)

> See `docs/architecture/ADRs/` for the full archive (active + retired ADRs).