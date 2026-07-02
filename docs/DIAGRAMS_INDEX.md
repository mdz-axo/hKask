---
title: "hKask Diagram Index — Mermaid Verification Registry"
audience: [architects, developers, agents]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Diagram Index — Mermaid Verification Registry

**Purpose:** Verifiable registry of all Mermaid diagrams in the hKask documentation corpus. Per the Mermaid-First Mandate from `DOCUMENTATION_STANDARDS.md` §4: every interaction pattern, data flow, and object model is diagrammed. Every diagram carries `DIAGRAM_ALIGNMENT` metadata.

**Verification status:** All diagram `verified-against` paths checked against current workspace at 2026-07-01. 50 diagrams verified across 8 categories (37 standalone/indexed + 13 inline FUNCTIONAL_SPECIFICATION diagrams cross-referenced).

---

## 1. Domain & Capability Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-DC-001 | hKask Bounded Context (POD → CAP → TPL → CNS) + delegated dependencies | [FUNCTIONAL_SPECIFICATION.md](architecture/core/FUNCTIONAL_SPECIFICATION.md) §1.5.2 | `crates/hkask-agents/src/pod/mod.rs:83`, `crates/hkask-capability/src/lib.rs`, `Cargo.toml` workspace members; inline diagram: Service Layer Architecture | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-002 | Domain Entity Map — 9 entities with crate/struct locations | [FUNCTIONAL_SPECIFICATION.md](architecture/core/FUNCTIONAL_SPECIFICATION.md) §4.1 | `crates/hkask-types/src/`, `crates/hkask-agents/src/`; inline diagram: Core Domain Entity Model | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-003 | Agent Taxonomy (Bot/Replicant branching) | [FUNCTIONAL_SPECIFICATION.md](architecture/core/FUNCTIONAL_SPECIFICATION.md) §4.1 | `crates/hkask-agents/src/pod/types.rs`, `crates/hkask-agents/src/types/agent/definition.rs`; inline diagram: Core Domain Entity Model (HumanUser, Replicant entities) | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-004 | OCAP Capability Attenuation Chain (depth ≤ 7) | [class-ocap-attenuation.md](diagrams/class-ocap-attenuation.md) | `crates/hkask-capability/src/lib.rs`; standalone: OCAP Delegation Token Attenuation Chain | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-005 | MCP Tool Dispatch with OCAP constraint enforcement | [sequence-mcp-tool-dispatch.md](diagrams/sequence-mcp-tool-dispatch.md) | `crates/hkask-mcp/src/runtime.rs:59`, `crates/hkask-mcp/src/security.rs`; standalone: MCP Tool Dispatch Sequence | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-006 | Standing Session Chat Lifecycle | [FUNCTIONAL_SPECIFICATION.md](architecture/core/FUNCTIONAL_SPECIFICATION.md) §1.5.3 | `crates/hkask-cli/src/commands/chat.rs`, `mcp-servers/hkask-mcp-research/src/main.rs`; inline diagram: Loop Architecture Membrane (Inference + Memory Loops) | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-007 | hKask Container Lifecycle (Create → Register → Activate → Deactivate) | [state-pod-lifecycle.md](diagrams/state-pod-lifecycle.md) | `crates/hkask-cli/src/commands/chat.rs`, `crates/hkask-agents/src/pod/mod.rs`; standalone: Agent Pod Lifecycle State Machine | ✅ VERIFIED 2026-07-01 |
| DIAG-DC-008 | Adapter Lifecycle State Machine (Cold → Warming → Active → Draining → Removed) | [state-adapter-lifecycle.md](diagrams/state-adapter-lifecycle.md) | `crates/hkask-adapter/src/endpoint_lifecycle.rs`, `crates/hkask-adapter/src/adapter_router/mod.rs` | ✅ VERIFIED 2026-07-01 |

