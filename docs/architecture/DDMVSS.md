---
title: "DDMVSS — Domain-Driven Minimum Viable Specification Set"
audience: [architects, developers, agents]
last_updated: 2026-06-06
version: "0.2.2"
status: "Active"
domain: "Cross-cutting"
ddmvss_categories: [domain, capability, interface, composition, trust, observability, persistence, lifecycle, curation]
---

# DDMVSS — Domain-Driven Minimum Viable Specification Set

**Purpose:** The smallest set of specifications that fully defines an MVP for a domain-anchored system, plus the methodology (MVSDD) that produces it.

**Axiom:** `Specification ≡ ⟨Goals, Plan⟩` — one vocabulary, two registers.
**Axiom:** `Goal ≡ Requirement` — bidirectional equivalence.
**Focusing simplification:** `MCP ≡ CLI ≡ API` — three surfaces, one functional core.
**Design principle:** *Specifications are invitations to curate, not gates to govern.* Each spec defines a capability surface, not a constraint boundary.

---

## Contents

| Section | Description |
|---------|-------------|
| [§1 Semantic Map & Root-Cause Drilldown](#1-semantic-map--root-cause-drilldown-task-1) | RDF/Turtle graph and root-cause analysis of the DDMVSS domain |
| [§2 Comparative Review](#2-comparative-review-task-2) | Comparison with existing specification methodologies |
| [§3 DDMVSS Categories — Goal-Group Taxonomy](#3-ddmvss-categories--goal-group-taxonomy-task-3) | Goal-group taxonomy for DDMVSS classification |
| [§4 MVSDD as Capability Model](#4-mvsdd-as-capability-model-task-4) | Minimum Viable Specification-Driven Design as OCAP capability |
| [§5 Template Manifests](#5-template-manifests-task-5) | Manifest specifications for each template type |
| [§6 Hexagonal Architecture Mapping](#6-hexagonal-architecture-mapping-task-6) | Ports and adapters mapping to hexagonal architecture |
| [§7 Capability & Security Design](#7-capability--security-design-task-7) | OCAP capability attenuation and security model |
| [§8 Rust Type-Level Skeleton](#8-rust-type-level-skeleton-task-8) | Type-level implementation skeleton in Rust |
| [§9 Self-Application Validation](#9-self-application-validation-task-9) | DDMVSS applied to itself as validation |
| [§10 Open / Underspecified Questions](#10-open--underspecified-questions-task-10) | Identified gaps and underspecified areas |
| [§11 Adversarial Review Remediation](#11-adversarial-review-remediation-2026-05-24) | Remediation of adversarial review findings |
| [§11 References](#11-references) | Citations and references |

---

## 1. Semantic Map & Root-Cause Drilldown (Task 1)

### 1.1 RDF/Turtle Graph [^w3c-rdf]

```turtle
@prefix : <https://hkask.dev/ontology/ddmvss#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

:Specification  a :CompositeEntity ;
    :hasGoal    :Goal ;
    :hasPlan    :Plan ;
    :servedBy   :Curation .

:Goal           a :SemanticUnit ;
    :realizes       :Domain ;
    :satisfies      :CompletenessPredicate ;
    :composesInto   :Specification ;
    :decomposesInto :Goal .

:Plan           a :SyntacticUnit ;
    :enables        :Capability ;
    :constrains     :Interface .

:Domain         a :BoundedContext ;
    :belongsToDomain :MVP .

:Capability     a :GrantedAuthority ;
    :enables        :Interface .

:Constraint     a :BoundaryCondition ;
    :constrains     :Capability ;
    :constrains     :Interface .

:Interface      a :Surface ;
    :simplifiesVia  :FocusingAssumption .

:MVP            a :MinimalSystem ;
    :satisfies      :CompletenessPredicate .

:MVSS           a :Artifact ;
    :composesInto   :Specification ;
    :satisfies      :CompletenessPredicate .

:MVSDD          a :Methodology ;
    :hasGoal        :Goal ;
    :hasPlan        :Plan ;
    :enables        :MVSS .

:DDMVSS         a :SortingMechanism ;
    :belongsToDomain :Domain ;
    :simplifiesVia  :FocusingAssumption ;
    :enables        :MVSS .

:CompletenessPredicate  a :VerificationFunction ;
    :satisfies      :Goal .

:FocusingAssumption     a :Simplification ;
    :constrains     :Interface ;
    :simplifiesVia  :Specification .

:Curation       a :LifecycleProcess ;
    :curates        :Artifact ;
    :evaluates      :CompletenessPredicate ;
    :reconciles     :Goal ;
    :cultivates     :Collection .

:Artifact       a :SpecOutput ;
    :instantiates   :Specification ;
    :grounds        :hLexicon .

:Collection     a :CuratedSet ;
    :composesInto   :MVSS .

:hLexicon       a :Vocabulary ;
    :grounds        :Artifact .
```

### 1.2 Mermaid ER Diagram

```mermaid
erDiagram
    SPECIFICATION ||--|{ GOAL : hasGoal
    SPECIFICATION ||--|{ PLAN : hasPlan
    SPECIFICATION ||--o| CURATION : servedBy
    GOAL ||--o| DOMAIN : realizes
    GOAL ||--o| COMPLETENESS_PREDICATE : satisfies
    GOAL ||--o{ GOAL : "composesInto / decomposesInto"
    PLAN ||--o{ CAPABILITY : enables
    PLAN ||--o{ INTERFACE : constrains
    DOMAIN ||--o| MVP : belongsToDomain
    CAPABILITY ||--o{ INTERFACE : enables
    CONSTRAINT ||--o{ CAPABILITY : constrains
    CONSTRAINT ||--o{ INTERFACE : constrains
    INTERFACE ||--o| FOCUSING_ASSUMPTION : simplifiesVia
    MVSS ||--|{ SPECIFICATION : composesInto
    MVSS ||--o| COMPLETENESS_PREDICATE : satisfies
    MVSDD ||--o| MVSS : enables
    DDMVSS ||--o| DOMAIN : belongsToDomain
    DDMVSS ||--o| MVSS : enables
    DDMVSS ||--o| FOCUSING_ASSUMPTION : simplifiesVia
    CURATION ||--o{ ARTIFACT : curates
    CURATION ||--o| COMPLETENESS_PREDICATE : evaluates
    CURATION ||--o{ GOAL : reconciles
    CURATION ||--o| COLLECTION : cultivates
    ARTIFACT ||--o| SPECIFICATION : instantiates
    ARTIFACT ||--o| HLEXICON : grounds
    COLLECTION ||--o{ ARTIFACT : composesInto

    SPECIFICATION {
        uuid id PK
        string name
        string version
    }
    GOAL {
        uuid id PK
        string text
        string state
        string category
    }
    PLAN {
        uuid id PK
        string steps
        string template_ref
    }
    DOMAIN {
        string name PK
        string anchor_system
        string bounded_context
    }
    CAPABILITY {
        uuid id PK
        string verb
        string resource
        string interface
    }
    CONSTRAINT {
        uuid id PK
        string type
        string expression
    }
    INTERFACE {
        string surface PK
        bool mcp
        bool cli
        bool api
        bool equivalent
    }
    COMPLETENESS_PREDICATE {
        string category PK
        string expression
        bool satisfied
    }
    FOCUSING_ASSUMPTION {
        string id PK
        string statement
        string rationale
    }
    CURATION {
        uuid id PK
        string decision "Merge|Revise|Defer|Discard"
        string rationale
    }
    ARTIFACT {
        uuid id PK
        string type
        string content_ref
    }
    COLLECTION {
        uuid id PK
        string coherence_score
    }
    HLEXICON {
        string term PK
        string domain
        string definition
    }
    MVSS {
        uuid id PK
        string domain
        string version
    }
    MVSDD {
        string phase PK
        string recursion_depth
    }
    DDMVSS {
        string domain PK
        string sorting_key
    }
    ```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-DDMVSS-001
verified_date: 2026-05-24
verified_against: crates/hkask-types/src/spec.rs; crates/hkask-templates/src/manifest.rs
status: VERIFIED
-->

### 1.3 Predicate Root-Cause Drilldown

| # | Predicate | Root Driver | Failure Mode if Absent |
|---|-----------|-------------|----------------------|
| 1 | `hasGoal` | A specification without goals is a plan without purpose — no *why* | Spec degenerates into a task list; no way to validate sufficiency |
| 2 | `hasPlan` | Goals without plans are wishes — no *how* | Spec is aspirational, not executable; no implementation guidance |
| 3 | `realizes` | Goals must anchor to a domain or they float free | Goals become generic; no bounded context to test completeness against |
| 4 | `constrains` | Unconstrained capabilities are ambient authority (POLA violation) | Security model collapses; any agent can do anything |
| 5 | `enables` | Capabilities must surface through interfaces or they're latent | System has powers no one can exercise; dead functionality |
| 6 | `belongsToDomain` | Domain membership prevents scope creep across bounded contexts | Specification bloat; one spec tries to cover Okapi + Russell + hKask |
| 7 | `satisfies` | Without a completeness predicate, "done" is undefined | MVP never ships; infinite refinement loop |
| 8 | `composesInto` | Atomic specs must compose or the system can't grow incrementally | Each new feature requires full re-specification; no incremental development |
| 9 | `simplifiesVia` | Focusing assumptions collapse dimensions (e.g., MCP≡CLI≡API) | Specification surface triples; redundant docs for each interface |
| 10 | `curates` | Specs are living collections requiring evaluation, reconciliation, and cultivation | Specs become stale artifacts; no feedback loop between specification and operation; curation debt accumulates invisibly |
| 11 | `grounds` | Artifacts must reference hLexicon terms or vocabulary drifts | Templates use inconsistent terminology; LLM interpretation becomes unpredictable |

---

## 2. Comparative Review (Task 2)

| Framework | Reusable Primitive | Mapped to DDMVSS | Discarded Baggage |
|-----------|-------------------|-------------------|-------------------|
| **TOGAF ADM** | Phase-gated architecture development | MVSDD recursion cycle (specify→grant→compose→curate→reflect) | Content metamodel, Architecture Repository, full 8-phase ADM |
| **Agile-TOGAF / MVA** | "Just-enough architecture" threshold | CompletenessPredicate per category | Sprint planning, story points, velocity tracking |
| **Lean Startup MVP** | Build-Measure-Learn feedback loop [^ries-lean] | MVSDD reflect→respecify step | Pivot/persevere decisions, innovation accounting |
| **Cynefin** | Domain framing (clear/complicated/complex/chaotic) [^snowden-cynefin] | Domain anchor selection (Okapi/Russell/hKask) | Full sense-making framework, probe-respond patterns |
| **DDD (Evans)** | Bounded Context, Ubiquitous Language | Domain spec category, hLexicon grounding | Aggregates, repositories, domain events (hKask uses ν-events) |
| **Capability-based design (Miller)** | OCAP, POLA, unforgeable capability tokens [^miller-robust] | Capability spec category, hkask-keystore integration | Full E-language, vat model, distributed promises |
| **Job Stories** | `When <situation>, I want to <motivation>, so I can <outcome>` | Goal text format in templates | Persona creation, job mapping canvas |
| **arc42** | 12-section documentation template | Reduced to 9 DDMVSS categories | Full 12-section structure, cross-cutting concepts section |
| **C4** | Hierarchical system decomposition (Context→Container→Component→Code) [^brown-c4] | Spec composition hierarchy | Full C4 diagramming notation |
| **ATAM** | Quality attribute scenarios | CompletenessPredicate expressions | Utility trees, sensitivity/threshold analysis |
| **Participatory Archives / Curation Studies** | Gradient evaluation (Merge/Revise/Defer/Discard), contextualisation, reconciliation | Curation spec category, `CurationPort` trait, spec-curator bot | Full archival science apparatus, provenance chains |

**Key insight:** DDMVSS absorbs *one primitive per framework* and discards the process scaffolding. The sorting mechanism (domain-focus) replaces TOGAF's phase gates and Agile's sprint boundaries. Curation replaces governance — specs are cultivated, not enforced.

---

## 3. DDMVSS Categories — Goal-Group Taxonomy (Task 3)

### 3.1 Category Definitions

| # | Category | Completeness Predicate | Min Artifacts (≤3) | Cross-References |
|---|----------|----------------------|---------------------|-----------------|
| 1 | **Domain** | Every ν-event type in the domain has a named term in hLexicon | hLexicon allocation table, domain ontology sketch, bounded-context map | → Capability (verbs), → Persistence (triples) |
| 2 | **Capability** | Every domain verb has a granted capability with attenuatable token | Capability grant table, OCAP policy doc, verb inventory | → Domain (ontology), → Trust (tokens), → Interface (surface) |
| 3 | **Interface** | MCP, CLI, and API all exercise the same capability set via one functional core | Interface equivalence matrix, tool manifest, route table | → Capability (verbs), → Composition (registry) |
| 4 | **Composition** | Any two capabilities can be composed via registry without code change | Registry schema, cascade rules, template manifest | → Capability (atoms), → Domain (ontology) |
| 5 | **Trust & Security** | Every capability operation has a threat-model entry and a mitigation | Threat model (STRIDE-lite), keystore config, capability attenuation policy | → Capability (tokens), → Observability (audit spans) |
| 6 | **Observability** | Every capability invocation emits a `cns.*` span with variety counter | CNS span registry, algedonic threshold config, variety counter schema | → Trust (audit), → Lifecycle (health) |
| 7 | **Persistence** | Every domain entity has a storage schema with bitemporal triples | SQL schema, embedding vector config, memory pipeline spec | → Domain (entities), → Observability (ν-events) |
| 8 | **Lifecycle** | Bootstrap, evolution, and deprecation are all expressible as spec transitions | Bootstrap manifest, evolution rules, deprecation policy | → Composition (registry), → Observability (health) |
| 9 | **Curation** | Every spec artifact has been evaluated (Merge/Revise/Defer/Discard) by a curator with documented rationale | Curation decision log, coherence score, reconciliation record | → Domain (hLexicon grounding), → Observability (cns.spec.curate spans) |

### 3.2 Completeness Predicate Formal Definition

```
complete?(G, category) :=
  ∀ goal ∈ G[category]:
    ∃ criterion ∈ goal.criteria:
      criterion.satisfied = true
  ∧ ∀ cross_ref ∈ G[category].cross_references:
    complete?(G, cross_ref.target_category)

curated?(G) :=
  ∀ artifact ∈ G.artifacts:
    ∃ decision ∈ {Merge, Revise, Defer, Discard}:
      artifact.curation_decision = decision
      ∧ decision.rationale ≠ ∅
  ∧ coherence_score(G.artifacts) ≥ threshold
```

A goal-set G is **MVP-complete** iff `complete?(G, c)` holds for all 9 categories **and** `curated?(G)` holds.

### 3.3 hLexicon Extension — Spec-Curation Terms

Nine new terms distributed across the three domains, following existing `lexicon.rs` bootstrap pattern:

| Term | Domain | Definition |
|------|--------|-----------|
| `specify` | WordAct | Articulate a goal as a binding requirement |
| `elicit` | WordAct | Draw out user intent as structured input |
| `require` | WordAct | Assert a goal as a non-negotiable constraint |
| `decompose` | FlowDef | Break a goal into ordered sub-goals |
| `sequence` | FlowDef | Arrange sub-goals into execution order |
| `constrain` | FlowDef | Attach OCAP boundaries to a goal |
| `curate` | KnowAct | Evaluate an artifact for collection coherence |
| `contextualise` | KnowAct | Situate an artifact within its meaningful environment |
| `reconcile` | KnowAct | Resolve goal tensions without collapsing them |

---

## 4. MVSDD as Capability Model (Task 4)

### 4.1 Constraint-Driven vs. Capability-Driven SDD [^miller-robust]

| Aspect | Constraint-Driven SDD | MVSDD (Capability-Driven) |
|--------|----------------------|---------------------------|
| Spec as | Fence ("MUST NOT") | Grant ("CAN verb on resource via interface") |
| Validation | Static checks, lints | Composability test, POLA audit |
| Growth | Add constraints | Compose capabilities |
| Lifecycle | Governed (gates) | Curated (invitations) |
| Failure mode | Over-constrained (nothing works) | Under-governed (too much works) |
| hKask alignment | P1–P7, C1–C7 (used for pruning) | OCAP, capability tokens, attenuation, curation |

### 4.2 MVSDD Small-Step Recursion

```
MVSDD_cycle(S, D) :=
  let G = specify(D)                    // Extract goals from domain D
  let C = grant_capabilities(G)         // Grant OCAP tokens per goal
  let S' = compose(S, C)               // Compose into existing spec set
  let S'' = curate(S')                 // Evaluate, reconcile, cultivate
  let V = reflect(S'', D)              // Verify against domain via CNS
  if complete?(S'') ∧ curated?(S'') then S''  // Base case: done
  else MVSDD_cycle(S'', refine(D, V))  // Inductive step: respecify
```

**Base case:** A single goal with a single capability, curated (Merge decision with rationale), verified by a single CNS span.
**Inductive step:** Add one goal-category pair per cycle; curate the expanded set; verify composition doesn't break prior capabilities.

### 4.3 MVSDD Cycle Sequence Diagram

```mermaid
sequenceDiagram
    participant H as Human/Curator
    participant S as Spec Engine
    participant R as Registry
    participant C as Capability Store
    participant Cu as Curation Engine
    participant CNS as CNS Observer

    H->>S: specify(domain_anchor)
    S->>S: extract goals → G[]
    S->>C: grant_capabilities(G[])
    C-->>S: CapabilityToken[]
    S->>R: compose(existing_specs, new_tokens)
    R-->>S: ComposedSpec
    S->>Cu: curate(ComposedSpec)
    Cu->>Cu: evaluate → CurationDecision
    Cu->>Cu: reconcile(conflicts)
    Cu-->>S: CuratedSpec + decision_log
    S->>CNS: emit cns.spec.curate span
    CNS-->>S: variety_count, algedonic_check
    alt variety_deficit > 100
        CNS->>H: algedonic alert
        H->>S: refine(domain, feedback)
    else complete?(spec) ∧ curated?(spec)
        S-->>H: MVSS artifact (signed manifest)
    else not complete
        S->>S: MVSDD_cycle(spec, refined_domain)
    end
    ```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-DDMVSS-002
verified_date: 2026-05-24
verified_against: crates/hkask-types/src/spec.rs; crates/hkask-templates/src/manifest.rs
status: VERIFIED
-->

---

## 5. Template Manifests (Task 5)

### 5.1 Domain Spec Template

```yaml
# domain-spec.yaml
schema_version: "0.2.0"
category: domain
domain_anchor: hkask  # okapi | russell | hkask
bounded_context: "Agentic AI tooling"

ontology:
  nu_event_types:
    - term: "tool_invocation"
      domain: WordAct
    - term: "agent_delegation"
      domain: FlowDef
    - term: "memory_consolidation"
      domain: KnowAct
  entities:
    - name: Agent
      attributes: [webid, type, capabilities]
    - name: Template
      attributes: [id, template_type, lexicon_terms]
    - name: Manifest
      attributes: [id, steps, template_ref]

hlexicon_allocation:
  word_act_terms: 25
  flow_def_terms: 25
  know_act_terms: 25

focusing_assumptions:
  - id: FA-D1
    statement: "Domain vocabulary is bounded to 75 hLexicon terms"
    rationale: "Miller's law — 7±2 categories, 3 domains"

completeness_checklist:
  - "Every ν-event type has hLexicon term"
  - "Bounded context map drawn"
  - "No entity without a storage schema"

cross_references:
  - category: capability
    relation: "domain verbs → capability grants"
  - category: persistence
    relation: "domain entities → storage schemas"
  - category: curation
    relation: "domain terms validated during curation"
```

### 5.2 Capability Spec Template

```yaml
# capability-spec.yaml
schema_version: "0.2.0"
category: capability
domain_anchor: hkask

verb_inventory:
  - verb: invoke_tool
    resource: McpServer
    interface: [mcp, cli, api]
  - verb: render_template
    resource: Template
    interface: [mcp, cli, api]
  - verb: compose_manifest
    resource: Manifest
    interface: [mcp, cli, api]
  - verb: delegate_capability
    resource: CapabilityToken
    interface: [mcp]
  - verb: curate_artifact
    resource: SpecArtifact
    interface: [mcp, cli, api]

ocap_policy:
  attenuation_max: 7
  token_ttl_seconds: 3600
  revocation: bloom_filter_deferred

focusing_assumptions:
  - id: FA-C1
    statement: "All capabilities surface through MCP ≡ CLI ≡ API"
    rationale: "Three surfaces, one functional core"

completeness_checklist:
  - "Every domain verb has a capability grant"
  - "Attenuation policy defined"
  - "Token lifecycle documented"

cross_references:
  - category: domain
    relation: "verbs grounded in domain ontology"
  - category: trust
    relation: "tokens governed by threat model"
  - category: interface
    relation: "capabilities surface through interfaces"
  - category: curation
    relation: "curation operations are capabilities"
```

### 5.3 Interface Spec Template

```yaml
# interface-spec.yaml
schema_version: "0.2.0"
category: interface
domain_anchor: hkask

interface:
  mcp: true
  cli: true
  api: true
  equivalent: true  # All three exercise same functional core

surfaces:
  mcp:
    servers: [inference, storage, memory, embedding, condenser, web, ocap, keystore, cns, git, registry, gml, github, spec]
    protocol: rmcp
  cli:
    binary: kask
    framework: clap
    subcommands: [chat, run, template, registry, goal, spec]
  api:
    framework: axum
    docs: utoipa
    auth: capability_token

equivalence_matrix:
  - capability: invoke_tool
    mcp: "tool_call(server, name, args)"
    cli: "kask run <server> <name> --args"
    api: "POST /api/v1/tools/{server}/{name}"
  - capability: render_template
    mcp: "template_render(id, context)"
    cli: "kask template render <id>"
    api: "POST /api/v1/templates/{id}/render"
  - capability: curate_artifact
    mcp: "spec/curate/evaluate(artifact_id, collection_id)"
    cli: "kask spec curate <artifact_id>"
    api: "POST /api/v1/specs/curate"

focusing_assumptions:
  - id: FA-I1
    statement: "MCP ≡ CLI ≡ API — three projections of one core"
    rationale: "Collapses entire UX specification dimension"

completeness_checklist:
  - "Every capability has all three surface entries"
  - "Equivalence matrix covers all verbs"
  - "Auth model consistent across surfaces"

cross_references:
  - category: capability
    relation: "interfaces surface capabilities"
  - category: composition
    relation: "registry discoverable through all surfaces"
```

### 5.4 Composition Spec Template

```yaml
# composition-spec.yaml
schema_version: "0.2.0"
category: composition
domain_anchor: hkask

registry:
  type: unified  # Not three separate registries
  discriminator: template_type  # WordAct | KnowAct | FlowDef
  index_method: filesystem + sqlite

cascade_rules:
  - rule: "Template cascade depth ≤ 7 (matroshka)"
  - rule: "Manifest steps execute sequentially"
  - rule: "Capability attenuation follows composition"

template_types:
  - type: Prompt
    domain: WordAct
    description: "Say — LLM prompt templates"
  - type: Process
    domain: FlowDef
    description: "Do — workflow/process templates"
  - type: Cognition
    domain: KnowAct
    description: "Think — reasoning/cognition templates"
  - type: Specification
    domain: FlowDef
    description: "Define — specification authoring templates"

focusing_assumptions:
  - id: FA-Co1
    statement: "One registry, four template types as metadata tags"
    rationale: "P1 — no trait without two consumers; C4 — repetition is missing primitive"

completeness_checklist:
  - "Registry schema defined"
  - "Cascade depth limit enforced"
  - "Template type discriminator documented"

cross_references:
  - category: capability
    relation: "composition operates on capability atoms"
  - category: domain
    relation: "templates grounded in hLexicon"
  - category: curation
    relation: "curated artifacts enter registry"
```

### 5.5 Trust & Security Spec Template

```yaml
# trust-spec.yaml
schema_version: "0.2.0"
category: trust
domain_anchor: hkask

threat_model:
  assets: [specs, manifests, signing_keys, completeness_attestations, capability_tokens, curation_decisions]
  adversaries:
    - name: malicious_template_author
      vector: template_injection
      mitigation: jinja2_sandbox + capability_gating
    - name: compromised_dependency
      vector: supply_chain
      mitigation: cargo_deny + pinned_versions
    - name: untrusted_mcp_client
      vector: capability_escalation
      mitigation: attenuation_enforcement + context_nonce
    - name: curation_override
      vector: unauthorized_curator_impersonation
      mitigation: CuratorId singleton + OCAP boundary enforcement

ocap_boundaries:
  - principle: "No ambient authority"
  - principle: "Attenuation on delegation"
  - principle: "Revocation via expiration"
  - principle: "Curation decisions require CuratorId or delegated authority"

keystore:
  encryption: AES-256-GCM
  key_derivation: Argon2id
  storage: OS_keychain + SQLCipher

focusing_assumptions:
  - id: FA-T1
    statement: "OCAP-only for v0.21.0 — no UCAN"
    rationale: "Minimize auth surface; UCAN deferred"

completeness_checklist:
  - "STRIDE-lite analysis per component"
  - "Capability attenuation policy defined"
  - "Keystore configuration documented"
  - "Curation authority bounded"

cross_references:
  - category: capability
    relation: "tokens governed by this spec"
  - category: observability
    relation: "audit spans for all security operations"
  - category: curation
    relation: "curation decisions are auditable security events"
```

### 5.6 Observability Spec Template

```yaml
# observability-spec.yaml
schema_version: "0.2.0"
category: observability
domain_anchor: hkask

cns_spans:
  - namespace: cns.tool
    covers: "Tool governance, invocation"
  - namespace: cns.prompt
    covers: "Render, validate, outcome"
  - namespace: cns.agent_pod
    covers: "Lifecycle, delegation"
  - namespace: cns.connector
    covers: "External I/O (LLM, embeddings)"
  - namespace: cns.spec
    covers: "Specification operations (capture, compose, validate, sign, curate)"

variety_counters:
  - counter: tool_diversity
    threshold: 100
    action: algedonic_alert
  - counter: template_diversity
    threshold: 50
    action: algedonic_alert
  - counter: spec_diversity
    threshold: 30
    action: algedonic_alert

algedonic:
  trigger: "variety_deficit > threshold/2 (Warning) or > threshold (Critical)"
  escalation: "Curator (Warning) → Human (Critical)"
  cooldown_seconds: 300

focusing_assumptions:
  - id: FA-O1
    statement: "CNS monitors production; tests verify correctness"
    rationale: "Separate concerns — CNS ≠ testing"

completeness_checklist:
  - "All capability invocations emit cns.* span"
  - "Variety counters configured"
  - "Algedonic alert path defined"

cross_references:
  - category: trust
    relation: "security audit spans"
  - category: lifecycle
    relation: "health monitoring"
  - category: curation
    relation: "cns.spec.curate spans for curation decisions"
```

### 5.7 Persistence Spec Template

```yaml
# persistence-spec.yaml
schema_version: "0.2.0"
category: persistence
domain_anchor: hkask

storage:
  engine: SQLite + SQLCipher
  schema: bitemporal_triples
  vector_store: sqlite-vec

memory_pipelines:
  - name: episodic
    perspective: "agent_id"
    visibility: private
  - name: semantic
    perspective: null
    visibility: public

encryption:
  at_rest: SQLCipher
  key_source: passphrase_derived (Argon2id)
  no_cross_machine_sync: true

focusing_assumptions:
  - id: FA-P1
    statement: "Local-first, Git backup — no sync"
    rationale: "User sovereignty; no cross-machine complexity"

completeness_checklist:
  - "Every domain entity has storage schema"
  - "Bitemporal semantics documented"
  - "Encryption configuration specified"

cross_references:
  - category: domain
    relation: "entities stored as triples"
  - category: trust
    relation: "encryption governed by keystore"
```

### 5.8 Lifecycle Spec Template

```yaml
# lifecycle-spec.yaml
schema_version: "0.2.0"
category: lifecycle
domain_anchor: hkask

bootstrap:
  sequence:
    - "Initialize SQLCipher database"
    - "Load hLexicon terms (including 9 spec-curation terms)"
    - "Register built-in templates (including spec templates)"
    - "Mint root capability token"
    - "Initialize Curator singleton"

evolution:
  versioning: git_sha_only  # No SemVer
  migration: "Forward-only, no rollback"
  template_evolution: "Jinja2/LLM selection, not Rust branching"

deprecation:
  policy: "Prefer deletion over deprecation (P7)"
  process: "Delete code → remove from registry → emit cns.spec.deprecated span"

focusing_assumptions:
  - id: FA-L1
    statement: "Git-only versioning — SHA-based, no SemVer"
    rationale: "Minimize versioning surface; Git is archive (PS-12)"

completeness_checklist:
  - "Bootstrap sequence defined and tested"
  - "Evolution rules documented"
  - "Deprecation policy specified"

cross_references:
  - category: composition
    relation: "registry entries evolve"
  - category: observability
    relation: "lifecycle events emit CNS spans"
  - category: curation
    relation: "Curator initialized during bootstrap"
```

### 5.9 Curation Spec Template

```yaml
# curation-spec.yaml
schema_version: "0.2.0"
category: curation
domain_anchor: hkask

curation_model:
  decision_gradient:
    - decision: Merge
      description: "Artifact accepted into collection"
    - decision: Revise
      description: "Artifact returned for revision with rationale"
    - decision: Defer
      description: "Decision postponed — needs more information"
    - decision: Discard
      description: "Artifact rejected — does not serve collection"

  curator:
    type: Replicant
    singleton: true  # CuratorId::system()
    authority: OCAPBoundary

  operations:
    - name: evaluate
      description: "Assess artifact against collection coherence"
      hlexicon_terms: [curate, evaluate]
    - name: contextualise
      description: "Situate artifact within meaningful environment"
      hlexicon_terms: [contextualise]
    - name: reconcile
      description: "Resolve goal tensions without collapsing them"
      hlexicon_terms: [reconcile]
    - name: cultivate
      description: "Grow collection toward coherence over time"
      hlexicon_terms: [cultivate]

  coherence_metric:
    method: "Weighted hLexicon term coverage + cross-reference saturation"
    threshold: 0.7

focusing_assumptions:
  - id: FA-Cu1
    statement: "Curation is evaluation, not governance"
    rationale: "Specs are invitations to curate, not gates to govern"
  - id: FA-Cu2
    statement: "Curation decisions are gradient, not binary"
    rationale: "Merge/Revise/Defer/Discard — matches existing CurationDecision enum"

completeness_checklist:
  - "All four curation decisions documented"
  - "Coherence metric defined and threshold set"
  - "Curator authority bounded via OCAPBoundary"
  - "Curation operations emit cns.spec.curate spans"

cross_references:
  - category: domain
    relation: "hLexicon grounding validated during curation"
  - category: trust
    relation: "curation decisions are auditable"
  - category: observability
    relation: "cns.spec.curate spans"
```

### 5.10 Jinja2 Spec Templates (Soft Layer) [^ronacher-jinja2]

Following hKask's loom/thread separation (Rust is loom, Jinja2/YAML is thread):

```
registry/templates/spec/
├── goal-capture.j2      # Elicit user intent as binding requirement
├── constraint-bind.j2    # Attach OCAP boundaries to goals
├── contextualise.j2      # Situate artifact within meaningful environment
├── reconcile-conflicts.j2 # Resolve goal tensions
├── curate-collection.j2  # Evaluate collection coherence
└── selector.j2           # Route input to best-fit specification template
```

**`goal-capture.j2` skeleton:**
```jinja2
{# spec/goal-capture.j2 — Elicit user intent as binding requirement #}
## Goal: {{ goal_name }}

**Domain:** {{ domain_anchor }}
**Category:** {{ spec_category }}

### Intent
When {{ situation }}, I want to {{ motivation }}, so I can {{ outcome }}.

### Criteria
{% for criterion in criteria %}
- [ ] {{ criterion.description }}
{% endfor %}

### hLexicon Grounding
{% for term in lexicon_terms %}
- **{{ term }}**: {{ hlexicon[term].definition }}
{% endfor %}

### Sub-goals
{% for sub in sub_goals %}
{{ loop.index }}. {{ sub.text }} (depth: {{ sub.depth }})
{% endfor %}
```

### 5.11 Specification Composition Manifest

```yaml
# registry/manifests/mvss-compose.yaml
manifest:
  name: mvss-compose
  description: Compose minimum viable specification set from user goals

steps:
  - ordinal: 1
    action: select
    template_ref: spec/templates/selector.j2
    model_tier: fast_local
    output_schema:
      selected_templates: array
      domain_hints: object

  - ordinal: 2
    action: populate
    template_ref: "{{ selected_templates }}"
    output_schema:
      goal_requirements: array

  - ordinal: 3
    action: execute
    target: spec/goal/capture
    mcp: hkask-mcp-spec
    output_schema:
      mvss: object
      completeness_score: float

  - ordinal: 4
    action: execute
    target: spec/curate/evaluate
    mcp: hkask-mcp-spec
    output_schema:
      curation_decision: string
      coherence_score: float
```

### 5.12 Spec-Curator Bot Manifest

```yaml
# registry/bots/spec-curator-bot.yaml
bot:
  name: spec-curator-bot
  type: Replicant
  editor: curator-or-human-admin

capabilities:
  - tool:spec/goal/capture
  - tool:spec/goal/decompose
  - tool:spec/curate/evaluate
  - tool:spec/curate/reconcile
  - tool:spec/curate/cultivate
  - tool:spec/graph/validate

process_manifest: spec/manifests/mvss-compose.yaml
```

### 5.13 hKask Self-Instantiation (Worked Example)

The templates above are instantiated for hKask (domain anchor: `hkask`, bounded context: "Agentic AI tooling"). Key self-application observations:

- **Domain spec:** hKask's hLexicon allocates 75 terms across WordAct/FlowDef/KnowAct + 9 spec-curation terms — matches FA-D1.
- **Capability spec:** 21 MCP servers × 6 actions + `hkask-mcp-spec` (8 tools) + `hkask-mcp-replicant` (3 tools) = 117 capability grant slots; current implementation covers ~60%.
- **Interface spec:** `kask` binary (CLI), `hkask-mcp` (MCP), `hkask-api` (HTTP) — all route through `hkask-agents` core. `kask spec` subcommands added.
- **Composition spec:** Unified registry with `template_type` discriminator (WordAct, KnowAct, FlowDef) — matches FA-Co1.
- **Trust spec:** `CapabilityToken` in `hkask-types` implements HMAC-SHA256 + attenuation — matches FA-T1. Curation authority bounded via `OCAPBoundary`.
- **Observability spec:** `NuEvent` in `hkask-types` with `Span` enum covers all `cns.*` namespaces. `cns.spec.curate` added.
- **Persistence spec:** Bitemporal triples in `hkask-storage` with SQLCipher — matches FA-P1.
- **Lifecycle spec:** Bootstrap in `hkask-cli`, Git-only versioning, Curator singleton initialization — matches FA-L1.
- **Curation spec:** `CurationDecision { Merge, Revise, Defer, Discard }` already exists in `hkask-types/src/curation.rs`. Spec-curator bot defined as Replicant.

**Gaps discovered (resolved 2026-05-25):**
1. ~~No `cns.spec.*` span namespace exists yet for specification operations~~ → **Resolved:** `Span::Spec` variant added (`crates/hkask-types/src/event.rs:102`)
2. ~~No `Span::Spec` variant in existing `Span` enum~~ → **Resolved:** Present in `Span` enum
3. ~~No `spec` resource in `CapabilityResource` enum~~ → Partially resolved via `Capability` type in `visibility.rs`
4. ~~No `Validate` action in `CapabilityAction` enum~~ → Partially resolved via `AccessEvaluator`
5. ~~Spec templates not yet registered in unified registry~~ → Deferred (OQ-7)
6. ~~`hkask-mcp-spec` MCP server does not yet exist~~ → **Resolved:** Implemented at 819 LOC with 8 tools (`mcp-servers/hkask-mcp-spec/`)

---

## 6. Hexagonal Architecture Mapping (Task 6) [^cockburn-hexagonal]

### 6.1 Component Diagram

```mermaid
graph TB
    subgraph DrivingPorts["Driving Ports (Left)"]
        MCP_IN["MCP Server<br/>hkask-mcp-spec"]
        CLI_IN["CLI Subcommand<br/>hkask-cli (kask spec)"]
        API_IN["HTTP API<br/>hkask-api"]
    end

    subgraph DomainCore["Domain Core (Hexagon Center)"]
        SPEC["Specification<br/>pure types"]
        GOAL_D["Goal<br/>decomposition"]
        CPRED["CompletenessPredicate<br/>verification"]
        MANIFEST_D["Manifest<br/>YAML/TOML"]
        CURATION["Curation<br/>evaluate/reconcile/cultivate"]
    end

    subgraph DrivenPorts["Driven Ports (Right)"]
        STORAGE["Storage<br/>hkask-storage"]
        MEMORY["Memory<br/>hkask-memory"]
        TEMPLATES["Templates Registry<br/>hkask-templates"]
        CNS_OUT["CNS Spans<br/>hkask-cns"]
        KEYSTORE["Keystore<br/>hkask-keystore"]
    end

    MCP_IN --> SPEC
    CLI_IN --> SPEC
    API_IN --> SPEC
    SPEC --> GOAL_D
    SPEC --> CPRED
    SPEC --> MANIFEST_D
    SPEC --> CURATION
    CURATION --> CPRED
    SPEC --> STORAGE
    SPEC --> MEMORY
    SPEC --> TEMPLATES
    SPEC --> CNS_OUT
    SPEC --> KEYSTORE
    CURATION --> CNS_OUT
    ```

<!-- DIAGRAM_ALIGNMENT
id: DIAG-DDMVSS-003
verified_date: 2026-05-24
verified_against: crates/hkask-types/src/spec.rs; crates/hkask-templates/src/manifest.rs
status: VERIFIED
-->

### 6.2 Port-to-Crate Reuse Map

| Port | Direction | Existing Crate | New Crate Needed? | Justification |
|------|-----------|---------------|-------------------|---------------|
| MCP server | Driving | `hkask-mcp` (runtime) | **Yes: `hkask-mcp-spec`** | 8 spec-specific tools following 3-file MCP server pattern; runtime alone doesn't define tools |
| CLI subcommand | Driving | `hkask-cli` | No | `kask spec` clap subcommand addition |
| HTTP API | Driving | `hkask-api` | No | Axum route addition |
| Storage | Driven | `hkask-storage` | No | Bitemporal triples store spec manifests |
| Memory | Driven | `hkask-memory` | No | Semantic recall of prior specs via embeddings |
| Templates registry | Driven | `hkask-templates` | No | Spec templates registered with `template_type: FlowDef` |
| CNS spans | Driven | `hkask-cns` | No | Add `cns.spec.*` span variant to existing `Span` enum |
| Keystore | Driven | `hkask-keystore` | No | Sign manifests with existing Ed25519 + AES-256-GCM |
| Curation engine | Domain | `hkask-types` (curation.rs) | No | `CurationDecision`, `OCAPBoundary`, `CurationRecord` already exist; add `CurationPort` trait |

**Gap analysis result:** One new MCP server crate (`hkask-mcp-spec`) justified. All other ports map to existing hKask infrastructure. The `hkask-mcp-spec` follows the established 3-file MCP server pattern (`main.rs`, `lib.rs`, `tools.rs`) used by `hkask-mcp-inference`, `hkask-mcp-web`, etc.

### 6.3 `hkask-mcp-spec` Tool Surface

| Tool | Input | Output | hLexicon Terms |
|------|-------|--------|----------------|
| `spec/goal/capture` | `{description, context, user_id}` | `{goal_id, requirements[], graph_position}` | `specify`, `require`, `elicit` |
| `spec/goal/decompose` | `{goal_id}` | `{sub_goals[], dependencies[]}` | `decompose`, `sequence` |
| `spec/require/bind` | `{goal_id, constraint}` | `{requirement_id, ocap_boundaries}` | `constrain`, `require` |
| `spec/curate/evaluate` | `{artifact_id, collection_id}` | `{decision, rationale}` | `curate`, `evaluate`, `contextualise` |
| `spec/curate/reconcile` | `{artifact_ids[], conflicts[]}` | `{resolution, tensions_preserved[]}` | `reconcile`, `compose` |
| `spec/curate/cultivate` | `{collection_id, time_horizon}` | `{growth_plan, coherence_score}` | `cultivate` |
| `spec/graph/query` | `{query, depth}` | `{nodes[], edges[], paths[]}` | `recognize`, `match` |
| `spec/graph/validate` | `{collection_id}` | `{violations[], suggestions[]}` | `evaluate`, `ground` |

### 6.4 Implementation Estimate

| Component | Estimated Scope |
|-----------|----------------|
| `hkask-mcp-spec` (3-file MCP server) | Core server |
| `hkask-types/src/spec.rs` (domain types) | Domain types |
| `hkask-types/src/event.rs` (Span::Spec variant) | Span variant |
| `hkask-types/src/capability.rs` (Spec resource, Validate action) | Capability types |
| `hkask-types/src/lexicon.rs` (9 bootstrap terms) | Lexicon terms |
| `hkask-cli` (spec subcommands) | CLI surface |
| `hkask-api` (spec routes) | API surface |
| Jinja2 templates (6 files) | Templates |
| YAML manifests + bot manifest | Manifests |
| Tests (unit + integration) | Test coverage |

**Discipline:** Every component is essential and minimal — ask "is this necessary?" before "how big is it?"

---

## 7. Capability & Security Design (Task 7)

### 7.1 Threat Model (Schneier-grade, lean) [^shostack-threat]

**Assets:**

| Asset | Sensitivity | Protection |
|-------|------------|------------|
| Spec manifests | Medium | Signed at rest, integrity-checked on load |
| Capability grants | High | OCAP tokens, HMAC-SHA256, attenuation enforced |
| Signing keys | Critical | OS keychain + Argon2id derivation, never in memory > TTL |
| Completeness attestations | Medium | Signed by Curator WebID, stored as bitemporal triples |
| Curation decisions | Medium | Attributed to CuratorId, auditable via `cns.spec.curate` spans |

**Adversaries:**

| Adversary | Capability | Attack Vector | Likelihood |
|-----------|-----------|---------------|------------|
| Malicious template author | Craft templates that exfiltrate spec content | Jinja2 injection, path traversal | Medium |
| Compromised dependency | Inject code into spec validation pipeline | Supply chain (Cargo.toml) | Low |
| Untrusted MCP client | Escalate capabilities beyond granted scope | Capability forgery, replay attacks | Medium |
| Curation override | Impersonate Curator to force Merge decisions | CuratorId forgery, OCAP boundary bypass | Low |

**Mitigations:**

| Threat | Mitigation | hKask Primitive |
|--------|-----------|-----------------|
| Template injection | Jinja2 sandbox + `internal_safe_search` | `minijinja` with sandboxing |
| Path traversal | Path validation at storage boundary | `hkask-storage` path guards |
| Capability forgery | HMAC-SHA256 verification + constant-time comparison | `CapabilityToken::verify()` |
| Capability escalation | Attenuation enforcement (`attenuation_level < max_attenuation`) | `CapabilityToken::attenuate()` |
| Spec tampering | Ed25519 manifest signing via keystore | `hkask-keystore` |
| Replay attacks | Context nonce binding + token expiry | `CapabilityToken.context_nonce` |
| Data at rest exposure | SQLCipher encryption | `hkask-storage` SQLCipher |
| Curation override | CuratorId singleton + OCAPBoundary enforcement | `hkask-types/src/curation.rs` |

### 7.2 Capability Grant Table

| Operation | Resource | Action | Capability Required | Attenuatable? | CNS Span |
|-----------|----------|--------|-------------------|---------------|----------|
| Read spec | `spec:{id}` | Read | `spec:read` | Yes | `cns.spec.read` |
| Amend spec | `spec:{id}` | Write | `spec:write` | Yes | `cns.spec.write` |
| Compose specs | `spec:*` | Compose | `spec:compose` | Yes | `cns.spec.compose` |
| Sign manifest | `manifest:{id}` | Execute | `manifest:sign` | No (root only) | `cns.spec.sign` |
| Publish spec | `spec:{id}` | Execute | `spec:publish` | No (root only) | `cns.spec.publish` |
| Validate completeness | `spec:{id}` | Validate | `spec:validate` | Yes | `cns.spec.validate` |
| Delegate spec access | `spec:{id}` | Attenuate | `spec:delegate` | Yes (always) | `cns.spec.delegate` |
| Evaluate artifact | `spec:{id}` | Execute | `spec:curate` | Yes | `cns.spec.curate` |
| Reconcile conflicts | `spec:*` | Compose | `spec:curate` | Yes | `cns.spec.curate` |
| Cultivate collection | `spec:*` | Write | `spec:curate` | No (Curator only) | `cns.spec.curate` |

**POLA enforcement:** Every spec operation requires presenting a `CapabilityToken` with matching `(resource, resource_id, action)`. No ambient authority. The MVSDD tool itself holds only `spec:*` capabilities — it cannot access `tool:*` or `template:*` resources. Curation operations are attenuatable except `cultivate`, which requires CuratorId authority.

### 7.3 Implementation Status (Post ADV-REVIEW-F2)

The security hardening completed in ADV-REVIEW-F2 (T01-T22, 2026-05-24) implements the following DDMVSS categories:

| Category | Implementation | Status | Evidence |
|----------|---------------|--------|----------|
| **Trust & Security** | Unified CapabilityToken with caveats, OCAP enforcement at all boundaries, secure memory (`Arc<Zeroizing<Vec<u8>>>`) | ✅ Complete | [`trust-security-observability.md`](trust-security-observability.md), [`ADR-022-comprehensive-security-hardening.md`](ADR-022-comprehensive-security-hardening.md) |
| **Capability** | Single primitive (`CapabilityToken`), attenuation chains (max 7 levels), persistent revocation tracking | ✅ Complete | [`domain-and-capability.md`](domain-and-capability.md) §3 |
| **Observability** | CNS spans on all capability mutations (`cns.cap.minted`, `cns.cap.attenuated`, `cns.cap.revoked`, `cns.cap.verified_ok`, `cns.cap.verified_denied`) | ✅ Complete | [`domain-and-capability.md`](domain-and-capability.md) §11 |
| **Lifecycle** | Deterministic WebID derivation (UUID v5 from persona content), persistent revocation survives restarts | ✅ Complete | [`domain-and-capability.md`](domain-and-capability.md) §2 |
| **Curation** | `AuditLogPort` dual-write (in-memory cache + SQLite storage), CNS span emission for audit trail | ⚠️ Partial | Curation decisions not yet gradient-evaluated (Merge/Revise/Defer/Discard) |
| **Domain** | Bounded context: "Agentic AI tooling". ν-events: `cns.agent_pod.*`, `cns.cap.*`. Entities: `AgentPod`, `CapabilityToken`, `WebID` | ✅ Complete | [`hKask-architecture-master.md`](hKask-architecture-master.md) |
| **Interface** | Hexagonal ports: `AcpPort`, `GitCASPort`, `MCPRuntimePort`, `MemoryStoragePort`, `CnsEmit`, `KeystorePort`, `SovereigntyPort`. All async (`#[async_trait]`) | ✅ Complete | [`reference/ports-inventory.md`](reference/ports-inventory.md) |
| **Composition** | Russell ACP bridge with session lifecycle, bidirectional federation via JSON-RPC 2.0 over stdio | ✅ Complete | [`domain-and-capability.md`](domain-and-capability.md) §6 |
| **Persistence** | `MemoryStoragePort` wired into pod lifecycle, episodic/semantic memory for lifecycle events | ✅ Complete | [`domain-and-capability.md`](domain-and-capability.md) §7 |

**Gaps Identified (updated 2026-05-25):**

1. ~~**No `cns.spec.*` span namespace**~~ → **Resolved:** `Span::Spec` variant present
2. ~~**No `Spec` resource** in `CapabilityResource` enum~~ → Partially resolved via `Capability` type
3. ~~**No `Validate` action** in `CapabilityAction` enum~~ → Partially resolved via `AccessEvaluator`
4. **Spec templates not yet registered** in unified registry (`template_type: FlowDef`) → Deferred (OQ-7)
5. ~~**`hkask-mcp-spec` MCP server does not yet exist**~~ → **Resolved:** 819 LOC, 8 tools implemented
6. **Curation decisions not gradient-evaluated** — `DefaultSpecCurator` implements `SpecCurator` trait but gradient evaluation needs integration testing

**Next Steps:**

- Implement `hkask-mcp-spec` MCP server (~500 LOC, see §6.4 Implementation Estimate)
- Add `Spec` resource and `Validate` action to `hkask-types/src/capability.rs`
- Add `cns.spec.*` span namespace to `hkask-types/src/event.rs`
- Register spec templates in unified registry
- Implement gradient curation evaluation in `AuditLogPort`

See [`trust-security-observability.md`](trust-security-observability.md) for implementation details and [`trust-security-observability.md`](trust-security-observability.md) for the DDMVSS-aligned security architecture.

---

## 8. Rust Type-Level Skeleton (Task 8)

```rust
//! hkask-storage/src/spec_types.rs — DDMVSS domain types
//!
//! Load-bearing skeleton: types + traits only, no business logic.
//! NOTE: This skeleton shows the DDMVSS type design. The actual implementation
//! lives in hkask-storage/src/spec_types.rs (not hkask-types).

use crate::id::WebID;
use crate::capability::{CapabilityResource, CapabilityAction};
use crate::curation::{CurationDecision, OCAPBoundary};
use crate::visibility::Visibility;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Newtype IDs ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpecId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub Uuid);

impl SpecId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

impl CapabilityId {
    pub fn new() -> Self { Self(Uuid::new_v4()) }
}

// ── Domain Core ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecCategory {
    Domain, Capability, Interface, Composition,
    Trust, Observability, Persistence, Lifecycle, Curation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: SpecId,
    pub name: String,
    pub category: SpecCategory,
    pub domain_anchor: DomainAnchor,
    pub goals: Vec<GoalSpec>,
    pub version_sha: String,
    pub signed_by: Option<WebID>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalSpec {
    pub id: crate::id::GoalID,
    pub text: String,
    pub criteria: Vec<Criterion>,
    pub sub_goals: Vec<GoalSpec>,  // Recursive: fixed-point, not loop
    pub depth: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Criterion {
    pub description: String,
    pub satisfied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainAnchor { Okapi, Russell, Hkask }

// ── Completeness Predicate ───────────────────────────────────

pub trait CompletenessCheck {
    fn is_complete(&self) -> bool;
}

impl CompletenessCheck for GoalSpec {
    fn is_complete(&self) -> bool {
        self.criteria.iter().all(|c| c.satisfied)
            && self.sub_goals.iter().all(|g| g.is_complete())
    }
}

impl CompletenessCheck for Spec {
    fn is_complete(&self) -> bool {
        self.goals.iter().all(|g| g.is_complete())
    }
}

// ── Curation Integration ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecCurationRecord {
    pub spec_id: SpecId,
    pub decision: CurationDecision,
    pub rationale: String,
    pub coherence_score: f64,
    pub ocap_boundary: OCAPBoundary,
    pub curated_at: DateTime<Utc>,
}

pub trait SpecCurator {
    fn evaluate(&self, spec: &Spec) -> Result<SpecCurationRecord, SpecError>;
    fn reconcile(&self, specs: &[Spec]) -> Result<Vec<SpecCurationRecord>, SpecError>;
    fn cultivate(&self, specs: &mut Vec<Spec>) -> Result<f64, SpecError>;
}

// ── Ports (traits) ───────────────────────────────────────────

pub trait SpecStore {
    fn load(&self, id: SpecId) -> Result<Spec, SpecError>;
    fn save(&self, spec: &Spec) -> Result<(), SpecError>;
    fn list_by_category(&self, cat: SpecCategory) -> Result<Vec<Spec>, SpecError>;
}

// SpecObserver trait removed in v0.23.0 — CNS span emission replaces it

// ── Error ────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum SpecError {
    #[error("Spec not found: {0}")]
    NotFound(SpecId),
    #[error("Capability denied: {0}")]
    CapabilityDenied(String),
    #[error("Signature invalid")]
    InvalidSignature,
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Depth limit exceeded: max 7")]
    DepthExceeded,
    #[error("Curation authority required")]
    CurationDenied,
    #[error("Coherence below threshold: {0}")]
    CoherenceInsufficient(f64),
}

// ── Adapter stub (pattern demonstration) ─────────────────────

pub struct SqliteSpecStore { /* hkask-storage connection */ }

impl SpecStore for SqliteSpecStore {
    fn load(&self, id: SpecId) -> Result<Spec, SpecError> {
        Err(SpecError::NotFound(id))
    }
    fn save(&self, _spec: &Spec) -> Result<(), SpecError> {
        Ok(())
    }
    fn list_by_category(&self, _cat: SpecCategory) -> Result<Vec<Spec>, SpecError> {
        Ok(vec![])
    }
}
```

**Design notes:**
- `GoalSpec.sub_goals` is recursive (fixed-point) — max depth 7 matches existing `Goal::can_have_subgoals()`.
- All I/O returns `Result<T, SpecError>` — no `unwrap` outside tests.
- Traits define ports; `SqliteSpecStore` demonstrates the adapter pattern.
- Zero `async` in domain core — async only at adapter boundary (via `async-trait` in adapters).
- Newtypes `SpecId`, `CapabilityId` follow existing `hkask-types/src/id.rs` conventions.
- `SpecCurationRecord` integrates with existing `CurationDecision` (3 variants: Merge, Discard, Revise) from `curation.rs`. The `Defer` variant was removed in v0.23.0.
- `SpecCurator` trait has ≥2 potential consumers (spec-curator bot, human-via-CLI) — P1 compliant.
- `SpecCategory` reduced to 4 live variants (Domain, Capability, Interface, Composition).
- `SpecError::CurationDenied` and `SpecError::CoherenceInsufficient` — distinct recovery paths per C5.

---

## 9. Self-Application Validation (Task 9)

**Test:** Can DDMVSS specify the MVSDD tool itself?

### 9.1 Self-Application Matrix

| DDMVSS Category | MVSDD Tool Spec | Status |
|-----------------|-----------------|--------|
| Domain | Bounded context: "Specification authoring & validation". ν-events: `cns.spec.*` (capture, compose, validate, sign, curate). Entities: `Spec`, `GoalSpec`, `Criterion`, `SpecCurationRecord`. | **Pass** — all entities defined in §8 |
| Capability | Verbs: `read_spec`, `amend_spec`, `compose_specs`, `sign_manifest`, `validate_completeness`, `delegate_access`, `curate_artifact`. All attenuatable except sign/publish/cultivate. | **Pass** — grant table in §7.2 |
| Interface | MCP: `hkask-mcp-spec` (8 tools). CLI: `kask spec <subcommand>`. API: `POST /api/v1/specs/*`. All equivalent via `SpecStore` port. | **Pass** — three surfaces, one core |
| Composition | Specs compose via `Spec.goals` aggregation. Registry stores spec templates with `template_type: FlowDef`. | **Pass** — unified registry |
| Trust | OCAP tokens govern all spec ops. Manifests signed via `hkask-keystore`. Curation authority bounded via `OCAPBoundary`. Threat model in §7.1. | **Pass** |
| Observability | `cns.spec.*` spans for all operations including curation. Variety counter on spec diversity. | **:partial** — Span::Spec exists in code. Variety counter on spec diversity not wired to algedonic system. SpecDriftAlert defined but not in CNS loop. |
| Persistence | Specs stored as bitemporal triples in `hkask-storage`. Embeddings for semantic recall of prior specs. | **:partial** — TripleStore has bitemporal columns, but SpecStore uses `created_at` only (valid_from/valid_to absent). Curation records not persisted (§11 R3.8). |
| Lifecycle | Bootstrap: register spec templates + initialize Curator. Evolution: Git SHA versioning. Deprecation: delete per P7. | **:drift** — `version_sha` removed as dead field; no replacement exists. Spec versioning absent. Bootstrap and deprecation are operational. |
| Curation | `CurationDecision` gradient (Merge/Revise/Defer/Discard) from `curation.rs`. Spec-curator bot as Replicant. Coherence metric defined. | **:drift** — CurationDecision has 4 variants (Defer added per audit remediation R2). Coherence threshold 0.7 uncalibrated (§10.3). Curation records not persisted (§11 R3.8). |

### 9.2 Gaps Discovered & Corrections

| # | Gap | Category | Correction |
|---|-----|----------|-----------|
| 1 | `Span::Spec(String)` variant missing | Observability | Add to `hkask-types/src/event.rs` `Span` enum |
| 2 | No `Spec` resource in `CapabilityResource` | Capability | Add `Spec` variant to `CapabilityResource` enum |
| 3 | No `Validate` action in `CapabilityAction` | Capability | Add `Validate` variant to `CapabilityAction` enum |
4 | Spec templates not registered | Composition | Register `.yaml` templates as `template_type: FlowDef` in unified registry |
| 5 | `hkask-mcp-spec` does not exist | Interface | Create 3-file MCP server (~500 LOC) per §6.3 |
| 6 | 9 hLexicon spec-curation terms not bootstrapped | Domain | Add to `lexicon.rs::bootstrap()` per §3.3 |

**Verdict:** DDMVSS passes self-application across all 9 categories with 6 gaps, all addressable by extending existing enums or adding one MCP server crate. The framework is recursively sound.

---

## 10. Open / Underspecified Questions (Task 10)

1. **Meta-completeness:** How is the completeness predicate itself validated? Currently `CompletenessCheck` is a trait implemented per type — but who verifies the trait implementation is correct? This bootstraps to: the completeness predicate is validated by the same MVSDD cycle that produces it (recursive closure), but this is a *claim*, not a *proof*. Formal verification of `is_complete()` is deferred.

2. **Cross-MVSS boundary conflicts:** When Okapi's DDMVSS and hKask's DDMVSS disagree on a shared interface contract (e.g., inference API shape), resolution is currently ad-hoc (human negotiation). A protocol is needed — likely: the *consuming* domain's spec takes precedence at the boundary, with the *providing* domain's spec governing internals.

3. **Versioning & evolution:** Specs are Git-SHA versioned (no SemVer). A v2 capability that attenuates v1 must be expressible as a new `CapabilityToken` with `parent: v1_token_id`. But what happens to dependents holding v1 tokens when v2 supersedes? Currently: v1 tokens expire naturally via TTL. This may be insufficient for long-lived specs.

4. **Empirical minimum goal count:** Is there a discoverable minimum number of goals before `complete?` flips true? Hypothesis: for a domain with N verbs, the minimum is approximately 2N goals (one capability + one constraint per verb). This is untested.

5. **CNS algedonic coupling:** When does spec-drift (a spec's actual implementation diverging from its goals) trigger an algedonic alert? Currently: CNS monitors variety counters, not spec-drift. A `cns.spec.drift` span with a drift-magnitude metric is needed.

6. **Cross-domain composition:** Can DDMVSS for Okapi + Russell + hKask be *summed* into a tri-domain spec? Current answer: **federated, not summed.** Each domain maintains its own MVSS; cross-domain capabilities are delegated tokens that cross boundaries. A "sum" operation would require a meta-domain, which violates bounded-context discipline.

7. **Goal ≡ Requirement naming:** Does the bidirectional equivalence survive contact with stakeholders who use "goal" and "requirement" differently? In DDMVSS, `Goal` is the internal type name; `Requirement` is the external-facing alias in templates. This is a *naming as cybernetic act* — the vocabulary shapes the thinking. If stakeholders resist, the framework should support configurable aliases without changing the type system.

8. **Atomic spec:** Is there a smallest non-trivial `Specification` that is still a `Specification`? Candidate: `Spec { goals: [GoalSpec { text: "System CAN <verb> on <resource>", criteria: [Criterion { satisfied: true }], sub_goals: [] }] }` — one goal, one criterion, zero sub-goals. This is the atomic unit. Anything smaller (zero goals) is not a specification; anything with sub-goals decomposes into atoms.

9. **Bootstrap loading order:** Are specification manifests and selector templates loaded by convention from fixed paths, or is there a Rust bootstrap sequence? Current assumption: filesystem convention (`registry/templates/spec/`, `registry/manifests/`). If dynamic loading is needed, the `SpecStore::load` port must accept a path parameter.

10. **Specification hot-reload:** When YAML/Jinja2/.md spec files change on disk, does Rust detect (fswatch/notify), or must it be signaled (API/CLI `kask spec reload`)? Affects caching strategy in `hkask-templates`. Default: signal-based reload via CLI command.

11. **Selector failure handling:** If the fast model returns confidence below threshold during template selection (step 1 of `mvss-compose`), does the manifest support conditional steps (`choice` in FlowDef), or is fallback handled outside the manifest by the Rust executor? Current assumption: Rust executor handles fallback (try fast → balanced → high_quality). Manifest step grammar remains `select|populate|execute`.

12. **Curation decision authority:** When `CurationDecision::Revise` is returned, who performs the revision — the Curator bot, the human, or the original author? Current answer: the entity that holds the `spec:write` capability for that artifact. The curation engine returns the decision; the capability holder acts on it.

13. **Coherence metric calibration:** How is `coherence_score` calculated? Current definition: weighted hLexicon term coverage + cross-reference saturation. The threshold (0.7) is a starting guess. Empirical calibration requires operational data from specification curation sessions.

14. **Sovereignty override:** Can a user override MVSS requirements? If so, through what mechanism — consent grant, capability token mint, or curator appeal? Current answer: user sovereignty (Principle 1.3) means the user can always mint new capabilities via `hkask-keystore`. The MVSS cannot prevent this; it can only emit a `cns.spec.override` span recording the deviation.

---

## 11. Adversarial Review Remediation (2026-05-24)

### Round 1 — Completed

| Task | Weakness | Status |
|------|----------|--------|
| T5 | `SpecId::from_string` silent UUID generation on parse failure | **Fixed** — returns `Result<Self, SpecError>` |
| T6 | `version_sha` dead field on `Spec` | **Fixed** — removed |
| T7 | `compute_coherence` private method instead of trait | **Fixed** — `coherence()` on `CompletenessCheck` trait |
| T3 | `format!` JSON in MCP tools | **Fixed** — `serde_json::to_string` with typed response structs |
| T1 | Three surfaces don't share state | **Fixed** — `SqliteSpecStore` implements `SpecStore` port |
| T4 | `SpecCurator` has zero implementations | **Fixed** — `DefaultSpecCurator` implements `SpecCurator` |
| T2 | MCP tools perform no capability checking | **Fixed** — `CapabilityToken` verified per-request |
| T8 | `hkask-mcp-spec` not registered in runtime | **Fixed** — `register_spec_server` in builtin servers |
| T9 | Jinja2 templates disconnected from rendering | **Fixed** — `kask spec render` CLI subcommand |

### Round 2 — Completed

| Task | Weakness | Status |
|------|----------|--------|
| R2-T6 | `spec_curate_evaluate` hardcoded coherence | **Fixed** — uses `spec.coherence()` |
| R2-T4 | `SqliteSpecCurator` misnamed (no SQLite) | **Fixed** — renamed `DefaultSpecCurator` |
| R2-T8 | `CompletenessCheck` for `[Spec]` conflates local/global semantics | **Fixed** — split into `CollectionCoherence` trait |
| R2-T1 | `SpecServer` HashMap shadow; reads never consult store | **Fixed** — HashMap removed; store is single source of truth |
| R2-T3 | `spec_require_bind` is a no-op | **Fixed** — mutates `GoalSpec.constraints` and persists |
| R2-T2 | `verify_capability` always returns `Ok(())` | **Fixed** — decodes base64 token, verifies signature, checks resource/action |
| R2-T5 | `persist_spec` silently discards errors | **Fixed** — errors propagated; `CnsSpecObserver` emits `cns.spec.*` spans |
| R2-T7 | CLI `Render` never loads spec from store | **Fixed** — loads via `SpecStore::load`, populates minijinja context |
| R2-T9 | `spec_curate_reconcile` echoes without analysis | **Fixed** — Jaccard similarity on goal word tokens; `TensionReport` in response |

### Round 3 — Deferred

1. **`SpecStore` needs `Send + Sync` bounds on the trait itself.** Currently the bounds are only on the `SpecServer` field type (`Arc<dyn SpecStore + Send + Sync>`). The trait should declare these bounds so all adapters are forced to be thread-safe. This is a breaking change to the trait signature.

2. **`SpecStore` needs bitemporal semantics.** The current schema stores a single `created_at` timestamp. The DDMVSS spec calls for bitemporal triples (valid-time + transaction-time). This requires extending the schema with `valid_from`, `valid_to`, `recorded_at` columns and updating `save`/`load` to accept a temporal context.

3. **`SpecSigner` implementation via `hkask-keystore` Ed25519.** The `signed_by: Option<WebID>` field remains `None`. A `KeystoreSpecSigner` adapter in `hkask-keystore` should sign the spec's canonical JSON serialization and store the signature alongside `signed_by`.

4. **Capability token minting for spec operations.** No code path mints `spec:read`/`spec:write`/`spec:compose` tokens. The Curator bot needs these to operate. Add `grant_spec` convenience method usage to the bootstrap sequence.

5. **`SpecObserver` → CNS span integration depth.** The `CnsSpecObserver` adapter emits `tracing::info!` spans. For full CNS integration, these should feed into `SpanEmitter` variety counters and trigger algedonic alerts when spec diversity drops below threshold.

6. **Cross-surface equivalence test.** No integration test verifies that MCP `spec_goal_capture`, CLI `kask spec capture`, and API `POST /api/specs/capture` produce identical `Spec` objects through the shared `SpecStore`. This is the load-bearing test for the `MCP ≡ CLI ≡ API` axiom.

7. **Coherence threshold calibration.** The 0.7 default is a guess. `DefaultSpecCurator::cultivate` should track historical coherence scores in a `curation_history` table and compute an empirical threshold.

8. **Persistent curation audit trail.** `CurationRecord` from `curation.rs` should be stored as bitemporal triples when `SpecCurator::evaluate` is called. Currently decisions are returned but not persisted.

9. **Manifest step grammar extension.** ~~`mvss-compose.yaml` uses `select|populate|execute` actions. If `validate` or `curate` actions are needed, the manifest executor in `hkask-templates` must be extended.~~ **Done (v0.23.0):** The manifest executor now supports `select`, `populate`, `execute`, `feedback`, `validate`, and `retrieve` actions. See `executor.rs` and `bundle.rs`.

10. **Spec drift detection.** `cns.spec.drift` span with drift-magnitude metric is specified in §10.5 but not implemented. Requires comparing `Spec` goals against actual implementation state — a non-trivial feedback loop.

---

## 12. Testing Protocol

This section codifies testing practices as normative DDMVSS requirements. Full detail lives in [`TESTING_STANDARDS.md`](../specifications/TESTING_STANDARDS.md).

### 12.1 Principles

| ID | Principle | Enforcement |
|----|-----------|-------------|
| TP-1 | Tests verify behavior through public interfaces, not implementation details | Review gate: classify every new test as Public Interface, Seam Integration, or Implementation-Coupled |
| TP-2 | Vertical slicing: one test → one implementation → repeat | Review gate: no PR merges a batch of tests without corresponding implementation |
| TP-3 | The interface is the test surface — hard-to-test modules signal shallow interfaces | Architecture review: flag modules with only implementation-coupled tests for deepening |
| TP-4 | Write regression tests before fixes, but only if a correct seam exists | If no seam exists, that is an architecture finding, not a testing gap |
| TP-5 | Implementation-coupled tests are technical debt tracked with `TEST-DEBT` comments | `grep -r "TEST-DEBT" crates/ --include="*.rs" | wc -l` must decrease over time |
| TP-6 | Every DDMVSS requirement maps to at least one test or a documented `GAP` | Traceability matrix `Tests` column: `— GAP` for untested requirements, never bare `—` |
| TP-7 | Skill-based workflows govern testing practices | Project-local skills in `.agents/skills/` are normative references |

### 12.2 Skill References

| Skill | DDMVSS Role | Location |
|-------|------------|----------|
| `tdd` | Red-green-refactor with vertical slicing | `.agents/skills/tdd/SKILL.md` |
| `diagnose` | Build feedback loop before hypothesizing | `.agents/skills/diagnose/SKILL.md` |
| `improve-codebase-architecture` | Identify shallow modules, deepen seams | `.agents/skills/improve-codebase-architecture/SKILL.md` |
| `coding-guidelines` | Surgical changes, simplicity first, goal-driven | `.agents/skills/coding-guidelines/SKILL.md` |
| `zoom-out` | Module map, caller graph, data flow | `.agents/skills/zoom-out/SKILL.md` |
| `grill-me` | Socratic interrogation of design decisions | `.agents/skills/grill-me/SKILL.md` |
| `skill-bundler` | Compose multiple skills into coordinated sessions | `.agents/skills/skill-bundler/SKILL.md` |

### 12.3 Category → Test Strategy Summary

| Category | Primary Seam | Key Invariant | Anti-Pattern |
|----------|-------------|---------------|-------------|
| Domain | `WebID`, `NuEvent`, `HLexicon` public APIs | hLexicon round-trips | Testing internal hashmap structure |
| Capability | `Capability`, `Delegation`, `AcpRuntime` traits | Fail-closed: no checker → denied | Testing HMAC internals rather than attenuation |
| Interface | CLI ↔ API ↔ MCP equivalence | `MCP ≡ CLI ≡ API` for every operation | Testing only one surface |
| Composition | `SqliteRegistry`, `TemplateResolver`, `ContractValidator` | Cascade terminates within depth limit | Testing Jinja2 string manipulation in isolation |
| Trust & Security | `SecurityGateway`, `AcpRuntime`, key derivation | Security boundaries never relaxed by default | Only testing happy paths |
| Observability | `CnsObserver`, `SseObserver`, `AlgedonicManager` | Alerts fire at threshold/2 (warning), threshold (critical) | Testing `tracing::info!` format rather than observer behavior |
| Persistence | Repository traits (`GoalRepository`, `TripleStore`, `SpecStore`) | Bitemporal queries correct; encrypted storage fails without key | Testing SQL query strings rather than repository behavior |
| Lifecycle | `main()` entry point, migration functions | Forward-only evolution — no rollback | Testing CLI arg parsing in isolation |
| Curation | `SpecCurator`, `SpecStore`, MCP spec tool handlers | Coherence threshold gates curation decisions | Testing Jaccard similarity without full pipeline |

### 12.4 Test Gap Priority

Priority is determined by risk: security and correctness-critical paths first.

| Priority | Category | Gap | Target Seam |
|----------|----------|-----|-------------|
| P0 | Trust & Security | Fail-closed capability checker | `CapabilityChecker` trait |
| P0 | Trust & Security | Per-replicant key derivation | `AcpRuntime::derive_agent_secret` |
| P0 | Trust & Security | Encrypted storage at rest | `Database` (SQLCipher) |
| P1 | Capability | OCAP attenuation depth | `Delegation` trait |
| P1 | Interface | MCP ≡ CLI ≡ API parity | `GoalServer`, `goal_router`, `kask goal` |
| P1 | Interface | CNS SSE endpoint | `SseObserver` |
| P2 | Observability | Algedonic alert thresholds | `AlgedonicManager` |
| P2 | Composition | Template cascade depth | `TemplateResolver` |
| P2 | Persistence | Bitemporal triple storage | `TripleStore` |
| P3 | Domain | hLexicon drift detection | `ContractValidator` |
| P3 | Lifecycle | Bootstrap sequence | `main()` |
| P3 | Curation | Spec curation pipeline | `DefaultSpecCurator` |

### 12.5 Self-Application

DDMVSS self-application (§9) is extended: the Testing Protocol applies to this specification itself. Every DDMVSS requirement must have a corresponding test or a documented `GAP` in the traceability matrix. The `spec_curate_test_verify` tool in `hkask-mcp-spec` can validate this.

---

## 11. References

[^evans-ddd]: Evans, E. (2003). *Domain-Driven Design: Tackling Complexity in the Heart of Software*. Addison-Wesley. The "Domain-Driven" in DDMVSS derives from Evans's pattern of bounding a model within an explicit context, ubiquitous language, and anti-corruption layers.

[^beer-vsm]: Beer, S. (1972). *Brain of the Firm*. Penguin Books. Viable System Model.

[^ashby-law]: Ashby, W. R. (1956). *An Introduction to Cybernetics*. Chapman & Hall. Law of Requisite Variety.
[^w3c-rdf]: World Wide Web Consortium. (2014). *RDF 1.1 Concepts and Abstract Syntax*. W3C Recommendation. https://www.w3.org/TR/rdf11-concepts/
[^ries-lean]: Ries, E. (2011). *The Lean Startup: How Today's Entrepreneurs Use Continuous Innovation to Create Radically Successful Businesses*. Crown Business.
[^snowden-cynefin]: Snowden, D. J., & Boone, M. E. (2007). A leader's framework for decision making. *Harvard Business Review*, 85(11), 68–76.
[^miller-robust]: Miller, M. S. (2006). *Robust composition: Towards a unified approach to access control and concurrency control* [Doctoral dissertation, Johns Hopkins University]. https://miller.emulab.net/papers/robust-composition.pdf
[^brown-c4]: Brown, S. (2020). *The C4 Model for Visualising Software Architecture*. https://c4model.com/
[^cockburn-hexagonal]: Cockburn, A. (2005). *Hexagonal Architecture*. https://alistair.cockburn.us/hexagonal-architecture/
[^shostack-threat]: Shostack, A. (2014). *Threat Modeling: Designing for Security*. Wiley.
[^ronacher-jinja2]: Ronacher, A. (2024). *Jinja*. Pallets Projects. https://jinja.palletsprojects.com/

---

*DDMVSS v0.2.0 — Domain-Driven Minimum Viable Specification Set for hKask*
*Self-applying, recursively sound, curated not governed.*
