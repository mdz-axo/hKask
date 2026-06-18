---
title: "hKask Diagram Index — Mermaid Verification Registry"
audience: [architects, developers, agents]
last_updated: 2026-06-15
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Diagram Index — Mermaid Verification Registry

**Purpose:** Verifiable registry of all Mermaid diagrams in the hKask documentation corpus. Per the Mermaid-First Mandate from `DOCUMENTATION_STANDARDS.md` §4: every interaction pattern, data flow, and object model is diagrammed. Every diagram carries `DIAGRAM_ALIGNMENT` metadata.

**Verification status:** All diagram `verified-against` paths checked against current workspace at 2026-06-13.

---

## 1. Domain & Capability Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-DC-001 | hKask Bounded Context (POD → CAP → TPL → CNS) + delegated dependencies | [MDS.md](architecture/core/MDS.md) §1 | `crates/hkask-agents/src/pod/mod.rs:83`, `crates/hkask-types/src/capability/mod.rs:223`, `Cargo.toml` workspace members | ✅ VERIFIED |
| DIAG-DC-002 | Domain Entity Map — 9 entities with crate/struct locations | [MDS.md](architecture/core/MDS.md) §3.1 | `crates/hkask-types/src/`, `crates/hkask-agents/src/` | ✅ VERIFIED |
| DIAG-DC-003 | Agent Taxonomy (Bot/Replicant branching) | [MDS.md](architecture/core/MDS.md) §4 | `crates/hkask-agents/src/pod/types.rs`, `crates/hkask-agents/src/curator_agent/bot_metrics.rs` | ✅ VERIFIED |
| DIAG-DC-004 | OCAP Capability Attenuation Chain (depth ≤ 7) | [MDS.md](architecture/core/MDS.md) §5 | `crates/hkask-types/src/capability/mod.rs:223` | ✅ VERIFIED |
| DIAG-DC-005 | MCP Tool Dispatch with OCAP constraint enforcement | [MDS.md](architecture/core/MDS.md) §6 | `crates/hkask-mcp/src/runtime.rs:59`, `crates/hkask-mcp/src/security.rs` | ✅ VERIFIED |
| DIAG-DC-006 | hLexicon Allocation Table (WordAct/FlowDef/KnowAct) | [MDS.md](architecture/core/MDS.md) §7 | `docs/architecture/reference/hKask-hLexicon.md` | ✅ VERIFIED |
| DIAG-DC-007 | Standing Session Chat Lifecycle | [MDS.md](architecture/core/MDS.md) §6.4 | `crates/hkask-cli/src/commands/chat.rs`, `mcp-servers/hkask-mcp-web/src/main.rs` | ✅ VERIFIED |
| DIAG-DC-008 | hKask Container Lifecycle (Create → Register → Activate → Deactivate) | [MDS.md](architecture/core/MDS.md) §6.5 | `crates/hkask-cli/src/commands/chat.rs`, `crates/hkask-agents/src/pod/mod.rs` | ✅ VERIFIED |

## 2. Interface & Composition Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-IC-001 | MCP ≡ CLI ≡ API Equivalence Model | [MDS.md](architecture/core/MDS.md) §1 | `crates/hkask-cli/src/cli/mod.rs:33`, `crates/hkask-api/src/lib.rs:636`, `crates/hkask-mcp/src/runtime.rs:59` | ✅ VERIFIED |
| DIAG-IC-002 | Hexagonal Architecture — Ports, Adapters, Core | [MDS.md](architecture/core/MDS.md) §2 | `docs/architecture/reference/ports-inventory.md` (19 traits) | ✅ VERIFIED |
| DIAG-IC-003 | Unified Registry with template_type discriminator | [MDS.md](architecture/core/MDS.md) §4 | `crates/hkask-templates/src/` (SqliteRegistry) | ✅ VERIFIED |
| DIAG-IC-004 | Template Cascade Flow (depth ≤ 7, DependencyGraph acyclic) | [MDS.md](architecture/core/MDS.md) §5 | `crates/hkask-templates/src/dependency.rs` | ✅ VERIFIED |
| DIAG-IC-005 | Rendering Pipeline — Template → Jinja2 → LLM | [MDS.md](architecture/core/MDS.md) §6 | `crates/hkask-templates/src/` (minijinja integration) | ✅ VERIFIED |
| DIAG-IC-006 | LLM Routing and Failover (Inference Router — OM/FW/DI) | [MDS.md](architecture/core/MDS.md) §2.5 | `crates/hkask-mcp/src/runtime.rs`, `crates/hkask-mcp/src/security.rs` | ✅ VERIFIED |

## 3. Trust & Observability Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-TO-001 | STRIDE-lite Threat Model (4 adversaries) | [MDS.md](architecture/core/MDS.md) §1 | `crates/hkask-mcp/src/security.rs`, `crates/hkask-keystore/src/` | ✅ VERIFIED |
| DIAG-TO-002 | OCAP Boundary Enforcement Flow | [MDS.md](architecture/core/MDS.md) §2 | `crates/hkask-mcp/src/security.rs` (SecurityGateway) | ✅ VERIFIED |
| DIAG-TO-003 | Encryption Stack — Argon2id → AES-256-GCM → SQLCipher | [MDS.md](architecture/core/MDS.md) §3 | `crates/hkask-keystore/src/`, `crates/hkask-storage/src/database.rs` | ✅ VERIFIED |
| DIAG-TO-004 | CNS Span Emission Flow (4 namespaces → Sink) | [MDS.md](architecture/core/MDS.md) §4 | `crates/hkask-cns/src/runtime.rs`, `crates/hkask-types/src/event.rs` | ✅ VERIFIED |
| DIAG-TO-005 | Algedonic Alert Escalation (variety deficit > 100 → Curator/Human) | [MDS.md](architecture/core/MDS.md) §4.4 | `crates/hkask-cns/src/algedonic.rs` | ✅ VERIFIED |
| DIAG-TO-006 | CNS Span Emission and Algedonic Alert End-to-End Flow | [MDS.md](architecture/core/MDS.md) §4.4.1 | `crates/hkask-agents/src/curator_agent/spec_curator.rs`, `crates/hkask-cns/src/cybernetics_loop.rs`, `crates/hkask-cns/src/algedonic.rs` | ✅ VERIFIED |
| DIAG-TO-006-CM | ConsentManager Authorization Flow | [MDS.md](architecture/core/MDS.md) §3.0.1 | `crates/hkask-agents/src/consent.rs`, `crates/hkask-agents/src/sovereignty.rs`, `crates/hkask-storage/src/consent_store.rs` | ✅ VERIFIED |

