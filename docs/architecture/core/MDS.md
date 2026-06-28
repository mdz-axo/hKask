---
title: "MDS — Minimal Domain Specification"
audience: [architects, developers, agents]
last_updated: 2026-06-18
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# MDS — Minimal Domain Specification

**Purpose:** A minimal, capability-driven specification framework for hKask. Specs are grants ("CAN verb on resource via interface"), not fences ("MUST NOT"). Five categories, five tools, one completeness predicate.

**Supersedes:** The previous 9-category DDMVSS. All MDS references in the codebase should be updated.

**Related:** [`PRINCIPLES.md`](PRINCIPLES.md), [`magna-carta.md`](magna-carta.md), [`FUNCTIONAL_SPECIFICATION.md`](FUNCTIONAL_SPECIFICATION.md), [`loop-architecture.md`](../loop-architecture.md)

---

## 1. Domain Ontology

### 1.1 Core Entities

| Entity | Crate | Description | Goal Principle |
|--------|-------|-------------|---------------|
| `HumanUser` | `hkask-storage` | Human identity with WebID, role (Admin\|Member), OAuth provider link | P1 |
| `Replicant` | `hkask-types` | Agent identity with persona, voice, wallet link | P6 |
| `AgentPod` | `hkask-agents` | Runtime container for a replicant (Inactive\|Active\|ServerMode) | P1 |
| `Wallet` | `hkask-wallet` | rJoule balance, encumbrance, multi-chain deposits | P9 |
| `ApiKey` | `hkask-wallet` | Scoped API key with spending limits and expiry | P1 |
| `Triple` | `hkask-storage` | Entity-Attribute-Value knowledge representation, bitemporal | P3 |
| `CnsRuntime` | `hkask-cns` | Cybernetic nervous system — variety monitoring, alerts, energy budgets | P9 |
| `EnergyBudget` | `hkask-cns` | Per-agent gas budget with cap, replenish rate, hold-settle pattern | P9 |
| `CircuitBreaker` | `hkask-cns` | Failure-gating state machine for external service calls | P9 |

### 1.2 Kanban Domain

**Crate:** `hkask-services-kanban` | **Goal Principle:** P3 (Generative Space) — agent coordination via headless task boards

| Entity | Description | Key Attributes |
|--------|-------------|---------------|
| `Board` | Named task board scoped to owner WebID | `board_id: BoardId`, `name`, `owner: WebID`, `columns: Vec<Column>` |
| `Column` | Ordered column on a board representing a workflow phase | `column_id: ColumnId`, `name`, `order: u32`, `tasks: Vec<TaskId>` |
| `Task` | Unit of work with status lifecycle, priority, verification criteria | `task_id: TaskId`, `title`, `status: TaskStatus`, `priority: Priority`, `owner: WebID`, `board_id: BoardId`, `column_id: ColumnId` |
| `Priority` | Task urgency level | `Low \| Medium \| High \| Critical` |
| `TaskStatus` | Strict column-ordered lifecycle state | `Backlog → Ready → InProgress → Review → Done` |
| `VerificationCriteria` | Acceptance spec with optional LLM evaluation prompt | `description: String`, `llm_prompt: Option<String>` |
| `Comment` | Task discussion thread entry | `comment_id: CommentId`, `task_id: TaskId`, `author: WebID`, `body: String` |

**CNS spans:** `cns.tool.kanban` — TaskCreated, TaskMoved, TaskAssigned, TaskVerified, BoardCreated

**Key contracts:** 34 `KAN-SVC-*` IDs (migration to `P{N}-svc-kanban-*` in progress)

### 1.3 Kata Domain

**Crate:** `hkask-services-kata` | **Goal Principle:** P3 (Generative Space) — Toyota Kata scientific thinking for agent capability development

