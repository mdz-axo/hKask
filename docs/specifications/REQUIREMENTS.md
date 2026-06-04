---
title: "hKask Requirements Specification"
audience: [architects, developers, agents]
last_updated: 2026-05-29
version: "1.2.0"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# hKask Requirements Specification

**Purpose:** Implemented requirements as DDMVSS goal specs, with traceability to specific crates, modules, and tests. Each requirement is grounded in code that compiles and passes tests today.

**Related:** [`TRACEABILITY_MATRIX.md`](TRACEABILITY_MATRIX.md), [`domain-and-capability.md`](../architecture/domain-and-capability.md), [`DDMVSS.md`](../architecture/DDMVSS.md)

**Verification:** `cargo test --workspace`

---

## 1. Goal Spec Format

Each requirement follows the DDMVSS goal spec pattern:

```
Goal ID: REQ-<CATEGORY>-<NNN>
Category: <DDMVSS category>
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

- **Category:** Domain, Observability
- **Text:** When any significant operation occurs, I want a typed event emitted, so I can observe system behavior.
- **Criteria:**
  - [x] `NuEvent` struct with span, phase, observer, payload
  - [x] 10 span namespaces covering all operations
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
  - [x] Terms referenced in template `lexicon_terms` field and enforced at registration by `ContractValidator`
- **Implementation:** `hkask-types::lexicon` (`HLexicon`), `hkask-templates::hlexicon_source::load_workspace_lexicon`, `hkask-templates::contract_validator::ContractValidator`
- **Tests:** `hkask-templates::hlexicon_source::tests::workspace_yaml_loads`
- **Status:** Implemented
- **Curation:** Merge

### REQ-DOM-004: hLexicon Single-Source Derivation

- **Category:** Domain, Lifecycle, Curation
- **Text:** When the hLexicon vocabulary evolves, I want the markdown catalog to be the single source of truth from which the YAML registry is derived, with regeneration kept explicit and human-driven, so the documentation and the data cannot silently drift apart and the derived YAML is never invisibly rewritten.
- **Criteria:**
  - [x] `docs/architecture/reference/hKask-hLexicon.md` is the canonical source; its term tables define the vocabulary
  - [x] `registry/registries/hlexicon-workspace.yaml` is a committed, derived artifact with its own data lifecycle (customizable/extensible, unlike compiled Rust)
  - [x] Derivation lives in Rust only — no new language toolchain: `hkask-templates::hlexicon_source` parses the markdown and renders the YAML
  - [x] Regeneration is explicit and opt-in (`#[ignore]`d `regenerate_workspace_yaml` test); the YAML is never auto-overwritten
  - [x] `load_workspace_lexicon()` loads the 86-term vocabulary from the committed YAML for validation
  - [x] Consistency check `hlexicon_yaml_matches_markdown` rides `cargo test --workspace` and fails on drift, prompting the maintainer to regenerate or restore the markdown from git
- **Implementation:** `hkask-templates::hlexicon_source` (`parse_markdown_catalog`, `render_workspace_yaml`, `load_workspace_lexicon`, `regenerate_workspace_yaml`), `registry/registries/hlexicon-workspace.yaml`
- **Tests:** `hkask-templates::hlexicon_source::tests::{markdown_parses_to_expected_counts, hlexicon_yaml_matches_markdown, workspace_yaml_loads}` (consistency rides `cargo test --workspace`); `hkask-types::lexicon::tests::bootstrap_domains_match_catalog`
- **Status:** Implemented
- **Curation:** Merge — closes the drift gap that allowed the doc/code term counts to diverge; markdown/YAML/Rust have distinct, intentional lifecycles

---

## 3. Capability Requirements

### REQ-CAP-001: Object-Capability Access Control

- **Category:** Capability, Trust
- **Text:** When any operation is attempted, I want OCAP enforcement, so I can prevent ambient authority.
- **Criteria:**
  - [x] `Capability` type with HMAC-SHA256 signing
  - [x] Resource + action scoping
  - [x] Caveats for additional restrictions
  - [x] Constant-time comparison via `subtle`
- **Implementation:** `hkask-types::visibility::Capability`, `hkask-types::visibility::AccessEvaluator`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge

### REQ-CAP-002: Capability Attenuation Chains

- **Category:** Capability, Trust
- **Text:** When delegating a capability, I want attenuation enforced, so I can limit delegated authority.
- **Criteria:**
  - [x] Attenuation depth configurable (default: 7)
  - [x] `Delegation` type with grantor/grantee/scope
  - [x] `DelegationStore` for persistent tracking
  - [x] `RevocationList` for revoked capabilities
- **Implementation:** `hkask-types::visibility::Delegation`, `hkask-types::visibility::DelegationStore`, `hkask-types::visibility::RevocationList`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge

### REQ-CAP-003: MCP Tool Surface

- **Category:** Capability, Interface
- **Text:** When an agent needs a tool, I want MCP server dispatch, so I can route tool calls to the correct server.
- **Criteria:**
  15 MCP servers registered in workspace
  - [x] `McpRuntime` manages server lifecycle
  - [x] `SecurityGateway` enforces OCAP before dispatch
  - [x] Three transport options (in-process, stdio, HTTP)
