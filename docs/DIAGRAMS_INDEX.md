---
title: "hKask Diagram Index — Mermaid Verification Registry"
audience: [architects, developers, agents]
last_updated: 2026-07-17
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Diagram Index — Mermaid Verification Registry

**Purpose:** Verifiable registry of all Mermaid diagrams in the hKask documentation corpus. Per the Mermaid-First Mandate from `DOCUMENTATION_STANDARDS.md` §4: every interaction pattern, data flow, and object model is diagrammed. Every diagram carries `DIAGRAM_ALIGNMENT` metadata.

**Consolidation status (2026-07-12):** Most standalone diagram files from the former `docs/diagrams/` directory have been inlined into their parent documents per `DOCUMENTATION_STANDARDS.md` §1. A small number of standalone reference diagrams remain in `docs/diagrams/` for crate-specific architecture flows (e.g., scenario forecasting pipeline, condenser pipeline). This registry maps each diagram to the document where it currently resides.

---

## 1. Domain & Capability Diagrams

| Diagram ID | Description | Now Inline In | Verified Against | Status |
|-----------|-------------|---------------|-----------------|--------|
| DIAG-DC-001 | hKask Bounded Context (POD → CAP → TPL → CNS) + delegated dependencies | `architecture/core/FUNCTIONAL_SPECIFICATION.md` §1.5.2 | `crates/hkask-agents/src/pod/mod.rs:83`, `crates/hkask-capability/src/lib.rs`, `Cargo.toml` workspace members | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-002 | Domain Entity Map — 9 entities with crate/struct locations | `architecture/core/FUNCTIONAL_SPECIFICATION.md` §4.1 | `crates/hkask-types/src/`, `crates/hkask-agents/src/` | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-003 | Agent Taxonomy (Bot/Replicant branching) | `architecture/core/FUNCTIONAL_SPECIFICATION.md` §4.1 | `crates/hkask-agents/src/pod/types.rs`, `crates/hkask-agents/src/types/agent/definition.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-004 | OCAP Capability Attenuation Chain (depth ≤ 7) | `explanation/sovereignty-and-ocap.md` | `crates/hkask-capability/src/lib.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-005 | MCP Tool Dispatch with OCAP constraint enforcement | `explanation/architecture-patterns.md` | `crates/hkask-mcp/src/runtime.rs:59`, `crates/hkask-mcp/src/security.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-006 | Standing Session Chat Lifecycle | `architecture/core/FUNCTIONAL_SPECIFICATION.md` §1.5.3 | `crates/hkask-cli/src/commands/chat.rs`, `mcp-servers/hkask-mcp-research/src/main.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-007 | hKask Container Lifecycle (Create → Register → Activate → Deactivate) | `how-to/agents-and-pods.md` | `crates/hkask-cli/src/commands/chat.rs`, `crates/hkask-agents/src/pod/mod.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-008 | Adapter Lifecycle State Machine (Cold → Warming → Active → Draining → Removed) | `explanation/federation-and-transport.md` | `crates/hkask-adapter/src/endpoint_lifecycle.rs`, `crates/hkask-adapter/src/adapter_router/mod.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-009 | CodeGraph Type System — Symbol, Edge, GraphStore, IndexPipeline, 10-tool MCP server | `reference/api-reference.md` | `crates/hkask-codegraph/src/types.rs`, `crates/hkask-codegraph/src/graph/store.rs`, `crates/hkask-codegraph/src/indexer/pipeline.rs`, `mcp-servers/hkask-mcp-codegraph/src/lib.rs` | ✅ VERIFIED 2026-07-04 |
| DIAG-DC-010 | CodeGraph Indexing Pipeline — SHA-256 hash → tree-sitter parse → extract → insert → rank | `reference/api-reference.md` | `crates/hkask-codegraph/src/indexer/pipeline.rs`, `crates/hkask-codegraph/src/indexer/extractor.rs`, `crates/hkask-codegraph/src/graph/store.rs` | ✅ VERIFIED 2026-07-04 |
| DIAG-DC-011 | CodeGraph Database Schema — 3 tables, 2 virtual tables, FTS5 triggers, WAL mode | `reference/api-reference.md` | `crates/hkask-codegraph/src/graph/schema.rs:26-126` | ✅ VERIFIED 2026-07-04 |
| DIAG-DC-012 | Research Compound Search Flow — validate → cache → strategy → join_all → RRF fusion → rerank → cache → record | `status/research-mcp-adversarial-review-2026-07-17.md` | `mcp-servers/hkask-mcp-research/src/lib.rs:307-414`, `mcp-servers/hkask-mcp-research/src/providers/mod.rs:224-387,495-575` | ✅ VERIFIED 2026-07-17 |