| Entity | Description | Key Attributes |
|--------|-------------|---------------|
| `KataEngine` | Orchestrates kata cycles (starter, improvement, coaching) | `state: KataState`, `manifest: KataManifest` |
| `KataState` | Current state of a kata practice cycle | `cycle: CyclePhase`, `observations: Vec<Observation>` |
| `KataManifest` | Declarative definition of a kata (steps, coaching questions, routines) | `meta: ManifestMeta`, `gas_config: GasConfig`, `steps: Vec<KataStep>`, `routines: Vec<PracticeRoutine>`, `cns: CnsConfig` |
| `KataStep` | A single step in a kata improvement cycle | `name: String`, `description: String`, `coach_question: CoachQuestion` |
| `CoachQuestion` | One of the 5 coaching kata questions | `number: u8`, `text: String` |
| `PracticeRoutine` | Deliberate practice routine (Five Questions Drill, PDCA, Observation) | `name: String`, `duration_minutes: u32`, `outcomes: Vec<StarterOutcome>` |
| `KataHistory` | Recorded history of practice sessions | `entries: Vec<PracticeEntry>` |
| `PracticeEntry` | A single practice session record | `routine: String`, `started_at: DateTime<Utc>`, `completed_at: Option<DateTime<Utc>>`, `score: Option<f64>` |

**5 coaching kata questions:** (1) Target condition? (2) Actual condition now? (3) What obstacles? Which ONE? (4) Next step? What do you expect? (5) How quickly can we go and see?

**CNS spans:** `cns.kata` — KataImprovEffectiveness, coaching loop events

**Key contracts:** 27 `P9-svc-kata-*`, `P3-svc-kata-*`, `P7-svc-kata-*` IDs

### 1.4 Adapter Domain

**Crate:** `hkask-adapter` | **Goal Principle:** P3 (Generative Space) — LoRA adapter lifecycle management for agent-specialized inference

| Entity | Description | Key Attributes |
|--------|-------------|---------------|
| `TrainedLoRAAdapter` | A trained LoRA adapter with provenance metadata | `adapter_id: String`, `source: AdapterSource`, `checksum: Checksum`, `expertise: Expertise`, `owner: WebID` |
| `AdapterSource` | Provenance of the adapter | `Local { path } \| Remote { url, sha256 } \| Registry { package_id }` |
| `AdapterStore` | CRUD store for trained adapters with checksum verification | Store, get_by_id, delete, list by owner |
| `AdapterRouter` | Routes inference requests to the best-matching adapter | CompositionEstimate, provider selection, endpoint guard |
| `EndpointLifecycle` | State machine for inference endpoint lifecycle | `EndpointPhase`: `Cold \| Warming \| Active \| Draining \| Removed` |
| `EndpointPhase` | Lifecycle phase of a deployed inference endpoint | `Cold → Warming → Active → Draining → Removed` |
| `AdapterConfig` | Configuration for adapter deployment | `model_id`, `base_url`, `timeout_secs`, `max_concurrency` |
| `Expertise` | Describes the domain expertise of a trained adapter | `domains: Vec<MdsDomain>`, `provenance: TrainingProvenance`, `capabilities: Vec<String>` |
| `CompositionEstimate` | Cost/time estimate for adapter composition | `estimated_cost_rj: f64`, `estimated_latency_ms: u64` |
| `ProviderSelection` | Selected inference provider for an adapter endpoint | `provider: String`, `model: String`, `cost_per_token_rj: f64` |

**CNS spans:** `cns.adapter` — AdapterStored, AdapterRetrieved, AdapterDeleted, endpoint lifecycle transitions

**Key contracts:** 44 pub fns with `expect:` + `[P{N}]` annotations

### 1.5 Service Layer Subsystems

**Crate:** `hkask-services` + subcrates | **Goal Principle:** P5 (Essentialism) — thin orchestration layer, delegates to domain crates

| Subcrate | Domain | Contract Prefix | Count | Status |
|----------|--------|----------------|-------|--------|
| `hkask-services` | Archival, bundle, chat, CNS, compose, consolidation, curator, experience, skills | `P{N}-svc-{domain}-*` | 102 | ✅ Realigned |
| `hkask-services-runtime` | Runtime services: classify + daemon | `P{N}-svc-runtime-*` | 13 | ✅ Realigned |
| `hkask-services-context` | Service context and contract monitoring | `P{N}-svc-context-*` | 31 | ✅ Realigned |
| `hkask-services-corpus` | Content corpus: discovery + embed | `P{N}-svc-corpus-*` | 30 | ✅ Realigned |
| `hkask-inference` | Inference orchestration | `P{N}-svc-inference-*` | 7 | ✅ Realigned |
| `hkask-services-kanban` | Kanban task board coordination | `KAN-SVC-*` (legacy) | 34 | ⚠️ Migration pending |
| `hkask-services-kata` | Toyota Kata engine | `P{N}-svc-kata-*` | 27 | ✅ Realigned |

