---
title: "hKask Requirements Specification"
audience: [architects, developers, agents]
last_updated: 2026-07-04
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Requirements Specification

**Purpose:** Implemented requirements as MDS goal specs, with traceability to specific crates, modules, and tests. Each requirement is grounded in code that compiles and passes tests today.

**Related:** [`MDS.md`](../architecture/core/MDS.md)

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
- **Text:** When using hKask as a human user (via `kask chat`), I want a clear bounded context, so I can reason about what hKask owns vs. delegates.
- **Criteria:**
  - [x] Bounded context documented and verified against code
  - [x] External dependencies (Okapi, SQLCipher) are delegated, not owned
  - [x] All domain entities have corresponding Rust types
- **Implementation:** `hkask-types::id`, `hkask-types::event`, `hkask-types::agent_registry`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge — foundational requirement, fully satisfied

### REQ-DOM-002: ν-Event Observability Primitive

- **Category:** Domain, Lifecycle
- **Text:** When any significant operation occurs, I want a typed event emitted, so I can observe system behavior.
- **Criteria:**
  - [x] `RegulationRecord` struct with span, phase, observer, payload
  - [x] 22 span namespaces (15 canonical + 7 hierarchical)
  - [x] `RegulationSink` trait for emission
- **Implementation:** `hkask-types::event::RegulationRecord`, `hkask-types::event::Span`, `hkask-types::event::RegulationSink`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge

### REQ-DOM-003: Vocabulary Grounding

- **Category:** Domain
- **Text:** When authoring templates or specifications, I want a bounded vocabulary, so I can ensure consistent terminology.
- **Criteria:**
  - [x] 120 term-slots bootstrapped from manifest `lexicon_terms` across the skill corpus
  - [x] Terms referenced in template `lexicon_terms` field and validated at registration via `hkask-templates::vocabulary::validate_entry`
- **Implementation:** `hkask-templates::vocabulary` (`is_known`, `unrecognized`, `validate_entry`)
- **Tests:** `hkask-templates::vocabulary` unit tests
- **Status:** Implemented
- **Curation:** Merge

### REQ-DOM-004: Vocabulary Single-Source

- **Category:** Domain, Lifecycle, Curation
- **Text:** When the vocabulary evolves, I want a single source of truth so documentation and validation cannot silently drift apart.
- **Criteria:**
  - [x] `crates/hkask-templates/src/vocabulary.rs` is the canonical source; the sorted `KNOWN_TERMS` array defines the vocabulary
  - [x] Terms are validated at registration time by `Registry::register()` and `SqliteRegistry::register()`
  - [x] New terms are added directly to the `KNOWN_TERMS` array (maintain sorted order)
- **Implementation:** `hkask-templates::vocabulary` (`KNOWN_TERMS`, `is_known`, `validate_entry`)
- **Tests:** `hkask-templates::vocabulary` unit tests
- **Status:** Implemented
- **Curation:** Merge — vocabulary is now compile-time embedded in Rust; no separate YAML artifact needed

### REQ-DOM-005: Codebase Understanding via Semantic Code Graph

- **Category:** Domain, Composition
- **Text:** When I interact with an agent in the codebase, I want it to understand the code it's operating on so it can answer structural questions, trace dependencies, assess change impact, and assemble relevant context for LLM prompts without me manually specifying files.
- **Criteria:**
  - [x] Agents can search the codebase with FTS5 keyword search over symbol names, signatures, and doc comments (`codegraph_query`)
  - [x] Agents can traverse dependency chains forward (dependencies) and reverse (callers) via recursive CTE (`codegraph_traverse`)
  - [x] Agents can assess blast radius of changing a symbol with risk classification — Critical (public traits), High (public types), Medium (impls), Low (private/test) (`codegraph_impact`)
  - [x] Agents can detect dead code (symbols with zero inbound non-test edges, non-public, not in test modules) (`codegraph_analysis`)
  - [x] Agents can assemble token-budgeted context for LLM prompts at four tiers: Minimal (512), Focused (2048), Standard (4096), Full (8192) tokens (`codegraph_context`)
  - [x] Indexing is incremental — SHA-256 content hash comparison skips unchanged files on re-index
  - [x] Index staleness is Regulation-observable — drives algedonic alerts when the index is stale
  - [x] Context feedback loop tracks symbol usage ratio to improve future assembly (`codegraph_feedback`)
- **Implementation:** `hkask-mcp-codegraph` (domain crate: types, graph store, indexer, search, traversal, analysis, context assembly), `hkask-mcp-codegraph` (MCP server: 11 tools)
- **Tests:** `component1_parser.rs`, `component3_pipeline.rs`, `component7_impact.rs` (22 tests total)
- **Status:** Implemented (v0.31.0)
- **Curation:** Merge — two-crate pattern matches `hkask-condenser`; no service layer needed

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
- **Implementation:** `hkask-capability::DelegationToken` (note: `Capability` type alias and `AccessEvaluator` not yet in code; see TODO.md P2-06)
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
- **Implementation:** `hkask-capability::DelegationToken` (attenuation via `attenuation_level` field; note: `Delegation`, `DelegationStore`, `RevocationList` types not yet in code; see TODO.md P2-06)
- **Tests:** —
- **Status:** Partially Implemented
- **Curation:** Merge

