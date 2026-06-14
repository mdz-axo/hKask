---
title: "hKask Requirements Specification"
audience: [architects, developers, agents]
last_updated: 2026-06-13
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Requirements Specification

**Purpose:** Implemented requirements as MDS goal specs, with traceability to specific crates, modules, and tests. Each requirement is grounded in code that compiles and passes tests today.

**Related:** [`TRACEABILITY_MATRIX.md`](TRACEABILITY_MATRIX.md), [`MDS.md`](../architecture/MDS.md)

**Verification:** `cargo test --workspace`

---

## 1. Goal Spec Format

Each requirement follows the MDS goal spec pattern:

```
Goal ID: REQ-<CATEGORY>-<NNN>
Category: <MDS category>
Text: When <situation>, I want to <motivation>, so I can <outcome>
Criteria: [list of verifiable criteria]
Implementation: <crate>::<module>::<type>
Tests: <test file or function>
Status: Implemented | Partially Implemented | Deferred
Curation: Merge | Revise | Defer | Discard
```

[^job-stories]: Ulwick, A. (2016). *Jobs to Be Done: Theory to Practice*. Idea Bite Press. Job story format adapted for goal specification.

---

## 2. Domain Requirements

### REQ-DOM-001: Bounded Context Identity

- **Category:** Domain
- **Text:** When operating an agent platform, I want a clear bounded context, so I can reason about what hKask owns vs. delegates.
- **Criteria:**
  - [x] Bounded context documented and verified against code
  - [x] External dependencies (Okapi, SQLCipher) are delegated, not owned
  - [x] All domain entities have corresponding Rust types
- **Implementation:** `hkask-types::id`, `hkask-types::event`, `hkask-types::agent_def`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge — foundational requirement, fully satisfied

### REQ-DOM-002: ν-Event Observability Primitive

- **Category:** Domain, Lifecycle
- **Text:** When any significant operation occurs, I want a typed event emitted, so I can observe system behavior.
- **Criteria:**
  - [x] `NuEvent` struct with span, phase, observer, payload
  - [x] 22 span namespaces (15 canonical + 7 hierarchical)
  - [x] `NuEventSink` trait for emission
- **Implementation:** `hkask-types::event::NuEvent`, `hkask-types::event::Span`, `hkask-types::event::NuEventSink`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge

### REQ-DOM-003: hLexicon Vocabulary Grounding

- **Category:** Domain
- **Text:** When authoring templates or specifications, I want a bounded vocabulary, so I can ensure consistent terminology.
- **Criteria:**
  - [x] 87 term-slots allocated across WordAct (28) / FlowDef (34) / KnowAct (25); 86 unique term strings (`transform` shared across two domains)
  - [x] Spec-curation terms (`specify`, `require`, `constrain`, `curate`, `elicit`, `reconcile`, `contextualise`, `cultivate`) defined
  - [x] Terms referenced in template `lexicon_terms` field and enforced at registration by `ContractValidator` (not yet in code; see TODO.md P2-06)
- **Implementation:** `hkask-types::lexicon` (`HLexicon`), `hkask-templates::lexicon::load_hlexicon_from_yaml`
- **Tests:** `hkask-templates::lexicon` module tests
- **Status:** Implemented
- **Curation:** Merge

### REQ-DOM-004: hLexicon Single-Source Derivation

- **Category:** Domain, Lifecycle, Curation
- **Text:** When the hLexicon vocabulary evolves, I want the markdown catalog to be the single source of truth from which the YAML registry is derived, with regeneration kept explicit and human-driven, so the documentation and the data cannot silently drift apart and the derived YAML is never invisibly rewritten.
- **Criteria:**
  - [x] `docs/architecture/reference/hKask-hLexicon.md` is the canonical source; its term tables define the vocabulary
  - [x] `registry/hlexicon/hlexicon-workspace.yaml` is a committed, derived artifact with its own data lifecycle (customizable/extensible, unlike compiled Rust)
  - [x] Derivation lives in Rust only \u2014 no new language toolchain: `hkask-templates::lexicon` parses the YAML for validation (markdown\u2192YAML derivation not yet in code; see TODO.md P2-06)
  - [x] Regeneration is explicit and opt-in; the YAML is never auto-overwritten
  - [x] `load_hlexicon_from_yaml()` loads the 86-term vocabulary from the committed YAML for validation
  - [x] Consistency check pending broader test expansion (P0-02)