---

## 2. Five Categories

| # | Category | Completeness Predicate | Min Artifacts | Cross-References |
|---|----------|----------------------|---------------|-----------------|
| 1 | **Domain** | Every entity has a named term and a bounded-context map | Domain ontology sketch | → Composition (verbs), → Lifecycle (persistence) |
| 2 | **Composition** | Every domain verb has a granted composition, registered interface, and composable path | Capability grant table, interface equivalence matrix, registry schema | → Domain (ontology), → Trust (tokens) |
| 3 | **Trust** | Every capability operation has a threat-model entry and an OCAP-bound mitigation | Threat model, keystore config, capability attenuation policy | → Composition (capabilities), → Lifecycle (audit) |
| 4 | **Lifecycle** | Bootstrap, evolution, deprecation, lifecycle, and persistence are expressible as spec transitions | Bootstrap manifest, evolution rules, deprecation policy, CNS span registry | → Domain (entities), → Trust (audit) |
| 5 | **Curation** | Every spec artifact has been evaluated for coherence by a curator with documented rationale | Curation decision log, coherence score | → Domain (grounding), → Lifecycle (health) |

[^evans-ddd]: Evans, Eric. *Domain-Driven Design: Tackling Complexity in the Heart of Software.* Addison-Wesley, 2003. — Bounded contexts, ubiquitous language, and the domain model that MDS categories extend.

---

## 3. Completeness Predicate

```
complete?(G, category) :=
  ∀ goal ∈ G[category]:
    ∃ criterion ∈ goal.criteria:
      criterion.satisfied = true
  ∧ ∀ cross_ref ∈ G[category].cross_references:
    complete?(G, cross_ref.target_category)

curated?(G) :=
  coherence_score(G.artifacts) ≥ threshold
  ∧ ∀ artifact ∈ G.artifacts:
    curation_decision ∈ {Accept, Revise, Reject}
    ∧ decision.rationale documented
```

A goal-set G is **MDS-complete** iff `complete?(G, c)` holds for all 5 categories **and** `curated?(G)` holds.

Curation decisions (Accept/Revise/Reject) are made by the Curator or human — not by any automated tool. The QA system validates coherence; the Curator makes decisions.

[^hoare-triple]: Hoare, C.A.R. "An Axiomatic Basis for Computer Programming." *Communications of the ACM*, 1969. — The {P} C {Q} Hoare triple that inspires MDS's completeness predicate: precondition → command → postcondition.

---

## 4. Spec Operations & QA Integration

Specifications are managed through three surfaces, following the MCP≡CLI≡API equivalence axiom:

### 4.1 CLI Surface (`kask spec`)

Thin passthrough to `SpecStore` in `hkask-storage`. No intermediate service layer — the CLI builds `Spec` domain objects and persists them directly.

| Command | Operation | Delegate |
|---------|-----------|----------|
| `kask spec capture` | Create a spec with name, category, domain, criteria | `SpecStore::save()` |
| `kask spec list` | List specs, optionally filtered by MDS category | `SpecStore::list_all()` / `list_by_category()` |
| `kask spec validate` | Evaluate a single spec via `DefaultSpecCurator::evaluate()` | Curator agent |
| `kask spec cultivate` | Validate + display per-category coherence requirements | Curator agent |
| `kask spec render` | Render a spec through a Jinja2 template | `minijinja` + `SpecStore::load()` |

### 4.2 API Surface

REST endpoints in `hkask-api` read and write specs directly through `AgentService::spec_store()`. Same `SpecStore` backend, same domain types, no service-layer intermediary.