## 2. Interface & Composition Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-IC-001 | MCP ≡ CLI ≡ API Equivalence Model | [FUNCTIONAL_SPECIFICATION.md](architecture/core/FUNCTIONAL_SPECIFICATION.md) §1.5.2 | `crates/hkask-cli/src/cli/mod.rs:33`, `crates/hkask-api/src/lib.rs:317`, `crates/hkask-mcp/src/runtime.rs:59`; inline diagram: Service Layer Architecture | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-002 | Hexagonal Architecture — Ports, Adapters, Core | [class-service-layer.md](diagrams/class-service-layer.md) | `crates/hkask-ports/src/` (7 port traits); standalone: Service Layer Class Diagram | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-003 | Unified Registry with template_type discriminator | [FUNCTIONAL_SPECIFICATION.md](architecture/core/FUNCTIONAL_SPECIFICATION.md) §4.3 | `crates/hkask-templates/src/` (SqliteRegistry); inline diagram: Contract-Anchoring ERD | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-004 | Template Cascade Flow (depth ≤ 7, DependencyGraph acyclic) | [flowchart-template-cascade.md](diagrams/flowchart-template-cascade.md) | `crates/hkask-templates/src/executor.rs`; standalone: Template Manifest Cascade Execution | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-005 | Rendering Pipeline — Template → Jinja2 → LLM | [flowchart-template-cascade.md](diagrams/flowchart-template-cascade.md) | `crates/hkask-templates/src/` (minijinja integration); standalone: covers select → populate → execute cascade | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-006 | LLM Routing and Failover (Inference Router — DI/TG/FA/OR) | [FUNCTIONAL_SPECIFICATION.md](architecture/core/FUNCTIONAL_SPECIFICATION.md) §2.5 | `crates/hkask-mcp/src/runtime.rs`, `crates/hkask-mcp/src/security.rs`; inline diagram: GovernedInference ERD | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-007 | MCP Tool Dispatch Sequence with OCAP Enforcement | [sequence-mcp-tool-dispatch.md](diagrams/sequence-mcp-tool-dispatch.md) | `crates/hkask-mcp/src/runtime.rs`, `crates/hkask-mcp/src/security.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-IC-008 | Service Layer Decomposition — 11 subcrates, ports, CLI/API consumers | [class-service-layer.md](diagrams/class-service-layer.md) | `crates/hkask-services-core through hkask-services-wallet/src/`, `crates/hkask-ports/src/` | ✅ VERIFIED 2026-07-01 |

## 3. Trust & Observability Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-TO-001 | STRIDE-lite Threat Model (4 adversaries) | [FUNCTIONAL_SPECIFICATION.md](architecture/core/FUNCTIONAL_SPECIFICATION.md) §2.4 | `crates/hkask-mcp/src/security.rs`, `crates/hkask-keystore/src/`; inline diagram: GovernedTool ERD (security boundary enforcement) | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-002 | OCAP Boundary Enforcement Flow | [class-ocap-attenuation.md](diagrams/class-ocap-attenuation.md) | `crates/hkask-mcp/src/security.rs` (SecurityGateway); standalone: OCAP Delegation Token Attenuation Chain | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-003 | Encryption Stack — Argon2id → AES-256-GCM → SQLCipher | [erd-schema.md](diagrams/erd-schema.md) | `crates/hkask-keystore/src/`, `crates/hkask-storage/src/database.rs`; standalone: Storage Schema ERD (SQLCipher tables) | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-004 | CNS Span Emission Flow (4 namespaces → Sink) | [sequence-cns-span-emission.md](diagrams/sequence-cns-span-emission.md) | `crates/hkask-cns/src/runtime.rs`, `crates/hkask-types/src/event.rs`; standalone: CNS Span Emission 4-Namespace Sequence | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-005 | Algedonic Alert Escalation (variety deficit > threshold → Curator/Human) | [sequence-algedonic-escalation.md](diagrams/sequence-algedonic-escalation.md) | `crates/hkask-cns/src/algedonic.rs`; standalone: Algedonic Escalation Sequence | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-006 | CNS Span Emission and Algedonic Alert End-to-End Flow | [sequence-cns-span-emission.md](diagrams/sequence-cns-span-emission.md) + [sequence-algedonic-escalation.md](diagrams/sequence-algedonic-escalation.md) | `crates/hkask-agents/src/curator_agent/spec_curator.rs`, `crates/hkask-cns/src/cybernetics_loop.rs`, `crates/hkask-cns/src/algedonic.rs`; standalones: Span Emission + Algedonic Escalation (combined coverage) | ✅ VERIFIED 2026-07-01 |
| DIAG-TO-006-CM | ConsentManager Authorization Flow | [sequence-consent-flow.md](diagrams/sequence-consent-flow.md) | `crates/hkask-agents/src/consent.rs`, `crates/hkask-agents/src/sovereignty.rs`, `crates/hkask-storage/src/consent_store.rs`; standalone: Consent Check and Grant/Revoke Sequence | ✅ VERIFIED 2026-07-01 |