## 2. Interface & Composition Diagrams

| Diagram ID | Description | Now Inline In | Verified Against | Status |
|-----------|-------------|---------------|-----------------|--------|
| DIAG-IC-001 | MCP ≡ CLI ≡ API Equivalence Model | `architecture/core/FUNCTIONAL_SPECIFICATION.md` §1.5.2 | `crates/hkask-cli/src/cli/mod.rs:33`, `crates/hkask-api/src/lib.rs:317`, `crates/hkask-mcp/src/runtime.rs:59` | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-002 | Hexagonal Architecture — Ports, Adapters, Core | `explanation/architecture-patterns.md` | `crates/hkask-ports/src/` (7 port traits) | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-003 | Unified Registry with template_type discriminator | `architecture/core/FUNCTIONAL_SPECIFICATION.md` §4.3 | `crates/hkask-templates/src/` (SqliteRegistry) | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-004 | Template Cascade Flow (depth ≤ 7, DependencyGraph acyclic) | `explanation/architecture-patterns.md` | `crates/hkask-templates/src/executor.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-005 | Rendering Pipeline — Template → Jinja2 → LLM | `explanation/architecture-patterns.md` | `crates/hkask-templates/src/` (minijinja integration) | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-006 | LLM Routing and Failover (Inference Router — DI/TG/FA/OR) | `architecture/core/FUNCTIONAL_SPECIFICATION.md` §2.5 | `crates/hkask-mcp/src/runtime.rs`, `crates/hkask-mcp/src/security.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-007 | MCP Tool Dispatch Sequence with OCAP Enforcement | `explanation/architecture-patterns.md` | `crates/hkask-mcp/src/runtime.rs`, `crates/hkask-mcp/src/security.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-008 | Service Layer Decomposition — 10 subcrates, ports, CLI/API consumers | `explanation/architecture-patterns.md` | `crates/hkask-services-core through hkask-services-wallet/src/`, `crates/hkask-ports/src/` | ✅ VERIFIED 2026-07-12 |
| DIAG-IC-009 | CodeGraph Agent Workflow — search, traverse, impact, context assembly, feedback loop | `reference/api-reference.md` | `mcp-servers/hkask-mcp-codegraph/src/lib.rs:243-632`, `crates/hkask-codegraph/src/graph/search.rs`, `crates/hkask-codegraph/src/graph/traversal.rs` | ✅ VERIFIED 2026-07-04 |
| DIAG-IC-010 | Companies provider routing — symbol selection, learning override, fallback, EODHD normalization | `architecture/hKask-architecture-master.md` | `mcp-servers/hkask-mcp-companies/src/providers.rs:84-247`, `mcp-servers/hkask-mcp-companies/src/lib.rs:340-361` | ✅ VERIFIED 2026-07-10 |
| DIAG-IC-011 | Companies forecast feedback — durable snapshot, revision, outcome, and daemon experience flow | `architecture/hKask-architecture-master.md` | `mcp-servers/hkask-mcp-companies/src/tools/analytics.rs:438-457`, `mcp-servers/hkask-mcp-companies/src/tools/valuation.rs:634-659,774-915`, `mcp-servers/hkask-mcp-companies/src/portfolio.rs:303-400` | ✅ VERIFIED 2026-07-10 |
| DIAG-IC-012 | CNS Architecture — responsibility clusters, wallet port, extraction status | `explanation/cns-and-loops.md` | `crates/hkask-cns/src/cybernetics_loop.rs`, `crates/hkask-cns/src/runtime.rs`, `crates/hkask-cns/src/wallet_budget.rs`, `crates/hkask-cns/src/slo_manager.rs`, `crates/hkask-storage-guard/src/lib.rs`, `crates/hkask-cns/src/seam_watcher.rs`, `crates/hkask-ports/src/wallet_budget_port.rs` | ✅ VERIFIED 2026-07-11 |
| DIAG-IC-013 | Research MCP Server Architecture — ResearchServer, ProviderPool, WebSearchPort, cache, rate limiter, RSS DB | `status/research-mcp-adversarial-review-2026-07-17.md` | `mcp-servers/hkask-mcp-research/src/lib.rs:48-56`, `mcp-servers/hkask-mcp-research/src/providers/mod.rs:130-135,494-618` | ✅ VERIFIED 2026-07-17 |
| DIAG-IC-014 | Research Provider Trait Hierarchy — WebSearchPort, WebSearchProvider, WebExtractProvider, WebBrowseProvider, 9 concrete providers | `status/research-mcp-adversarial-review-2026-07-17.md` | `mcp-servers/hkask-mcp-research/src/providers/mod.rs:50-135`, `mcp-servers/hkask-mcp-research/src/providers/brave.rs:20`, `mcp-servers/hkask-mcp-research/src/providers/firecrawl.rs:30,102,183` | ✅ VERIFIED 2026-07-17 |

## 3. Trust & Observability Diagrams

| Diagram ID | Description | Now Inline In | Verified Against | Status |
|-----------|-------------|---------------|-----------------|--------|
| DIAG-TO-001 | STRIDE-lite Threat Model (4 adversaries) | `architecture/core/FUNCTIONAL_SPECIFICATION.md` §2.4 | `crates/hkask-mcp/src/security.rs`, `crates/hkask-keystore/src/` | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-002 | OCAP Boundary Enforcement Flow | `explanation/sovereignty-and-ocap.md` | `crates/hkask-mcp/src/security.rs` (SecurityGateway) | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-003 | Encryption Stack — Argon2id → AES-256-GCM → SQLCipher | `architecture/hKask-architecture-master.md` | `crates/hkask-keystore/src/`, `crates/hkask-storage/src/database.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-004 | CNS Span Emission Flow (4 namespaces → Sink) | `explanation/cns-and-loops.md` | `crates/hkask-cns/src/runtime.rs`, `crates/hkask-types/src/event.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-005 | Algedonic Alert Escalation (variety deficit > threshold → Curator/Human) | `explanation/cns-and-loops.md` | `crates/hkask-cns/src/algedonic.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-006 | CNS Span Emission and Algedonic Alert End-to-End Flow | `explanation/cns-and-loops.md` | `crates/hkask-agents/src/curator_agent/spec_curator.rs`, `crates/hkask-cns/src/cybernetics_loop.rs`, `crates/hkask-cns/src/algedonic.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-006-CM | ConsentManager Authorization Flow | `explanation/sovereignty-and-ocap.md` | `crates/hkask-agents/src/consent.rs`, `crates/hkask-agents/src/sovereignty.rs`, `crates/hkask-storage/src/consent_store.rs` | ✅ VERIFIED 2026-07-01 |

## 4. Persistence & Lifecycle Diagrams

| Diagram ID | Description | Now Inline In | Verified Against | Status |
|-----------|-------------|---------------|-----------------|--------|
| DIAG-PL-001 | Database Architecture — SQLCipher with 9 specialized stores | `architecture/hKask-architecture-master.md` | `crates/hkask-storage/src/database.rs:74` | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-002 | Bitemporal hMem Schema (valid-time × transaction-time) | `architecture/hKask-architecture-master.md` | `crates/hkask-storage/src/triples.rs:79` | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-003 | Memory Architecture — Episodic/Semantic public/private gating | `explanation/cognition-and-replica.md` | `crates/hkask-memory/src/` | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-004 | Bootstrap Sequence (superseded by DIAG-PL-006) | `how-to/install-and-configure.md` | `crates/hkask-cli/src/main.rs` (superseded) | ⚠️ SUPERSEDED by DIAG-PL-006 |
| DIAG-PL-005 | Embedding Vector Lifecycle (model → sqlite-vec → KNN search) | `architecture/hKask-architecture-master.md` | `crates/hkask-storage/src/embeddings.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-006 | Bootstrap Flowchart — full CLI entry → AgentService assembly → surface mount | `how-to/install-and-configure.md` | `crates/hkask-cli/src/main.rs`, `crates/hkask-services-context/src/context_impl/build/`, `crates/hkask-services-core/src/config.rs`, `crates/hkask-api/src/lib.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-010 | Database Schema ERD — 37 tables, 16 relationships, full Crow's Foot notation | `architecture/hKask-architecture-master.md` | `crates/hkask-storage/src/sql/schema.sql`, `crates/hkask-storage/src/sql/users.sql`, `crates/hkask-storage/src/*.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-011 | CodeGraph Indexing Pipeline — walk → hash → parse → extract → insert → rank → FTS5 | `reference/api-reference.md` | `crates/hkask-codegraph/src/indexer/pipeline.rs`, `crates/hkask-codegraph/src/indexer/extractor.rs`, `crates/hkask-codegraph/src/graph/store.rs` | ✅ VERIFIED 2026-07-04 |
| DIAG-PL-012 | CodeGraph Pipeline Lifecycle — Uninitialized → Indexing → Ready → Stale | `reference/api-reference.md` | `crates/hkask-codegraph/src/indexer/pipeline.rs:38-273` | ✅ VERIFIED 2026-07-04 |

## 5. Framework & Methodology Diagrams

| Diagram ID | Description | Now Inline In | Verified Against | Status |
|-----------|-------------|---------------|-----------------|--------|
| DIAG-FW-001 | MDS RDF/Turtle Semantic Graph | `architecture/core/MDS.md` §1.1 | `docs/architecture/core/MDS.md` (textual RDF reference) | ✅ VERIFIED 2026-07-01 |
| DIAG-FW-002 | MDS Entity Relationship Diagram (Spec ↔ Goal ↔ Curation) | `architecture/core/MDS.md` §1.2 | `docs/architecture/core/MDS.md` (textual ERD reference) | ✅ VERIFIED 2026-07-01 |
| DIAG-FW-003 | MVSDD Cycle Sequence Diagram (Specify → Grant → Compose → Curate → Reflect) | `architecture/core/MDS.md` §4.3 | `docs/architecture/core/MDS.md` (textual cycle reference) | ✅ VERIFIED 2026-07-01 |
| DIAG-FW-004 | Hexagonal Component Diagram (HKaskHexagon) | `explanation/architecture-patterns.md` | `crates/hkask-ports/src/` | ✅ VERIFIED 2026-07-01 |
| DIAG-FW-005 | Kata PDCA State Machine — Plan → Do → Check → Act with Kanban integration | `how-to/skills-and-composition.md` | `crates/hkask-services-kata-kanban/src/kata/`, `crates/hkask-services-kata-kanban/src/kanban/` | ✅ VERIFIED 2026-07-01 |
| DIAG-FW-006 | Kata-Kanban execution boundary — MCP prompt generation vs optional full Kata bridge | `how-to/skills-and-composition.md` | `mcp-servers/hkask-mcp-kata-kanban/src/lib.rs`, `crates/hkask-services-kata-kanban/src/bridge.rs`, `crates/hkask-services-kata-kanban/src/kanban/service_impl/kata.rs` | ✅ VERIFIED 2026-07-10 |
| DIAG-FW-007 | Scenario forecasting pipeline — framing, reviewed events, computation, calibration, assessment | `architecture/hKask-architecture-master.md` | `mcp-servers/hkask-mcp-scenarios/src/lib.rs:459-1708`, `mcp-servers/hkask-mcp-scenarios/src/superforecast.rs:165-400` | ✅ VERIFIED 2026-07-10 |

## 6. Reference Diagrams

| Diagram ID | Description | Now Inline In | Verified Against | Status |
|-----------|-------------|---------------|-----------------|--------|
| DIAG-RF-001 | ERD Schema Documentation (superseded by DIAG-PL-010) | `architecture/hKask-architecture-master.md` | `crates/hkask-storage/src/` (retained as historical reference; canonical is DIAG-PL-010) | ✅ VERIFIED 2026-07-01 |
| DIAG-RF-002 | Condenser Pipeline — tool dispatch, algorithm selection, ontology anchoring | `diagrams/flowchart-condenser-pipeline.md` | `mcp-servers/hkask-mcp-condenser/src/lib.rs`, `crates/hkask-condenser/src/engine.rs`, `crates/hkask-condenser/src/algorithms.rs` | ✅ VERIFIED 2026-07-17 |
| DIAG-RF-003 | Filesystem sandbox path resolution + tool dispatch flow (execute_tool → sandbox_path → canonicalize → containment → emit_cns) | `reference/mcp-servers/filesystem.md` | `mcp-servers/hkask-mcp-filesystem/src/lib.rs:55-77`, `crates/hkask-mcp/src/server/tool_span.rs:246-259` | ✅ VERIFIED 2026-07-17 |

## 7. Undocumented Interaction Patterns (V1.1+ Candidates)

These interaction patterns exist in the codebase but lack dedicated diagram coverage. They are candidates for v1.1+ diagram work.

| Pattern | MDS Category | Crates Involved | Priority |
|---------|----------------|----------------|----------|
| Federation Message Flow (deferred) | Composition | `hkask-*` (deferred to v1.1+) | P2 |
| Competition Socket Protocol (ACP) | Interface | `hkask-agents` (ACP) | P2 |
| Git CAS Content-Addressed Blob Flow | Persistence | `hkask-storage (git_cas)`, `gix 0.81` | P2 |
| Template Manifest Validation Flow (ContractValidator) | Composition | `hkask-templates` | P2 |
| MVSDD Cycle (Specify → Grant → Compose → Curate → Reflect) | Curation | `hkask-templates`, `hkask-agents` | P2 |

> **Note (2026-06-09):** `hkask-mcp-memory` consolidates episodic and semantic memory operations. Its interaction patterns with the memory subsystem are now covered by DIAG-PL-003 (inlined in `explanation/cognition-and-replica.md`).

---

## 8. FUNCTIONAL_SPECIFICATION.md — Inline Mermaid Diagrams

The `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` contains 14 inline Mermaid ERD and flowchart diagrams covering gas budgeting, governance, runtime observability, deployment, and entity models. These were always inline (never standalone) and are cross-referenced in §1–§4 above.

| Diagram ID | Description | Section | Diagram Type |
|-----------|-------------|---------|-------------|
| DIAG-FS-001 | Service Layer Architecture — CLI/API → Service Subcrates → Domain Crates | §1.5.2 | Flowchart (graph TD) |
| DIAG-FS-002 | Loop Architecture Membrane — Domain Loops (Inference, Memory) + Curation + Cybernetics + Transport | §1.5.3 | Flowchart (graph TD) |
| DIAG-FS-003 | GasBudget ERD — Energy Budgeting with OCAP/P9 constraints | §2.1 | ERD (erDiagram) |
| DIAG-FS-004 | AlgedonicManager ERD — Algedonic Signalling with VarietyCounter, CurationLoop | §2.2 | ERD (erDiagram) |
| DIAG-FS-005 | CnsRuntime ERD — Runtime Observability with VarietyMonitor, OutcomeTracker | §2.3 | ERD (erDiagram) |
| DIAG-FS-006 | GovernedTool ERD — Tool Governance with OCAP, Consent, GasBudget constraints | §2.4 | ERD (erDiagram) |
| DIAG-FS-007 | GovernedInference ERD — Inference Governance with CompositeEnergyEstimator | §2.5 | ERD (erDiagram) |
| DIAG-FS-008 | CircuitBreaker ERD — Failure/Success/HalfOpen state tracking | §2.6 | ERD (erDiagram) |
| DIAG-FS-009 | ApiMeter ERD — Rate-Limit Buckets with per-key TokenTracker | §2.7 | ERD (erDiagram) |
| DIAG-FS-010 | CompositeEnergyEstimator ERD — Multi-backend Energy Estimation | §2.8 | ERD (erDiagram) |
| DIAG-FS-011 | CloudServer ERD — Deployment Domain (Caddy, Conduit, UserSession, Wallet) | §3.18 | ERD (erDiagram) |
| DIAG-FS-012 | Core Domain Entity Model — Full entity map (HumanUser, Replicant, Wallet, Session, etc.) | §4.1 | ERD (erDiagram) |
| DIAG-FS-013 | Deployment Domain Entity Model — KaskBinary, ServerProfile, deployment infra | §4.2 | ERD (erDiagram) |
| DIAG-FS-014 | Contract-Anchoring ERD — Principles ↔ Contracts ↔ Sub-Contracts | §4.3 | ERD (erDiagram) |

---

## 9. Training and Corpus Diagrams

| Diagram ID | Type | Description | Now Inline In | Verified Against | Status |
|------------|------|-------------|---------------|------------------|--------|
| DIAG-TRAIN-001 | flowchart | Unsloth Qwen3.6-27B training pipeline | `how-to/training-and-adapters.md` | HF: `Axolotl-Partners/rust-adapter-scripts` | ✅ VERIFIED 2026-07-10 |
| DIAG-TRAIN-002 | flowchart | Replica, corpus, and training readiness boundary | `how-to/training-and-adapters.md` | `hkask-mcp-replica`, `hkask-mcp-docproc`, `hkask-mcp-training` | ✅ VERIFIED 2026-07-10 |
| DIAG-TRAIN-003 | flowchart | Replica pipeline dispatch and unsupported-step boundary | `how-to/training-and-adapters.md` | `hkask-mcp-replica`, `hkask-ports`, `hkask-mcp` | ✅ VERIFIED 2026-07-10 |
| DIAG-TRAIN-004 | flowchart | Full training pipeline (reasoning + Rust adapters + eval) | `how-to/training-and-adapters.md` | HF: `Axolotl-Partners/rust-adapter-scripts` | ✅ VERIFIED 2026-07-11 |
| DIAG-TRAIN-005 | state | Training job lifecycle: Queued → Running → Completed → Terminated | `how-to/training-and-adapters.md` | `hkask-mcp-training/src/providers/types.rs`, HF: `Axolotl-Partners/rust-adapter-scripts` | ✅ VERIFIED 2026-07-11 |
| DIAG-TRAIN-006 | class | Training server type hierarchy: TrainingHost, HarnessAdapter, params | `how-to/training-and-adapters.md` | `hkask-mcp-training/src/providers/` | ✅ VERIFIED 2026-07-11 |

## 10. TUI (Terminal UI) Diagrams

| Diagram ID | Type | Description | Now Inline In | Verified Against | Status |
|------------|------|-------------|---------------|------------------|--------|
| DIAG-TUI-001 | class | Window trait hierarchy, bridge traits, 22 window types | `reference/api-reference.md` | `crates/hkask-tui/src/window.rs`, `bridges/`, `mcp_tabbed.rs`, `window_catalog.rs` | ✅ VERIFIED 2026-07-10 |
| DIAG-TUI-002 | flowchart | Event dispatch pipeline: crossterm → global → palette → window | `reference/api-reference.md` | `crates/hkask-tui/src/lib.rs:145-218`, `workspace.rs:543-646` | ✅ VERIFIED 2026-07-10 |
| DIAG-TUI-003 | state | Workspace lifecycle: init → splash → running → splits → quit | `reference/api-reference.md` | `crates/hkask-tui/src/workspace.rs`, `layout.rs`, `window.rs` | ✅ VERIFIED 2026-07-10 |
| DIAG-TUI-004 | flowchart | Bridge wiring: CLI → `with_bridges!` → `WorkspaceBridges` → `create_window` → window | `reference/api-reference.md` | `crates/hkask-tui/src/bridges/mod.rs`, `window_catalog.rs`, `crates/hkask-repl/src/tui_bridges.rs` | ✅ VERIFIED 2026-07-10 |

## 11. Additional Inlined Diagrams (Not Previously Indexed)

The following diagrams were standalone files not individually tracked in the original index sections 1–10. They are now inlined in their parent documents.

| Diagram File (former) | Type | Now Inline In | Description |
|----------------------|------|---------------|-------------|
| `class-database-driver.md` | class | `architecture/hKask-architecture-master.md` | Database Driver Class Diagram |
| `class-ports-trait-hierarchy.md` | class | `explanation/architecture-patterns.md` | Hexagonal Ports Trait Hierarchy |
| `class-service-error-hierarchy.md` | class | `explanation/architecture-patterns.md` | ServiceError Hierarchy |
| `erd-k8s-resources.md` | ERD | `how-to/deployment-and-transport.md` | K8s Resource Relationships |
| `erd-multi-user.md` | ERD | `architecture/hKask-architecture-master.md` | Multi-User Data Model |
| `erd-sqlcipher-schema.md` | ERD | `architecture/hKask-architecture-master.md` | SQLCipher Schema |
| `flowchart-architecture-overview.md` | flowchart | `explanation/architecture-patterns.md` | Classification + Guard Architecture Overview |
| `flowchart-connection-lifecycle.md` | flowchart | `architecture/hKask-architecture-master.md` | Database Connection Lifecycle |
| `flowchart-cns-homeostatic-loop.md` | flowchart | `explanation/cns-and-loops.md` | CNS Homeostatic Loop |
| `flowchart-cns-regulation.md` | flowchart | `explanation/cns-and-loops.md` | CNS Regulation Pipeline — 5-Phase Cybernetic Cycle |
| `flowchart-curator-metacognition.md` | flowchart | `explanation/cns-and-loops.md` | Curator Metacognition Loop |
| `flowchart-deployment-architecture.md` | flowchart | `how-to/deployment-and-transport.md` | K8s Deployment Architecture |
| `flowchart-drift-detection.md` | flowchart | `explanation/cns-and-loops.md` | Classifier Drift Detection (removed — superseded by algo / no-judge) |
| `flowchart-dual-classification.md` | flowchart | `explanation/cognition-and-replica.md` | Algo / No-Judge Classification Flow |
| `flowchart-guard-pipeline.md` | flowchart | `explanation/sovereignty-and-ocap.md` | Content Safety Guard Pipeline |
| `flowchart-memory-remember.md` | flowchart | `explanation/cognition-and-replica.md` | Memory Remember — Algo / No-Judge Template Cascade |
| `flowchart-oauth-registration.md` | flowchart | `how-to/deployment-and-transport.md` | OAuth Registration & Onboarding Flow |
| `flowchart-pod-startup.md` | flowchart | `how-to/deployment-and-transport.md` | K8s Pod Startup Sequence |
| `sequence-auth-flow.md` | sequence | `how-to/deployment-and-transport.md` | Authentication Flow — OAuth Sequence |
| `sequence-classify-to-memory.md` | sequence | `explanation/cognition-and-replica.md` | Classification-to-Memory Sequence |
| `sequence-mcp-bootstrap.md` | sequence | `explanation/architecture-patterns.md` | MCP Bootstrap and Tool Dispatch |
| `state-guard-violations.md` | state | `explanation/sovereignty-and-ocap.md` | Guard Violation Lifecycle |
| `state-invite-lifecycle.md` | state | `how-to/deployment-and-transport.md` | Invite Lifecycle State Machine |
| `state-loop-action-lifecycle.md` | state | `explanation/cns-and-loops.md` | LoopAction Lifecycle |

## 12. Summary

All Mermaid diagrams are now inline in their parent documents. The former `docs/diagrams/` directory has been eliminated. 72 diagram artifacts total: 57 formerly standalone diagrams inlined into 12 parent documents + 14 inline diagrams in `FUNCTIONAL_SPECIFICATION.md` + 1 newly authored inline diagram (DIAG-RF-003, filesystem sandbox model).

**Parent document diagram distribution:**

| Parent Document | Inlined Diagram Count |
|----------------|----------------------|
| `explanation/cns-and-loops.md` | 8 |
| `explanation/architecture-patterns.md` | 7 |
| `reference/api-reference.md` | 9 |
| `architecture/hKask-architecture-master.md` | 8 |
| `how-to/training-and-adapters.md` | 6 |
| `how-to/deployment-and-transport.md` | 6 |
| `explanation/cognition-and-replica.md` | 4 |
| `explanation/sovereignty-and-ocap.md` | 4 |
| `how-to/skills-and-composition.md` | 2 |
| `explanation/federation-and-transport.md` | 1 |
| `how-to/install-and-configure.md` | 1 |
| `how-to/agents-and-pods.md` | 1 |
| `reference/mcp-servers/filesystem.md` | 1 |
| `architecture/core/FUNCTIONAL_SPECIFICATION.md` | 14 (always inline) |
| **Total** | **72** |

**MDS completeness:** all five MDS categories have diagram coverage. Training diagrams are additionally anchored to the P2 consent boundary, P4 capability-boundary requirement, and P9 feedback-loop requirement in [`PRINCIPLES.md`](architecture/core/PRINCIPLES.md).

---

## References

[^mds]: hKask Team. (2026). *MDS — Minimal Domain Specification*. `docs/architecture/core/MDS.md`.
[^doc-standards]: hKask Team. (2026). *Documentation Standards*. `docs/specifications/DOCUMENTATION_STANDARDS.md`.

---

*ℏKask - A Minimal Viable Container for Replicants — v0.31.0*
*Mermaid-First Mandate: Every interaction pattern, data flow, and object model is diagrammed.*
*All diagrams inline per DOCUMENTATION_STANDARDS §1 — consolidated 2026-07-12.*