| Endpoint | Operation |
|----------|----------|
| `GET /api/specs` | List specs with optional category filter |
| `GET /api/specs/{id}` | Get spec detail with requirements |
| `POST /api/specs/capture` | Capture a spec from description + context |
| `GET /api/specs/coherence` | Category coverage ratio across all specs |
| `GET /api/specs/{id}/writing-quality` | Structural quality check (name, criteria, completeness) |

### 4.3 QA Integration (`kask qa spec-check`)

Spec validation, coherence checking, and quality assessment moved from the former MCP spec server into the QA system. The QA system's classifier + self-heal pipeline naturally extends to spec health.

| Command | Operation |
|---------|----------|
| `kask qa spec-check` | Full collection check: category coverage + per-spec quality |
| `kask qa spec-check --spec-id <uuid>` | Single-spec validation via `DefaultSpecCurator::evaluate()` |

### 4.4 Replica Integration (`replica_rewrite`)

The Gentle-Lovelace prose rewriting capability moved to `hkask-mcp-replica` as the `replica_rewrite` tool. It takes a passage/code snippet + quality dimension (gentle/schriver/hopper/lovelace/composite) and delegates to `ComposeService::compose()` with dimension-specific prompts.

| Tool | Server | Description |
|------|--------|-------------|
| `replica_rewrite` | `hkask-mcp-replica` | Rewrite prose optimized for a Gentle Lovelace quality dimension |
| `replica_compose` | `hkask-mcp-replica` | Generate prose in any author's style (underlying engine) |
| `replica_compare` | `hkask-mcp-replica` | Evaluate document against persona centroids (per-dimension scoring) |

### 4.5 The Spec Store

The canonical persistence surface is `hkask_storage::SpecStore` (implemented by `SqliteSpecStore`). All spec operations — CLI, API, and QA — read and write through this single interface. Domain types (`Spec`, `GoalSpec`, `SpecCategory`, `SpecId`) live in `hkask-storage::spec_types`.

```
CLI ──→ SpecStore ──→ SQLite
API ──→ SpecStore ──→ SQLite
QA  ──→ SpecStore ──→ SQLite  (spec-check)
     ──→ DefaultSpecCurator  (validation)
```

---

### 4.6 Replica Server Tools

The replica server provides 9 tools for style corpus management, prose generation, and author comparison:

| Server | Tools | Domain | Status |
|--------|-------|--------|--------|
| `hkask-mcp-replica` | `replica_build`, `replica_compose`, `replica_rewrite`, `replica_mashup`, `replica_compare`, `replica_registry`, `replica_explain`, `replica_discover`, `replica_cache_work` | Style replication + prose rewriting | ✅ Implemented |

### 4.7 Replicant Architecture

The replica system models a **human exemplar** — a named individual whose body of work constitutes a representational corpus. The logical validity of the replica derives from the relationship between the human and their work: the corpus *is* the evidence of their voice, style, and intellectual framework. Each passage is a sample of that relationship.

**Corpus sources by exemplar type:**

| Exemplar type | Discovery | Source examples | Status |
|--------------|-----------|----------------|--------|
| Public domain author | Static YAML (`works:` list pointing to Gutenberg URLs) | Hemingway, Woolf, Austen, Wilde, Twain, Grant, Christie, Eliot | ✅ Implemented |
| Mashup persona | Two-author centroid interpolation; exemplars drawn from both source corpora | Jane Wilde (Austen×Wilde), Ulysses S. Twain (Grant×Twain), Agatha Eliot (Christie×Eliot) | ✅ Implemented |
| Academic author | Dynamic corpus discovery via research MCP tools; disambiguation required | "David Dunning" → "David Dunning, University of Michigan" | 🔮 Planned |

### Academic Author Pipeline (Planned)

For academic exemplars, the corpus is not statically declared — it is discovered dynamically through the existing research infrastructure. The research MCP server (`hkask-mcp-research`) provides tools that can discover, extract, and cache academic content without replicating infrastructure:

| Research tool | Role in corpus discovery |
|--------------|--------------------------|
| `web_search` | Find the author's papers, talks, interviews, and profiles across the open web |
| `web_extract` | Download full-text content from discovered URLs (papers, transcripts, blog posts) |
| `web_find_similar` | Expand the corpus by finding related work and responses to the author |
| `web_browse` | Navigate academic profiles (Google Scholar, Semantic Scholar, arXiv author pages) to enumerate works |