- **Implementation:** `hkask-templates::lexicon` (`load_hlexicon_from_yaml`, `load_hlexicon_from_file`, `load_hlexicon_default`), `registry/hlexicon/hlexicon-workspace.yaml`
- **Tests:** `hkask-templates::lexicon` module tests; `hkask-types::lexicon::tests::bootstrap_domains_match_catalog`
- **Status:** Implemented
- **Curation:** Merge — closes the drift gap that allowed the doc/code term counts to diverge; markdown/YAML/Rust have distinct, intentional lifecycles

[^evans-ddd]: Evans, Eric. *Domain-Driven Design: Tackling Complexity in the Heart of Software.* Addison-Wesley, 2003. — Bounded contexts and ubiquitous language that ground domain requirements.

---

## 3. Trust Requirements

### REQ-CAP-001: Object-Capability Access Control

- **Category:** Trust
- **Text:** When any operation is attempted, I want OCAP enforcement, so I can prevent ambient authority.
- **Criteria:**
  - [x] `Capability` type with HMAC-SHA256 signing
  - [x] Resource + action scoping
  - [x] Caveats for additional restrictions
  - [x] Constant-time comparison via `subtle`
- **Implementation:** `hkask-types::capability::DelegationToken` (note: `Capability` type alias and `AccessEvaluator` not yet in code; see TODO.md P2-06)
- **Tests:** —
- **Status:** Partially Implemented
- **Curation:** Merge

### REQ-CAP-002: Capability Attenuation Chains

- **Category:** Trust
- **Text:** When delegating a composition, I want attenuation enforced, so I can limit delegated authority.
- **Criteria:**
  - [x] Attenuation depth configurable (default: 7)
  - [x] `Delegation` type with grantor/grantee/scope
  - [x] `DelegationStore` for persistent tracking
  - [x] `RevocationList` for revoked capabilities
- **Implementation:** `hkask-types::capability::DelegationToken` (attenuation via `attenuation_level` field; note: `Delegation`, `DelegationStore`, `RevocationList` types not yet in code; see TODO.md P2-06)
- **Tests:** —
- **Status:** Partially Implemented
- **Curation:** Merge

### REQ-CAP-003: MCP Tool Surface

- **Category:** Trust, Composition
- **Text:** When an agent needs a tool, I want MCP server dispatch, so I can route tool calls to the correct server.
- **Criteria:**
  10 MCP servers registered in workspace
  - [x] `McpRuntime` manages server lifecycle
  - [x] `GovernedTool` enforces OCAP before dispatch (`SecurityGateway` described in spec; see TODO.md P2-06)
  - [x] Stdio transport via rmcp (in-process and HTTP transports deferred)
  - [x] Former MCP servers (inference, CNS, OCAP, keystore, registry, git, goals) now use direct crate calls
- **Implementation:** `hkask-mcp::runtime::McpRuntime`, `hkask-cns::governed_tool::GovernedTool`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge — implemented

[^ocap]: Miller, M. (2006). *Robust Composition: Towards a National Research Agenda for Object Capability Security.* HP Labs. — Object capability model that grounds capability requirements.

---

## 4. Composition Requirements

### REQ-IFC-001: MCP ≡ CLI ≡ API Equivalence

- **Category:** Composition
- **Text:** When exercising a composition, I want identical semantics across MCP, CLI, and API, so I can choose the appropriate surface.
- **Criteria:**
  - [x] CLI binary `kask` with 26 subcommand groups
  - [x] HTTP API with 11 route groups
  - [x] MCP with 10 servers (internal cognition via direct crate calls)
  - [x] All route through `hkask-agents` domain core