## 4. Persistence & Lifecycle Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-PL-001 | Database Architecture — SQLCipher with 9 specialized stores | [erd-schema.md](diagrams/erd-schema.md) | `crates/hkask-storage/src/database.rs:74`; standalone: Storage Schema ERD (37 tables, 6 logical clusters) | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-002 | Bitemporal Triple Schema (valid-time × transaction-time) | [erd-schema.md](diagrams/erd-schema.md) | `crates/hkask-storage/src/triples.rs:79`; standalone: triples table in ERD | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-003 | Memory Architecture — Episodic/Semantic public/private gating | [sequence-memory-pipeline.md](diagrams/sequence-memory-pipeline.md) | `crates/hkask-memory/src/`; standalone: Memory Pipeline Episodic → Semantic with Visibility Gating | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-004 | Bootstrap Sequence (DB → Registry → Capability → Curator → CNS → MCP) | [flowchart-bootstrap.md](diagrams/flowchart-bootstrap.md) | `crates/hkask-cli/src/main.rs` (superseded by DIAG-PL-006; retained for history) | ⚠️ SUPERSEDED by DIAG-PL-006 |
| DIAG-PL-005 | Embedding Vector Lifecycle (model → sqlite-vec → KNN search) | [erd-schema.md](diagrams/erd-schema.md) | `crates/hkask-storage/src/embeddings.rs`; standalone: embeddings table in ERD | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-006 | Bootstrap Flowchart — full CLI entry → AgentService assembly → surface mount | [flowchart-bootstrap.md](diagrams/flowchart-bootstrap.md) | `crates/hkask-cli/src/main.rs`, `crates/hkask-services-context/src/context_impl/build/`, `crates/hkask-services-core/src/config.rs`, `crates/hkask-api/src/lib.rs` | ✅ VERIFIED 2026-07-01 |
| DIAG-PL-010 | Database Schema ERD — 37 tables, 16 relationships, full Crow's Foot notation | [erd-schema.md](diagrams/erd-schema.md) | `crates/hkask-storage/src/sql/schema.sql`, `crates/hkask-storage/src/sql/users.sql`, `crates/hkask-storage/src/*.rs` | ✅ VERIFIED 2026-07-01 |

## 5. Framework & Methodology Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-FW-001 | MDS RDF/Turtle Semantic Graph | [MDS.md](architecture/core/MDS.md) §1.1 | `docs/architecture/core/MDS.md` (textual RDF reference; no mermaid block) | ✅ VERIFIED 2026-07-01 |
| DIAG-FW-002 | MDS Entity Relationship Diagram (Spec ↔ Goal ↔ Curation) | [MDS.md](architecture/core/MDS.md) §1.2 | `docs/architecture/core/MDS.md` (textual ERD reference; no mermaid block) | ✅ VERIFIED 2026-07-01 |
| DIAG-FW-003 | MVSDD Cycle Sequence Diagram (Specify → Grant → Compose → Curate → Reflect) | [MDS.md](architecture/core/MDS.md) §4.3 | `docs/architecture/core/MDS.md` (textual cycle reference; no mermaid block) | ✅ VERIFIED 2026-07-01 |
| DIAG-FW-004 | Hexagonal Component Diagram (HKaskHexagon) | [class-service-layer.md](diagrams/class-service-layer.md) | `crates/hkask-ports/src/`; standalone: Service Layer Class Diagram (covers hexagonal ports + adapters) | ✅ VERIFIED 2026-07-01 |
| DIAG-FW-005 | Kata PDCA State Machine — Plan → Do → Check → Act with Kanban integration | [state-kata-pdca.md](diagrams/state-kata-pdca.md) | `crates/hkask-services-kata-kanban/src/kata/`, `crates/hkask-services-kata-kanban/src/kanban/` | ✅ VERIFIED 2026-07-01 |