The planned `replica_discover` tool would orchestrate this pipeline:

1. **Name disambiguation**: Given a name (e.g., "David Dunning"), search academic and open sources, present candidate matches to the Curator for confirmation. This is a consent boundary — the Curator selects *which* David Dunning.
2. **Work enumeration**: From the confirmed identity, enumerate their known works across sources (arXiv, Semantic Scholar, open web, institutional pages, conference proceedings, transcripts).
3. **Content acquisition**: Download and cache each work via `web_extract`, producing `.cache/{slug}.txt` files mirroring the public-domain author pattern.
4. **Corpus config generation**: Produce a `corpus.yaml` with the discovered works, ready for `replica_build`.
5. **Embedding and replication**: Standard pipeline from this point forward — chunk, tag, embed, store triples, compute centroid.


---

## 5. Capability-Driven Model

MDS is capability-driven, not constraint-driven:

| Aspect | Constraint-Driven | MDS (Capability-Driven) |
|--------|-------------------|-------------------------|
| Spec as | Fence ("MUST NOT") | Grant ("CAN verb on resource via interface") |
| Validation | Static checks, lints | Composability test, POLA audit |
| Growth | Add constraints | Compose capabilities |
| Lifecycle | Governed (gates) | Curated (invitations) |
| Failure mode | Over-constrained | Under-governed |
| hKask alignment | — | OCAP, capability tokens, attenuation |

[^ocap]: Miller, M. (2006). *Robust Composition: Towards a National Research Agenda for Object Capability Security.* HP Labs. — Object capability model: access is granted by possession of a capability token.

---

## 6. MDS Cycle

```
MDS_cycle(S, D) :=
  let spec = capture(D)            // Build Spec from domain description
  store.save(spec)                 // Persist via SpecStore
  curate(spec)                     // Validate via DefaultSpecCurator
  qa spec-check                    // Category coverage + quality gate
  human_or_curator decides:        // External governance
    Accept | Revise | Reject
```

Spec capture and listing go through `SpecStore` directly. Validation and curation delegate to `DefaultSpecCurator`. Collection-wide health checks run through `kask qa spec-check`. Curation decisions remain external.

[^beck-tdd]: Beck, Kent. *Test-Driven Development: By Example.* Addison-Wesley, 2003. — The red-green-refactor cycle that MDS's capture→decompose→validate→curate cycle parallels.

---

## 7. Template Manifests

Each category has a minimal YAML template. All use `schema_version: "0.30.0"`.

### 7.1 Domain Spec Template

```yaml
schema_version: "0.30.0"
category: domain
domain_anchor: hkask
bounded_context: "..."

ontology:
  entities:
    - name: Agent
      attributes: [webid, capabilities, persona]

focusing_assumptions:
  - id: FA-D1
    statement: "..."
    rationale: "..."

completeness_checklist:
  - "Every entity has a named term"
  - "Bounded-context map exists"

cross_references:
  - category: composition
    relation: "Entities expose composable verbs"
  - category: lifecycle
    relation: "Entity state persisted across lifecycle"
```

### 7.2 Composition Spec Template

```yaml
schema_version: "0.30.0"
category: composition
domain_anchor: hkask

verb_inventory:
  - verb: invoke_tool
    resource: McpServer
    interface: [mcp, cli, api]
  - verb: render_template
    resource: Template
    interface: [mcp, cli, api]

interface_equivalence:
  mcp: true
  cli: true
  api: true
  equivalent: true  # All three exercise same functional core

registry:
  type: unified
  discriminator: template_type
  cascade_depth_max: 7

ocap_policy:
  attenuation_max: 7
  token_ttl_seconds: 3600
```

### 7.3 Trust Spec Template

```yaml
schema_version: "0.30.0"
category: trust
domain_anchor: hkask

threat_model:
  adversaries:
    - name: malicious_template_author
      vector: template_injection
      mitigation: jinja2_sandbox + capability_gating
    - name: compromised_dependency
      vector: supply_chain
      mitigation: cargo_deny + pinned_versions

ocap_boundaries:
  - "Every resource access passes through require_capability + require_sovereignty"
  - "Tokens are unforgeable, attenuating, no admin override"

keystore:
  encryption: AES-256-GCM
  key_derivation: Argon2id + HKDF-SHA256
  storage: OS_keychain + SQLCipher
```