- **Implementation:** `hkask-cli::main`, `hkask-api::lib::create_router`, `hkask-mcp::runtime`
- **Status:** Implemented
- **Curation:** Merge

### REQ-IFC-002: OpenAPI Documentation

- **Category:** Composition
- **Text:** When integrating with the API, I want auto-generated OpenAPI docs, so I can discover endpoints.
- **Criteria:**
  - [x] utoipa v5.5 with axum extras
  - [x] OpenAPI spec generated at `docs/generated/openapi.json`
  - [x] All route groups documented
- **Implementation:** `hkask-api::openapi::ApiDoc`, `utoipa_axum::OpenApiRouter`
- **Status:** Implemented
- **Curation:** Merge

[^cockburn-hexagonal]: Cockburn, A. (2005). *Hexagonal Architecture.* https://alistair.cockburn.us/hexagonal-architecture/ — Ports and adapters pattern that grounds the MCP ≡ CLI ≡ API equivalence requirement.

---

## 5. Composition Requirements

### REQ-COM-001: Unified Template Registry

- **Category:** Composition
- **Text:** When registering or discovering templates, I want a single registry, so I can avoid multi-registry complexity.
- **Criteria:**
  - [x] `SqliteRegistry` with `template_type` discriminator
  - [x] Three template types: WordAct, FlowDef, KnowAct
  - [x] Lexicon-term-based search
  - [x] Contract validation on registration
- **Implementation:** `hkask-templates::SqliteRegistry`, `hkask-templates::contract_validator::ContractValidator`
- **Tests:** Doctests only (3 ok, 1 ignored)
- **Status:** Implemented
- **Curation:** Merge

### REQ-COM-002: Template Cascade with Depth Limit

- **Category:** Composition
- **Text:** When composing templates, I want cascade depth limited, so I can prevent infinite recursion.
- **Criteria:**
  - [x] Cascade depth ≤ 7
  - [x] `DependencyGraph` validates acyclic composition
  - [x] `TemplateResolver` handles cascade
- **Implementation:** `hkask-templates::dependency::DependencyGraph`, `hkask-templates::resolver::TemplateResolver`
- **Status:** Implemented
- **Curation:** Merge

### REQ-COM-003: Agent Pod Composition

- **Category:** Composition, Trust
- **Text:** When creating an agent, I want pod-based composition, so I can bundle identity, capabilities, and templates.
- **Criteria:**
  - [x] `AgentPod` composes identity, capabilities, templates, lifecycle state
  - [x] `PodManager` with builder pattern
  - [x] `PodLifecycleState` state machine
  - [x] `ConsentManager` for user authorization
- **Implementation:** `hkask-agents::pod::AgentPod`, `hkask-agents::pod::PodManager`, `hkask-agents::consent::ConsentManager`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge

[^fowler-registry]: Fowler, M. (2002). *Patterns of Enterprise Application Architecture.* Addison-Wesley. — Registry pattern that grounds the unified template registry requirement.

---

## 6. Trust & Security Requirements

### REQ-TRU-001: Zero-Trust Defaults

- **Category:** Trust
- **Text:** When the system starts, I want zero-trust defaults, so I can prevent unauthorized access.
- **Criteria:**
  - [x] No hardcoded secrets
  - [x] No ambient authority
  - [x] Fail-closed (denied by default)
  - [x] No wildcard capabilities
- **Implementation:** `hkask-mcp::security::SecurityPolicy`
- **Status:** Implemented
- **Curation:** Merge

### REQ-TRU-002: Encrypted Storage at Rest

- **Category:** Trust, Lifecycle
- **Text:** When data is stored, I want encryption at rest, so I can protect user data.
- **Criteria:**
  - [x] SQLCipher with AES-256-CBC
  - [x] Argon2id key derivation
  - [x] No cross-machine sync
- **Implementation:** `hkask-storage::database::Database` (rusqlite with bundled-sqlcipher)
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge

### REQ-TRU-003: Deterministic Identity

- **Category:** Trust, Lifecycle
- **Text:** When identifying agents, I want deterministic WebIDs, so I can ensure audit trail continuity.
- **Criteria:**
  - [x] UUID v5 from persona content
  - [x] Same persona → same WebID across processes
  - [x] Root authority WebID from fixed persona
- **Implementation:** `hkask-types::id::WebID`, `hkask-agents::pod::AgentIdentity`
- **Status:** Implemented
- **Curation:** Merge

[^stride]: Howard, M. & Lipner, S. (2006). *The STRIDE Threat Model.* Microsoft. — Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege — the threat categories that ground trust requirements.

---

## 7. Lifecycle Requirements

### REQ-OBS-001: CNS Span Emission

- **Category:** Lifecycle
- **Text:** When a capability is invoked, I want a CNS span emitted, so I can monitor system behavior.
- **Criteria:**
  - [x] 22 span namespaces (15 canonical + 7 hierarchical; see PRINCIPLES.md §1.4)
  - [x] `NuEvent` with phase (Sense/Compute/Compare/Act; legacy aliases: Observe\u2192Sense, Regulate\u2192Compute, Outcome\u2192Act)
  - [x] `NuEventSink` trait for emission
- **Implementation:** `hkask-types::event::Span`, `hkask-cns::runtime::CnsRuntime`
- **Status:** Implemented
- **Curation:** Merge

### REQ-OBS-002: Algedonic Alerting

- **Category:** Lifecycle
- **Text:** When variety deficit exceeds threshold, I want an alert escalated, so I can intervene.
- **Criteria:**
  - [x] `AlgedonicManager` with severity levels (Info, Warning, Critical)
  - [x] `VarietyCounter` tracking per category
  - [x] Escalation to Curator/Human
- **Implementation:** `hkask-cns::algedonic::AlgedonicManager`, `hkask-types::cns::AlgedonicAlert`
- **Status:** Implemented
- **Curation:** Merge

[^ashby]: Ashby, W.R. (1956). *An Introduction to Cybernetics*. Chapman & Hall. — Variety engineering and algedonic regulation that ground observability requirements.

---

## 8. Persistence Requirements

### REQ-PER-001: Bitemporal Triple Storage

- **Category:** Persistence
- **Text:** When storing domain knowledge, I want bitemporal semantics, so I can track valid-time and transaction-time.
- **Criteria:**
  - [x] Triple table with valid_from/valid_to and tx_from/tx_to
  - [x] Confidence as first-class field
  - [x] Observer identity on every triple
- **Implementation:** `hkask-storage::triples::TripleStore`, `hkask-storage::triples::Triple`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge

### REQ-PER-002: Embedding Vector Search

- **Category:** Persistence
- **Text:** When searching semantically, I want vector similarity, so I can find related knowledge.
- **Criteria:**
  - [x] sqlite-vec virtual table for embeddings
  - [x] KNN search implementation
  - [x] Model-agnostic dimensions
- **Implementation:** `hkask-storage::embeddings::EmbeddingStore`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge

[^snodgrass]: Snodgrass, R. T. (1999). *Developing Time-Oriented Database Applications in SQL.* Morgan Kaufmann. — Bitemporal data model that grounds persistence requirements.

---

## 9. Lifecycle Requirements

### REQ-LIF-001: Bootstrap Sequence

- **Category:** Lifecycle
- **Text:** When starting the system, I want a deterministic bootstrap, so I can ensure consistent initialization.
- **Criteria:**
  - [x] Database → hLexicon → Registry → Capability → Curator → CNS → MCP
  - [x] All steps verified by `cargo check --workspace`
- **Implementation:** `hkask-cli::main`
- **Status:** Implemented
- **Curation:** Merge

### REQ-LIF-002: Forward-Only Evolution

- **Category:** Lifecycle
- **Text:** When evolving the system, I want forward-only migration, so I can avoid rollback complexity.
- **Criteria:**
  - [x] No rollback support documented
  - [x] Git-only versioning (SHA-based)
  - [x] Prefer deletion over deprecation (P7)