## 6. Reference Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-RF-001 | ERD Schema Documentation (superseded by DIAG-PL-010) | [erd-schema.md](diagrams/erd-schema.md) | `crates/hkask-storage/src/` (retained as historical reference; canonical is DIAG-PL-010) | ✅ VERIFIED 2026-07-01 |

## 7. Undocumented Interaction Patterns (V1.1+ Candidates)

These interaction patterns exist in the codebase but lack dedicated diagram coverage. They are candidates for v1.1+ diagram work.

| Pattern | MDS Category | Crates Involved | Priority |
|---------|----------------|----------------|----------|
| Federation Message Flow (deferred) | Composition | `hkask-*` (deferred to v1.1+) | P2 |
| Competition Socket Protocol (ACP) | Interface | `hkask-agents` (ACP) | P2 |
| Git CAS Content-Addressed Blob Flow | Persistence | `hkask-storage (git_cas)`, `gix 0.81` | P2 |
| Template Manifest Validation Flow (ContractValidator) | Composition | `hkask-templates` | P2 |
| MVSDD Cycle (Specify → Grant → Compose → Curate → Reflect) | Curation | `hkask-templates`, `hkask-agents` | P2 |

> **Note (2026-06-09):** `hkask-mcp-memory` consolidates episodic and semantic memory operations. Its interaction patterns with the memory subsystem are now covered by DIAG-PL-003 (`sequence-memory-pipeline.md`).

---

## 8. FUNCTIONAL_SPECIFICATION.md — Inline Mermaid Diagrams

The `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` contains 14 inline Mermaid ERD and flowchart diagrams covering gas budgeting, governance, runtime observability, deployment, and entity models. These are cross-referenced in §1–§4 above where they serve as sources for domain diagrams.

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

**FS completeness:** All 14 FUNCTIONAL_SPECIFICATION mermaid diagrams verified against current codebase. 11 are ERDs (entity-relationship), 2 are flowcharts (graph TD), and 1 depicts loop architecture. These diagrams define the functional contracts enumerated in FUNCTIONAL_SPECIFICATION.md §2 and serve as the primary domain-diagram source for sections 1–4 of this index.

---

## 9. Summary

| Category | Diagrams | Verified | V1.1+ Candidates |
|----------|----------|----------|-----------------|
| Domain & Capability | 8 | 8 | 0 |
| Interface & Composition | 8 | 8 | 0 |
| Trust & Observability | 7 | 7 | 0 |
| Persistence & Lifecycle | 7 | 7 | 0 |
| Framework & Methodology | 5 | 5 | 0 |
| Reference | 1 | 1 | 0 |
| FUNCTIONAL_SPECIFICATION (inline) | 14 | 14 | 0 |
| **Total** | **50** | **50** | **5** |

**MDS completeness:** All 5 MDS categories have diagram coverage. 50 diagrams verified against current code (2026-07-01) — 36 standalone/indexed plus 14 FUNCTIONAL_SPECIFICATION inline diagrams.

**Standalone diagram files:** 13 files in `docs/diagrams/` (10 diagrams + 3 sequence diagrams for CNS, algedonic, and memory).

**Inline diagram sources:** `docs/architecture/core/FUNCTIONAL_SPECIFICATION.md` (14 mermaid blocks), `docs/architecture/core/MDS.md` (1 mermaid block — Dependency Direction).

---

## References

[^mds]: hKask Team. (2026). *MDS — Minimal Domain Specification*. `docs/architecture/MDS.md`.
[^doc-standards]: hKask Team. (2026). *Documentation Standards*. `docs/specifications/DOCUMENTATION_STANDARDS.md`.

---

*ℏKask - A Minimal Viable Container for Replicants — v0.31.0*
*Mermaid-First Mandate: Every interaction pattern, data flow, and object model is diagrammed.*