### 7.4 Lifecycle Spec Template

```yaml
schema_version: "0.30.0"
category: lifecycle
domain_anchor: hkask

bootstrap:
  sequence: [resolve_secrets, open_databases, build_service_context, start_loops]

evolution:
  versioning: git_sha_only
  migration: "Schema migrations run on version bump"

deprecation:
  policy: "Prefer deletion over deprecation (P5)"

observability:
  cns_spans:
    - namespace: cns.tool
      covers: "Tool invocation governance"
    - namespace: cns.inference
      covers: "Inference budget tracking"
  variety_counters:
    - counter: tool_diversity
      threshold: 50
    - counter: template_diversity
      threshold: 30
  algedonic:
    trigger: "variety_deficit > threshold"
    escalation: "Curator → Human"

persistence:
  engine: SQLite + SQLCipher
  schema: bitemporal_triples
  vector_store: sqlite-vec
  memory_pipelines:
    - name: episodic
      visibility: private
    - name: semantic
      visibility: public
```

### 7.5 Curation Spec Template

```yaml
schema_version: "0.30.0"
category: curation
domain_anchor: hkask

curation_model:
  decisions: [Accept, Revise, Reject]
  curator:
    type: Daemon
    authority: "Human-augmented — curator proposes, human decides"
  guidance: |
    Accept — spec is coherent and complete, publish it.
    Revise — spec needs work, return with rationale.
    Reject — spec is not useful, remove it.

coherence_metric:
  method: "Jaccard similarity of declared vs. registered verbs"
  threshold: 0.7
```

[^fowler-poeaa]: Fowler, M. (2002). *Patterns of Enterprise Application Architecture.* Addison-Wesley. — Template pattern: a standard structure that captures domain knowledge in a reusable form.

---

## 8. Testing Protocol

### Principles

1. **Contract-anchored:** Every test verifies a behavioral contract via `expect:` + `[P{N}]` annotations.
2. **Public seam only:** Tests verify behavior through public interfaces, not implementation.
3. **Tracer bullet:** One RED→GREEN cycle per behavior. No horizontal slicing.
4. **Category coverage:** Each MDS category has at least one integration test.

### Category → Test Strategy

| Category | Test Strategy |
|----------|--------------|
| Domain | Entity definition + term validation |
| Composition | Capability composition + interface equivalence verification |
| Trust | OCAP boundary enforcement + threat model audit |
| Lifecycle | Bootstrap + evolution + deprecation + CNS span emission |
| Curation | Coherence scoring + decision rationale documentation |

[^principles-p8]: hKask Team. (2026). *Architecture Principles — P8.* `docs/architecture/core/PRINCIPLES.md` (P8) — Every `#[test]` verifies a stated behavioral property of a public seam.

---

## 9. Documentation Structure

> **Incorporated from:** `docs/specifications/specs/MDS_SCAFFOLD.md`

### 9.1 Category → Directory Mapping

Where each MDS category's authoritative documents live:

| # | MDS Category | Primary Directory | Key Documents |
|---|--------------|-------------------|---------------|
| 1 | **Domain** | `architecture/` | MDS.md, FUNCTIONAL_SPECIFICATION.md |
| 2 | **Composition** | `architecture/` | MDS.md, loop-architecture.md |
| 3 | **Trust** | `architecture/` | magna-carta.md, PRINCIPLES.md |
| 4 | **Lifecycle** | `architecture/` + `plans/` | MDS.md, deployment-and-backup.md |
| 5 | **Curation** | `architecture/` + `specifications/` | WRITING_EXCELLENCE.md, DOCUMENTATION_STANDARDS.md |

**Rule:** New documents go in the directory of their primary MDS category. Cross-cutting documents go in the directory of their dominant category.

### 9.2 Document Lifecycle

```
Draft → Active → Deprecated → Superseded → Removed
```

