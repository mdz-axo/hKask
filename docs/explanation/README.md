---
title: "Explanation — Architecture and Design Decisions"
audience: [architects, developers]
last_updated: 2026-07-07
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, curation]
last-verified-against: "3d1a876f"
---

# Explanation — Architecture and Design Decisions

Background, context, and reasoning for hKask's design. "This design exists because…"

## Core Architecture

| Document | Topic | Domain Tier |
|----------|-------|-------------|
| [Hexagonal Ports and Adapters](hexagonal-ports.md) | Ports/adapter layout — why hexagonal, trait contracts, dependency inversion. How `InferencePort`, `ToolPort`, `CnsObserver` form the dependency boundary. | Core |
| [OCAP-Governed MCP Dispatch](ocap-mcp-dispatch.md) | OCAP enforcement at the MCP tool boundary. How `GovernedTool` wraps `ToolPort` with capability checking, energy reservation, and ν-event emission. The 6-step membrane. | Core |
| [CNS Homeostatic Loop](cns-homeostatic-loop.md) | Cybernetic Nervous System self-regulation. How `CyberneticsLoop` senses deviation, selects `LoopAction`, and observes outcomes. Variety engineering and algedonic alerts. | Core |
| [Viable System Model Mapping](vsm-mapping.md) | How hKask maps onto Stafford Beer's Viable System Model (Systems 1-5). Pod autonomy (S1), CNS coordination (S2), Curator oversight (S3), policy (S4), identity (S5). | Core |
| [Nu-Event Semantics](nu-event-semantics.md) | ν-event (nu-event) design. Thin domain events as the observability contract. How `NuEvent` and `ObservableSpan` differ — events are assertions, spans are traces. Emission points and the CNS observer pattern. | Core |
| [The Good Regulator Contract](good-regulator.md) | Conant-Ashby theorem applied to hKask. The CNS must contain a model of the system it regulates. How `SetPoints`, `SloManager`, and `SeamWatcher` form that model. | Core |

## Agent Architecture

| Document | Topic | Domain Tier |
|----------|-------|-------------|
| [Curator Metacognition](curator-metacognition.md) | How the Curator agent reasons about system health. Semantic indexing, escalation handling, curation loop. The `CuratorAgent`, `CurationLoop`, and `CuratorContext` types. | Core |
| [Skill PDCA Model](skill-pdca-model.md) | How skills implement Plan-Do-Check-Act loops. FlowDef manifests, convergence thresholds, gas budgets. The difference between templates (one-shot) and skills (iterative). | Core |

## Ontology & Knowledge

| Document | Topic | Domain Tier |
|----------|-------|-------------|
| [Dual-Axis Ontology](dual-axis-ontology.md) | P5.4 dual-axis anchoring. Why PKO (process) and DC+BIBO (state) are complementary, not competing. The 5W1H core as the minimal ontological filter. Bridge crates (`hkask-bridge-dublincore`, `hkask-bridge-pko`). | Core |
| [Loom and Thread](loom-and-thread.md) | The design philosophy: Rust is the loom (fixed logic), YAML/Jinja2 is the thread (mutable content). Why this separation enables safe composition. | Core |

## Infrastructure

| Document | Topic | Domain Tier |
|----------|-------|-------------|
| [Energy, Gas, and rJoule](energy-gas-system.md) | The economic layer. How `GasBudget`, `RJoule`, and `WalletEnergyEstimator` regulate resource consumption. API key lifecycle and the Well & Wallet system. | Core |
| [Database Driver Abstraction](database-driver-abstraction.md) | Why `DatabaseDriver` exists. SQLite/SQLCipher vs PostgreSQL. Connection pooling (r2d2). Schema auto-initialization. The `from_driver()` pattern. | Core |
| [Federation Model](federation-model.md) | Cross-instance agent federation. CRDT-based sync, link lifecycle, merged registries. `FederationDispatch` and `ReplicaId`. | Core |

## ADR Archive

Design decisions recorded as Architecture Decision Records:

- [ADR-031: Consolidation Authorization](../architecture/ADRs/ADR-031-consolidation-authorization.md)
- [ADR-035: Replicant Server Mode](../architecture/ADRs/ADR-035-replicant-server-mode.md)
- [ADR-043: Database Driver](../architecture/ADR-043-database-driver.md)

> See `docs/architecture/ADRs/` for the full archive (active + retired ADRs).