### REQ-CAP-003: MCP Tool Surface

- **Category:** Trust, Composition
- **Text:** When an agent needs a tool, I want MCP server dispatch, so I can route tool calls to the correct server.
- **Criteria:**
  - [x] 15 MCP servers registered in workspace
  - [x] `McpRuntime` manages server lifecycle
  - [x] `GovernedTool` enforces OCAP before dispatch (`SecurityGateway` described in spec; see TODO.md P2-06)
  - [x] Stdio transport via rmcp (in-process and HTTP transports deferred)
  - [x] Former MCP servers (inference, Regulation, OCAP, keystore, registry, git, goals) now use direct crate calls
- **Implementation:** `hkask-mcp::runtime::McpRuntime`, `hkask-regulation::governed_tool::GovernedTool`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge — implemented

[^ocap]: Miller, M. (2006). *Robust Composition: Towards a National Research Agenda for Object Capability Security.* HP Labs. — Object capability model that grounds capability requirements.

---

## 4. Composition Requirements

### REQ-IFC-001: MCP ≡ CLI ≡ API Equivalence

- **Category:** Composition
- **Text:** When exercising a composition, I want identical semantics across MCP, CLI, and API **for core operations**, so I can choose the appropriate surface. Spec capture/list/validate/cultivate are **CLI + API + QA only**; MCP exposes **spec drift** via the Curator server.
- **Criteria:**
  - [x] CLI binary `kask` with 26 subcommand groups
  - [x] HTTP API with 11 route groups
  - [x] MCP with 16 servers (internal cognition via direct crate calls)
  - [x] Core operations route through `hkask-pods` domain core
  - [x] Spec lifecycle operations are CLI + API + QA only (MCP excludes spec capture/list/validate/cultivate)
- **Implementation:** `hkask-cli::main`, `hkask-api::lib::create_router`, `hkask-mcp::runtime`, `hkask-mcp-curator` (spec drift)
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
- **Text:** When the human user's sovereign container (userpod) is provisioned, I want pod-based composition so identity, capabilities, and templates are bundled for the user's `kask chat` session.
- **Criteria:**
  - [x] `AgentPod` composes identity, capabilities, templates, lifecycle state
  - [x] `ActivePods` with builder pattern
  - [x] `PodLifecycleState` state machine
  - [x] `ConsentManager` for user authorization
- **Implementation:** `hkask-pods::pod::AgentPod`, `hkask-pods::pod::ActivePods`, `hkask-pods::consent::ConsentManager`
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
- **Implementation:** `hkask-types::id::WebID`, `hkask-pods::pod::AgentIdentity`
- **Status:** Implemented
- **Curation:** Merge

[^stride]: Howard, M. & Lipner, S. (2006). *The STRIDE Threat Model.* Microsoft. — Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege — the threat categories that ground trust requirements.

---

## 7. Lifecycle Requirements

### REQ-OBS-001: Regulation Span Emission

- **Category:** Lifecycle
- **Text:** When a capability is invoked, I want a Regulation span emitted, so I can monitor system behavior.
- **Criteria:**
  - [x] 22 span namespaces (15 canonical + 7 hierarchical; see canonical Regulation span registry: `crates/hkask-types/src/regulation.rs` (`RegulationSpan`))
  - [x] `RegulationRecord` with phase (Sense/Compute/Compare/Act; legacy aliases: Observe\u2192Sense, Regulate\u2192Compute, Outcome\u2192Act)
  - [x] `RegulationSink` trait for emission
- **Implementation:** `hkask-types::event::Span`, `hkask-regulation::runtime::RegulationLedger`
- **Status:** Implemented
- **Curation:** Merge

### REQ-OBS-002: Algedonic Alerting

- **Category:** Lifecycle
- **Text:** When variety deficit exceeds threshold, I want an alert escalated, so I can intervene.
- **Criteria:**
  - [x] `AlgedonicManager` with severity levels (Info, Warning, Critical)
  - [x] `VarietyCounter` tracking per category
  - [x] Escalation to Curator/Human
- **Implementation:** `hkask-regulation::algedonic::AlgedonicManager`, `hkask-types::regulation::AlgedonicAlert`
- **Status:** Implemented
- **Curation:** Merge

[^ashby]: Ashby, W.R. (1956). *An Introduction to Cybernetics*. Chapman & Hall. — Variety engineering and algedonic regulation that ground observability requirements.

---

## 8. Persistence Requirements

### REQ-PER-001: Bitemporal hMem Storage

- **Category:** Persistence
- **Text:** When storing domain knowledge, I want bitemporal semantics, so I can track valid-time and transaction-time.
- **Criteria:**
  - [x] hMem table with valid_from/valid_to and tx_from/tx_to
  - [x] Confidence as first-class field
  - [x] Observer identity on every hMem
- **Implementation:** `hkask-storage::triples::hMemStore`, `hkask-storage::triples::hMem`
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

### REQ-CUR-001: MDS Specification Persistence