| State | Rule |
|-------|------|
| **Active** | Must map to ≥1 MDS category via `mds_categories` frontmatter |
| **Deprecated** | Move to `docs/archive/YYYY-MM-DD-<label>/` |
| **Superseded** | Move to archive; successor must reference it |
| **Removed** | `git rm` from working tree; git history is archive of record |

### 9.3 Verification

```bash
bash docs/ci/check-links.sh    # Zero broken cross-references
```

---

## 10. References

[^w3c-rdf]: W3C. (2014). *RDF 1.1 Concepts and Abstract Syntax*. <https://www.w3.org/TR/rdf11-concepts/>.
[^miller-robust]: Miller, M. S. (2006). *Robust Composition: Towards a Unified Approach to Access Control and Concurrency Control*. Johns Hopkins University.
[^cockburn-hexagonal]: Cockburn, A. (2005). *Hexagonal Architecture*. <https://alistair.cockburn.us/hexagonal-architecture/>.
[^shostack-threat]: Shostack, A. (2014). *Threat Modeling: Designing for Security*. Wiley.
[^ronacher-jinja2]: Ronacher, A. (2026). *Jinja2 Template Designer Reference*. <https://jinja.palletsprojects.com/>.

---

*MDS v0.33.0 — five categories, SpecStore + QA. Five thin service passthroughs removed (contacts, scheduler, goal, pod, spec).*

---

## AgentService Specification

> **Incorporated from:** docs/specifications/specs/MDS-agent-service.md

**Purpose:** Specification for the condensed service layer architecture. `AgentService` is the canonical service layer that owns all shared infrastructure. Both CLI and API surfaces compose an `AgentService` instance and add only presentation-specific state.

### Bounded Context

`AgentService` is the **single source of truth** for all shared infrastructure in hKask. **Boundary:** In-process only. MCP servers do NOT depend on `AgentService` (P1 Prohibition — out-of-process isolation).

All 28 fields are **private** and exposed through **individual named accessor methods** — one method per field or small domain-coherent pair. This replaces the earlier 8-group-method tuple pattern.

| Method | Returns | Category |
|--------|---------|----------|
| `config()` | `&ServiceConfig` | Configuration |
| `wallet()` | `Option<&Arc<WalletService>>` | Payments |
| `wallet_store()` | `Option<&Arc<WalletStore>>` | Payments |
| `memory()` | `(&Arc<dyn EpisodicStoragePort>, &Arc<dyn SemanticStoragePort>)` | Memory |
| `registry()` | `&Arc<tokio::sync::Mutex<SqliteRegistry>>` | Storage |
| `goal_repo()` | `&Arc<SqliteGoalRepository>` | Storage |
| `cns_runtime()` | `&Arc<RwLock<CnsRuntime>>` | CNS |
| `cybernetics_loop()` | `&Arc<RwLock<CyberneticsLoop>>` | CNS |
| `loop_system()` | `&Arc<LoopSystem>` | CNS |
| `event_sink()` | `&Arc<dyn NuEventSink>` | CNS |
| `seam_watcher()` | `&Arc<RwLock<Option<SeamWatcher>>>` | CNS |
| `capability_checker()` | `&Arc<CapabilityChecker>` | Governance |
| `mcp_dispatcher()` | `&Arc<McpDispatcher>` | Governance |
| `escalation_queue()` | `&Arc<EscalationQueue>` | Governance |
| `inference_port()` | `Option<Arc<dyn InferencePort>>` | Coordination |
| `mcp_runtime()` | `&Arc<McpRuntime>` | Coordination |
| `active_pods()` | `&Arc<ActivePods>` | Coordination |
| `identity()` | `(&WebID, &Arc<hkask_agents::A2ARuntime>)` | Identity |
| `sovereignty()` | `SovereigntyService` | Sovereignty |
| `curation_inbox_tx()` | `&Option<mpsc::UnboundedSender<CurationInput>>` | Internal |
| `sovereignty_boundary_store()` | `&SovereigntyBoundaryStore` | Sovereignty |
| `spec_store()` | `&SqliteSpecStore` | Surface-specific |
| `agent_registry_store()` | `&hkask_storage::AgentRegistryStore` | Surface-specific |
| `user_store()` | `&Arc<std::sync::Mutex<UserStore>>` | Surface-specific |
| `daemon_handler()` | `&Arc<ServiceDaemonHandler>` | Daemon |
| `matrix_transport()` | `Option<&Arc<tokio::sync::Mutex<MatrixTransport>>>` | Communication |

