---
title: "Public Surface Justifications — Deep-Module Audit"
audience: [architects, developers]
last_updated: 2026-06-17
version: "0.28.0"
status: "Active"
domain: "Governance"
mds_categories: [composition, curation]
---

# Public Surface Justifications — Deep-Module Audit

**Purpose:** Single-source governance record justifying every crate whose public surface exceeds the Ousterhout deep-module threshold (≤7 public functions, P5). Consolidated from 16 individual files (archived 2026-06-17).

**Threshold:** ≤7 public items per crate. Exceptions must pass the deletion test with documented rationale.

---

## Consolidated Justifications

| Crate | Pub Items | Key Concerns | Justification |
|-------|-----------|-------------|---------------|
| `hkask-services` | 66 | 28 private `AgentService` fields, domain submodules (chat, goal, wallet, pods, sovereignty, backup, settings), unified `ServiceError` | Strangler fig consolidation target. Each submodule ≤7 functions individually. Re-exports are organizational, not 66 distinct functions in one file. |
| `hkask-services-backup` | 12 | BackupService, GitCASPort policy layer, snapshot metadata, encryption config, artifact serialization | Extracted from `hkask-services` for parallel compilation. Policy layer atop hexagonal GitCASPort — config, loop, metadata, scope, and serialization are distinct concerns. Each is independently testable. |
| `hkask-types` | 50 | CNS span registry (28 variants), WebID, RDF types, gas types, wallet types | Canonical type crate. CNS spans alone justify the surface — each span is a domain concept, not a function. Types have zero behavior; they *define* the vocabulary. Reduced from 77 to 28 variants (v0.28.0) by consolidating tool-spans into a single `Tool` variant with `ToolSubsystem` discriminator and removing spans for removed/deferred features (improv, training, adapter/endpoint, kanban, kata, condenser, lazy universe, prompts, connectors, pipelines, reviews, templates, variety, goals, tests, set points, backpressure, cadence, evolution, outcome). |
| `hkask-test-harness` | 42 | Contract verification, proptest strategies, test utilities, CNS test helpers | Testing infrastructure crate. Each strategy and utility is a test-only concern. Contract verification macros are code generation, not runtime functions. |
| `hkask-storage` | 39 | `define_store!` macro, `TripleStore` CRUD, RDF persistence, vector store, migrations | Persistence orchestration. `define_store!` generates per-store types. Each store (episodic, semantic, spec, etc.) adds surface items but follows the same deep pattern. |
| `hkask-agents` | 26 | ActivePods, AgentRegistry, replicant lifecycle, capability delegation, AgentMode | Multi-concern crate spanning pod creation, agent registration, capability management, and server-mode lifecycle. Each concern is independently testable. |
| `hkask-cns` | 25 | CyberneticsLoop, VarietyTracker, AlgedonicManager, OutcomeTracker, SeamWatcher, backpressure | CNS is the system's regulatory surface. Each component is a cybernetic feedback loop with distinct responsibilities. Composition, not sprawl. |
| `hkask-improv` | 19 | 5 improv modes (Plussing, YesAnd, YesBut, Freestyling, Riffing), kata improv integration, ensemble coordination | Each mode is a distinct interaction grammar. 5 modes × 3–4 functions each ≈ 19 items. Reduced from 22 (v0.28.0): `cns` module deleted (ImprovCns trait + TracingImprovCns struct removed). Passes deep-module individually per mode. |
| `hkask-templates` | 22 | Jinja2 rendering, template registry, bootstrap manifest, template types (KnowAct/FlowDef/WordAct) | Template engine surface. Registry, rendering, and type classification are distinct concerns. Bootstrap registration adds surface items mechanically. |
| `hkask-wallet` | 22 | WalletManager, key lifecycle, rJoule currency, multi-chain bridges, shielded transactions | Wallet is a domain boundary. Keys, balances, deposits, withdrawals, and privacy shielding are distinct operations — each earns its existence. |
| `hkask-inference` | 18 | InferenceRouter, provider backends (DeepInfra, Fireworks, fal.ai), model catalog, budget tracking | Provider abstraction layer. Each provider backend is a concern; the router composes them. Backend count scales with provider support. |
| `hkask-adapter` | 17 | Expertise, TrainedLoRAAdapter, AdapterStore, AdapterRouter, EndpointLifecycle, EndpointGuard, CostModel | Multi-concern crate spanning domain types, persistence, lifecycle, provider routing, and pricing. Each concern is a distinct architecture domain. |
| `hkask-mcp` | 17 | MCP gateway, capability verification (Gate-3), transport abstraction, tool governance | MCP protocol surface. Gateway, transport, and governance are distinct protocol layers. Gate-3 capability verification alone justifies 4 items. |
| `hkask-api` | 16 | HTTP router, OpenAPI generation, endpoint handlers, delegation token auth | API surface crate. Each endpoint group (bots, cns, wallet, goals, specs, templates) is a resource. OpenAPI generation adds surface items mechanically. |
| `hkask-memory` | 14 | Episodic memory, semantic memory, narrative generation, dual encoding, memory pipelines | Memory subsystem. Each memory type (episodic, semantic) is a distinct concern. Dual encoding and narrative generation compose them. |
| `hkask-keystore` | 11 | Argon2id + HKDF key derivation, OS keychain, SQLCipher, key versioning | Security crate. Key derivation, storage, encryption, and versioning are distinct security concerns. Each is independently auditable. |
| `hkask-mcp-training` | 4 | Training job submission, dataset preparation, HuggingFace integration, provider configuration | Passes (≤7). Training pipeline is a focused MCP surface. 4 items all serve the single concern of adapter training. |

---

## Deletion Test Summary

Every crate above passes the deletion test: delete it and its complexity reappears duplicated across consumers. The public surface count reflects **breadth of domain concerns**, not shallow design. Individual submodules within each crate consistently stay at or near the ≤7 threshold.

**Verification:** `scripts/check-public-surface.sh` audits surface sizes programmatically. All 16 crates have been reviewed and accepted.

---

*Consolidated from 16 individual PUBLIC_SURFACE-*.md files (archived 2026-06-17). Governance value preserved; file count reduced 16→1.*