- **Category:** Curation
- **Text:** When authoring specifications, I want CLI/API/QA surfaces to capture, list, validate, and check specs against MDS categories.
- **Criteria:**
  - [x] `kask spec {capture,list,validate,cultivate,render}` CLI commands
  - [x] `GET/POST /api/specs` REST endpoints
  - [ ] `kask qa spec-check` for collection-wide validation (planned, not yet built)
  - [x] `SpecStore`, `SpecCurator`, `SpecObserver` traits
  - [x] `SqliteSpecStore` implementation
  - [x] `DefaultSpecCurator` implementation
- **Implementation:** `hkask-storage::spec_types` (domain types), `hkask-storage::spec_store` (persistence), `hkask-pods::curator_agent::spec_curator` (validation)
- **Status:** Implemented
- **Note:** Former `mcp-spec` MCP server (12 tools) and `SpecService` service layer removed v0.31.0. Spec operations now call `SpecStore` directly; validation folded into QA; prose rewriting moved to `hkask-mcp-replica::replica_rewrite`.
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

### REQ-CAP-005: Research MCP Server

- **Category:** Trust, Composition
- **Text:** When an agent needs to retrieve web content, I want a dedicated MCP server for search, fetch, and crawl operations, so I can ground agent responses in current external data.
- **Criteria:**
  - [x] 1,044 LOC in `hkask-mcp-research`
  - [x] `search`, `fetch`, `crawl`, `extract` tools
  - [x] Content extraction and formatting
  - [x] Rate limiting and domain filtering
- **Implementation:** `hkask-mcp-research` (1,044 LOC)
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge — promoted from deferred REQ-CAP-D03

---

### REQ-COM-004: Matrix Communication Transport

- **Category:** Curation, Composition
- **Text:** When the human user opts into cross-device or federated communication, I want Matrix-based transport with Regulation-observable message flow, so I can interact with my chat session from mobile devices and federate across hKask instances. Matrix is opt-in infrastructure; the primary surface is `kask chat`.
- **Criteria:**
  - [x] `hkask-communication` core infrastructure crate (952 LOC)
  - [x] `MatrixTransport` — matrix-sdk wrapper: login, send/receive, rooms, files
  - [x] `SevenR7Listener` — passive room observer, polls Matrix rooms, persists Regulation RegulationRecords
  - [x] `AgentRegistry` — WebID↔Matrix UserId mapping, thread watchlists
  - [x] Regulation bridge — communication events flow to `RegulationArchive` → curation inbox
  - [x] CAT engagement gate — `convergence_bias` scalar per agent
  - [x] Response dispatch — agent responses routed back via Matrix rooms
  - [x] CLI: `kask matrix deploy-sidecar`, `register`, `listen`, `status-sidecar`
  - [x] Integration tests (652 LOC, Conduit-dependent, `#[ignore]`d)
  - [ ] E2EE (deferred — SQLCipher/SQLite linking conflict)
  - [ ] Continuous sync (deferred — v1 uses on-demand polling)
- **Implementation:** `crates/hkask-communication` (matrix.rs, listener.rs, agent_registration.rs)
- **Tests:** `crates/hkask-communication/tests/` (integration_test.rs, matrix_transport_tests.rs)
- **Status:** Implemented
- **Curation:** Merge — promoted from deferred

[^mds]: hKask Team. (2026). *MDS — Minimal Domain Specification.* `docs/architecture/MDS.md` — The 5-category curation model that grounds curation requirements.

---

### REQ-COM-005: WebSocket Streaming Chat

- **Category:** Composition, Lifecycle
- **Text:** When the human user chooses a browser-based chat experience, I want a persistent WSS (WebSocket Secure) chat endpoint with streaming token output and MCP tool integration, so I can have real-time bidirectional conversations with full memory pipeline support.
- **Criteria:**
  - [x] `GET /api/v1/chat/ws` — WebSocket upgrade with session cookie auth
  - [x] JSON protocol: `{"type":"prompt"}` → `{"type":"token",...}` → `{"type":"done",...}`
  - [x] `ChatService::chat_stream()` — full pipeline: prepare_chat → streaming inference → episodic storage (sovereignty-gated)
  - [x] `ChatStreamEvent` enum — transport-agnostic streaming event type (Token, Done, Error)
  - [x] MCP tool auto-discovery — tools resolved at connection time and passed to inference
  - [x] `generate_stream_with_model()` accepts `tools: Option<&[ChatToolDefinition]>` — threaded through all 5 backends
  - [x] Multi-turn over single persistent connection
  - [x] Ping/Pong keepalive
  - [ ] Cancel mid-generation (Phase 2)
  - [ ] Bearer token auth (Phase 2)
- **Implementation:** `crates/hkask-api/src/routes/chat_ws.rs`, `crates/hkask-services-chat/src/chat/service.rs` (`chat_stream()`), `crates/hkask-types/src/inference_port.rs` (tools in streaming trait)
- **Tests:** — (integration test deferred to Phase 2)
- **Status:** Implemented (Phase 1)
- **Curation:** Merge — foundational for browser-based chat UIs

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