**Design rationale:** Individual accessors replaced the 8-group-method tuple pattern because callers typically need one field, not an entire domain group, and individual methods are self-documenting.

### Service Layer Contracts

| Subcrate | Domain | Contract Prefix | Count | Status |
|----------|--------|----------------|-------|--------|
| `hkask-services` | Archival, bundle, chat, CNS, compose, consolidation, curator, experience, skills | `P{N}-svc-{domain}-*` | 102 | Realigned |
| `hkask-services-runtime` | Runtime services: classify + daemon | `P{N}-svc-runtime-*` | 13 | Realigned |
| `hkask-services-context` | Service context and contract monitoring | `P{N}-svc-context-*` | 31 | Realigned |
| `hkask-services-corpus` | Content corpus: discovery + embed | `P{N}-svc-corpus-*` | 30 | Realigned |
| `hkask-inference` | Inference orchestration | `P{N}-svc-inference-*` | 7 | Realigned |
| `hkask-services-kanban` | Kanban task board coordination | `KAN-SVC-*` (legacy) | 34 | Migration pending |
| `hkask-services-kata` | Toyota Kata engine | `P{N}-svc-kata-*` | 27 | Realigned |

### Crate-to-Domain Mappings

| Crate | MDS Category | Key Entities |
|-------|-------------|-------------|
| `hkask-services` | Domain, Composition | `AgentService`, `ServiceConfig`, `PerAgentMemory` |
| `hkask-services-runtime` | Lifecycle | `ClassifyService`, `DaemonService` |
| `hkask-services-context` | Lifecycle | `ContextService`, contract monitoring |
| `hkask-services-corpus` | Domain | `CorpusService`, embedding pipelines |
| `hkask-inference` | Composition | Inference orchestration, provider routing |
| `hkask-services-kanban` | Domain | `Board`, `Column`, `Task`, Kanban coordination |
| `hkask-services-kata` | Domain, Curation | `KataEngine`, `KataManifest`, coaching loops |

### Dependency Direction

```mermaid
graph TD
    CLI["hkask-cli"]
    API["hkask-api"]
    SVC["hkask-services (AgentService)"]
    CLI --> SVC
    API --> SVC
    SVC --> AGENTS[hkask-agents]
    SVC --> CNS[hkask-cns]
    SVC --> MEM[hkask-memory]
    SVC --> TEMPLATES[hkask-templates]
    SVC --> TYPES[hkask-types]
    SVC --> STORAGE[hkask-storage]
```

Domain crates **never** depend on `hkask-services`. MCP servers **never** depend on `hkask-services` for orchestration (P1 Prohibition).

### OCAP Boundaries

| Boundary | Enforcement | Principle |
|----------|-------------|-----------|
| Tool invocation | `CapabilityChecker` gating via `governed_tool` | P4 |
| Inference calls | `governed_inference` membrane with energy budget checks | P4 |
| MCP server isolation | Out-of-process; no `AgentService` dependency | P1 |
| Capability attenuation | Max depth limit, TTL expiry on tokens | P4 |

### Bootstrap Sequence

1. `AgentService::build(config)` assembles all shared infrastructure
2. Per-agent memory created via `build_per_agent_memory(db)`
3. Consolidation is routed through `AgentService::consolidate_agent_memory(agent_name, request)` — the single OCAP-gated, consent-checked entry point
4. CLI surface wraps with `ReplState` (= `AgentService` + REPL fields)
5. API surface wraps with `ApiState` (= `Arc<AgentService>` + HTTP fields)

### Interface Equivalence

Both CLI and API surfaces use identical `AgentService` accessors and the same `consolidate_agent_memory` entry point. The only surface-specific methods are `daemon_handler()` (CLI daemon mode only) and `matrix_transport()` (REPL only). All remaining 26 accessors are equivalent across surfaces.