- **Implementation:** `hkask-mcp::runtime::McpRuntime`, `hkask-mcp::security::SecurityGateway`, `hkask-mcp::transport::*`
- **Tests:** —
- **Status:** Implemented
- **Curation:** Merge — formerly listed as stubs, now implemented

---

## 4. Interface Requirements

### REQ-IFC-001: MCP ≡ CLI ≡ API Equivalence

- **Category:** Interface
- **Text:** When exercising a capability, I want identical semantics across MCP, CLI, and API, so I can choose the appropriate surface.
- **Criteria:**
  - [x] CLI binary `kask` with 14 subcommand groups
  - [x] HTTP API with 11 route groups
  [x] MCP with 15 servers
  - [x] All route through `hkask-agents` domain core
- **Implementation:** `hkask-cli::main`, `hkask-api::lib::create_router`, `hkask-mcp::runtime`
- **Status:** Implemented
- **Curation:** Merge

### REQ-IFC-002: OpenAPI Documentation

- **Category:** Interface
- **Text:** When integrating with the API, I want auto-generated OpenAPI docs, so I can discover endpoints.
- **Criteria:**
  - [x] utoipa v5.5 with axum extras
  - [x] OpenAPI spec generated at `docs/generated/openapi.json`
  - [x] All route groups documented
- **Implementation:** `hkask-api::openapi::ApiDoc`, `utoipa_axum::OpenApiRouter`
- **Status:** Implemented
- **Curation:** Merge

---

## 5. Composition Requirements

### REQ-COM-001: Unified Template Registry

- **Category:** Composition
- **Text:** When registering or discovering templates, I want a single registry, so I can avoid multi-registry complexity.
- **Criteria:**
  - [x] `SqliteRegistry` with `template_type` discriminator
  - [x] Four template types: Prompt, Process, Cognition, Specification
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

- **Category:** Composition, Capability
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
- **Implementation:** `hkask-mcp::security::SecurityPolicy`, `hkask-types::visibility::AccessEvaluator`
- **Status:** Implemented
- **Curation:** Merge

### REQ-TRU-002: Encrypted Storage at Rest

- **Category:** Trust, Persistence
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

---

## 7. Observability Requirements

### REQ-OBS-001: CNS Span Emission

- **Category:** Observability
- **Text:** When a capability is invoked, I want a CNS span emitted, so I can monitor system behavior.
- **Criteria:**
  - [x] 10 span namespaces (Prompt, Tool, AgentPod, Connector, Pipeline, Energy, Review, Sovereignty, Goal, Spec)
  - [x] `NuEvent` with phase (Observe/Regulate/Outcome)
  - [x] `NuEventSink` trait for emission
- **Implementation:** `hkask-types::event::Span`, `hkask-cns::runtime::CnsRuntime`
- **Status:** Implemented
- **Curation:** Merge

### REQ-OBS-002: Algedonic Alerting

- **Category:** Observability
- **Text:** When variety deficit exceeds threshold, I want an alert escalated, so I can intervene.
- **Criteria:**
  - [x] `AlgedonicManager` with severity levels (Info, Warning, Critical)
  - [x] `VarietyCounter` tracking per category
  - [x] Escalation to Curator/Human
- **Implementation:** `hkask-cns::algedonic::AlgedonicManager`, `hkask-types::cns::AlgedonicAlert`
- **Status:** Implemented
- **Curation:** Merge

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

---

## 10. Curation Requirements

### REQ-CUR-001: DDMVSS Specification Tools

- **Category:** Curation
- **Text:** When authoring specifications, I want MCP tools for capture/decompose/curate/validate, so I can follow the MVSDD cycle.
- **Criteria:**
  - [x] 8 MCP tools in `hkask-mcp-spec`
  - [x] `SpecStore`, `SpecCurator`, `SpecObserver` traits
  - [x] `SqliteSpecStore` implementation
  - [x] `DefaultSpecCurator` implementation
- **Implementation:** `hkask-mcp-spec` (819 LOC), `hkask-storage::spec_types` (trait), `hkask-agents::curator::spec_curator` (impl)
- **Status:** Implemented
- **Curation:** Merge

### REQ-CUR-002: Curation Decision Gradient

- **Category:** Curation
- **Text:** When evaluating artifacts, I want gradient decisions, so I can express nuance beyond binary accept/reject.
- **Criteria:**
  - [x] `CurationDecision` enum: Merge, Revise, Defer, Discard
  - [x] Rationale required for every decision
  - [x] `SpecCurationRecord` with coherence score
- **Implementation:** `hkask-types::spec::SpecCurationRecord`, `hkask-types::curation` (CurationDecision)
- **Status:** Implemented
- **Curation:** Merge

---

## 11. Deferred Requirements

| ID | Requirement | Reason | ADR |
|----|------------|--------|-----|
| REQ-CAP-D01 | Full condenser MCP server | Implemented (761 LOC) — no longer a stub | — |
| REQ-CAP-D03 | Full web MCP server | Implemented (3,389 LOC) — no longer a stub | — |
| REQ-IFC-D01 | Remote LLM fallback | Local-first invariant | ADR pending |
| REQ-COM-D01 | Federation transport | Complexity exceeds budget | ADR pending |
| REQ-PER-D01 | Qdrant vector search | sqlite-vec sufficient for MVP | ADR pending |

---

## References

[^job-stories]: Ulwick, A. (2016). *Jobs to Be Done: Theory to Practice*. Idea Bite Press.