## 4. Persistence & Lifecycle Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-PL-001 | Database Architecture — SQLCipher with 9 specialized stores | [MDS.md](architecture/core/MDS.md) §1 | `crates/hkask-storage/src/database.rs:74` | ✅ VERIFIED |
| DIAG-PL-002 | Bitemporal Triple Schema (valid-time × transaction-time) | [MDS.md](architecture/core/MDS.md) §2 | `crates/hkask-storage/src/triples.rs:79` | ✅ VERIFIED |
| DIAG-PL-003 | Memory Architecture — Episodic/Semantic public/private gating | [MDS.md](architecture/core/MDS.md) §3 | `crates/hkask-memory/src/` | ✅ VERIFIED |
| DIAG-PL-004 | Bootstrap Sequence (DB → hLexicon → Registry → Capability → Curator → CNS → MCP) | [MDS.md](architecture/core/MDS.md) §5 | `crates/hkask-cli/src/main.rs` | ✅ VERIFIED |
| DIAG-PL-005 | Embedding Vector Lifecycle (model → sqlite-vec → KNN search) | [MDS.md](architecture/core/MDS.md) §4 | `crates/hkask-storage/src/embeddings.rs` | ✅ VERIFIED |

## 5. Framework & Methodology Diagrams

| Diagram ID | Description | Document | Verified Against | Status |
|-----------|-------------|----------|-----------------|--------|
| DIAG-FW-001 | MDS RDF/Turtle Semantic Graph | [`MDS.md`](architecture/core/MDS.md) §1.1 | `docs/architecture/MDS.md` | ✅ VERIFIED |
| DIAG-FW-002 | MDS Entity Relationship Diagram (Spec ↔ Goal ↔ Curation) | [`MDS.md`](architecture/core/MDS.md) §1.2 | `docs/architecture/MDS.md` | ✅ VERIFIED |
| DIAG-FW-003 | MVSDD Cycle Sequence Diagram (Specify → Grant → Compose → Curate → Reflect) | [`MDS.md`](architecture/core/MDS.md) §4.3 | `docs/architecture/MDS.md` | ✅ VERIFIED |
| DIAG-FW-004 | Hexagonal Component Diagram (HKaskHexagon) | [`MDS.md`](architecture/core/MDS.md) §6.1 | `docs/architecture/reference/ports-inventory.md` | ✅ VERIFIED |

## 6. Reference Diagrams

*No reference diagrams remain; ERD documentation has been retired. Recoverable from git history.*


## 7. Undocumented Interaction Patterns (V1.1+ Candidates)

These interaction patterns exist in the codebase but lack dedicated diagram coverage. They are candidates for v1.1+ diagram work.

| Pattern | MDS Category | Crates Involved | Priority |
|---------|----------------|----------------|----------|
| Federation Message Flow (deferred) | Composition | `hkask-*` (deferred to v1.1+) | P2 |
| Competition Socket Protocol (ACP) | Interface | `hkask-agents` (ACP) | P2 |
| Git CAS Content-Addressed Blob Flow | Persistence | `hkask-storage (git_cas)`, `gix 0.81` | P2 |
| Template Manifest Validation Flow (ContractValidator) | Composition | `hkask-templates` | P2 |
| MVSDD Cycle (Specify → Grant → Compose → Curate → Reflect) | Curation | `hkask-templates`, `hkask-agents` | P2 |

> **Note (2026-06-09):** `hkask-mcp-memory` consolidates episodic and semantic memory operations. Its interaction patterns with the memory subsystem are not yet diagrammed and should be considered candidates for v1.1+ coverage.

---

## 8. Summary

| Category | Diagrams | Verified | V1.1+ Candidates |
|----------|----------|----------|-----------------|
| Domain & Capability | 8 | 8 | 0 |
| Interface & Composition | 6 | 6 | 0 |
| Trust & Observability | 7 | 7 | 0 |
| Persistence & Lifecycle | 5 | 5 | 0 |
| Framework | 4 | 4 | 0 |
| Reference | 0 | 0 | 0 |
| **Total** | **30** | **30** | **5** |

**MDS completeness:** All 5 MDS categories have diagram coverage. 30 diagrams verified against current code (2026-06-13).

---

## References

[^mds]: hKask Team. (2026). *MDS — Minimal Domain Specification*. `docs/architecture/MDS.md`.
[^doc-standards]: hKask Team. (2026). *Documentation Standards*. `do../specifications/standards/DOCUMENTATION_STANDARDS.md`.

---

*ℏKask - A Minimal Viable Container for Agents — v0.27.0*
*Mermaid-First Mandate: Every interaction pattern, data flow, and object model is diagrammed.*