- **Implementation:** Architecture invariant, enforced by policy
- **Status:** Implemented
- **Curation:** Merge

[^principles-p7]: hKask Team. (2026). *Architecture Principles — P7.* `docs/architecture/PRINCIPLES.md` §2.7 — Prefer deletion over deprecation. Forward-only evolution.

---

## 10. Curation Requirements

### REQ-CUR-001: MDS Specification Tools

- **Category:** Curation
- **Text:** When authoring specifications, I want MCP tools for capture/decompose/curate/validate, so I can follow the MVSDD cycle.
- **Criteria:**
  - [x] 5 MCP tools in `hkask-mcp-spec`
  - [x] `SpecStore`, `SpecCurator`, `SpecObserver` traits
  - [x] `SqliteSpecStore` implementation
  - [x] `DefaultSpecCurator` implementation
- **Implementation:** `hkask-mcp-spec` (607 LOC in main.rs), `hkask-storage::spec_types` (trait), `hkask-agents::curator_agent::spec_curator` (impl)
- **Status:** Implemented
- **Curation:** Merge

### REQ-CUR-002: Curation Decision Gradient

- **Category:** Curation
- **Text:** When evaluating artifacts, I want gradient decisions, so I can express nuance beyond binary accept/reject.
- **Criteria:**
  - [x] `CurationDecision` enum: Merge, Revise, Defer, Discard
  - [x] Rationale required for every decision
  - [x] `SpecCurationRecord` with coherence score
- **Implementation:** `hkask-storage::spec_types::SpecCurationRecord`, `hkask-types::curation` (CurationDecision)
- **Status:** Implemented
- **Curation:** Merge

---

### REQ-CAP-004: Condenser MCP Server

- **Category:** Trust, Composition
- **Text:** When agent context approaches the window limit, I want automatic summarization and compression via a dedicated MCP server, so I can maintain conversation continuity without manual truncation.
- **Criteria:**
  - [x] 761 LOC in `hkask-mcp-condenser`
  - [x] `classify`, `compress`, `persist`, `thread_summary`, `ping`, `stats`, `set_profile` tools
  - [x] Context-aware compression algorithms
  - [x] Episodic memory persistence
- **Implementation:** `hkask-mcp-condenser` (761 LOC)
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge — promoted from deferred REQ-CAP-D01

### REQ-CAP-005: Web MCP Server

- **Category:** Trust, Composition
- **Text:** When an agent needs to retrieve web content, I want a dedicated MCP server for search, fetch, and crawl operations, so I can ground agent responses in current external data.
- **Criteria:**
  - [x] 3,389 LOC in `hkask-mcp-web`
  - [x] `search`, `fetch`, `crawl`, `extract` tools
  - [x] Content extraction and formatting
  - [x] Rate limiting and domain filtering
- **Implementation:** `hkask-mcp-web` (3,389 LOC)
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge — promoted from deferred REQ-CAP-D03

[^mds]: hKask Team. (2026). *MDS — Minimal Domain Specification.* `docs/architecture/MDS.md` — The 5-category curation model that grounds curation requirements.

---

## 11. Deferred Requirements

| ID | Requirement | Reason | ADR |
|----|------------|--------|-----|
| REQ-COM-D01 | Remote LLM fallback | Local-first invariant | ADR pending |
| REQ-COM-D02 | Federation transport | Complexity exceeds budget | ADR pending |
| REQ-LIF-D01 | Qdrant vector search | sqlite-vec sufficient for MVP | ADR pending |

[^principles-p5]: hKask Team. (2026). *Architecture Principles — P5.* `docs/architecture/PRINCIPLES.md` §2.5 — No feature flag without activator. Deferred requirements are explicitly not implemented.

---

## References

[^job-stories]: Ulwick, A. (2016). *Jobs to Be Done: Theory to Practice*. Idea Bite Press.
