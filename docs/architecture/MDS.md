---
title: "MDS — Minimal Domain Specification"
audience: [architects, developers, agents]
last_updated: 2026-06-09
version: "0.27.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# MDS — Minimal Domain Specification

**Purpose:** A minimal, capability-driven specification framework for hKask. Specs are grants ("CAN verb on resource via interface"), not fences ("MUST NOT"). Five categories, six tools, one completeness predicate.

**Supersedes:** [`MDS.md`](MDS.md) — the previous 9-category Domain-Driven Minimum Viable Specification Set is archived. All MDS references in the codebase should be updated to MDS.

**Related:** [`PRINCIPLES.md`](PRINCIPLES.md), [`magna-carta.md`](magna-carta.md), [`loop-architecture.md`](loop-architecture.md)

---

## 1. Five Categories

| # | Category | Completeness Predicate | Min Artifacts | Cross-References |
|---|----------|----------------------|---------------|-----------------|
| 1 | **Domain** | Every entity has a named term in hLexicon and a bounded-context map | hLexicon allocation, domain ontology sketch | → Composition (verbs), → Lifecycle (persistence) |
| 2 | **Composition** | Every domain verb has a granted composition, registered interface, and composable path | Capability grant table, interface equivalence matrix, registry schema | → Domain (ontology), → Trust (tokens) |
| 3 | **Trust** | Every capability operation has a threat-model entry and an OCAP-bound mitigation | Threat model, keystore config, capability attenuation policy | → Composition (capabilities), → Lifecycle (audit) |
| 4 | **Lifecycle** | Bootstrap, evolution, deprecation, lifecycle, and persistence are expressible as spec transitions | Bootstrap manifest, evolution rules, deprecation policy, CNS span registry | → Domain (entities), → Trust (audit) |
| 5 | **Curation** | Every spec artifact has been evaluated for coherence by a curator with documented rationale | Curation decision log, coherence score | → Domain (hLexicon grounding), → Lifecycle (health) |

---

## 2. Completeness Predicate

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
    curation_decision ∈ {Merge, Revise, Defer, Discard}
    ∧ decision.rationale documented
```

A goal-set G is **MDS-complete** iff `complete?(G, c)` holds for all 5 categories **and** `curated?(G)` holds.

Curation decisions (Merge/Revise/Defer/Discard) are made by the Curator or human — not by a tool in the spec server. The spec server validates coherence; the Curator makes decisions.

---

## 3. Spec Tool Surface (`hkask-mcp-spec`)

Six tools. Three were deleted (evaluate, reconcile, cultivate) as agent-hallucinated curation logic. Curation is external to the spec server.

| # | Tool | Input | Output | hLexicon Terms |
|---|------|-------|--------|----------------|
| 1 | `spec/goal/capture` | `{description, context}` | `{goal_id, requirements[]}` | `specify`, `elicit` |
| 2 | `spec/goal/decompose` | `{goal_id}` | `{sub_goals[], dependencies[]}` | `decompose`, `sequence` |
| 3 | `spec/require/bind` | `{goal_id, constraint}` | `{requirement_id, ocap_boundaries}` | `require`, `constrain` |
| 4 | `spec/require/writing-quality` | `{spec_id}` | `{dimensions_passing, meets_publication_standard}` | `evaluate` |
| 5 | `spec/graph/query` | `{query, depth}` | `{nodes[], edges[], paths[]}` | `recognize`, `match` |
| 6 | `spec/graph/coherence` | `{collection_id}` | `{coherence_score, violations[], suggestions[]}` | `ground` |

---

## 4. hLexicon Extension — Spec Terms

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

---

## 6. MDS Cycle

```
MDS_cycle(S, D) :=
  let G = capture(D)              // Extract goals from domain D
  let C = decompose(G)            // Break into sub-goals with dependencies
  let B = bind(G, constraints)    // Attach OCAP boundaries per goal
  validate writing_quality(S)     // Gate: spec must be readable
  validate coherence(S)           // Gate: collection must be coherent
  human_or_curator decides:       // External to spec server
    Merge | Revise | Defer | Discard
```

The spec server handles capture → decompose → bind → quality → coherence. Curation is external.

---

## 7. Template Manifests

Each category has a minimal YAML template. All use `schema_version: "0.27.0"`.

### 7.1 Domain Spec Template

```yaml
schema_version: "0.27.0"
category: domain
domain_anchor: hkask
bounded_context: "..."

ontology:
  entities:
    - name: Agent
      attributes: [webid, capabilities, persona]
  hlexicon_allocation:
    word_act_terms: [...]
    flow_def_terms: [...]
    know_act_terms: [...]

focusing_assumptions:
  - id: FA-D1
    statement: "..."
    rationale: "..."

completeness_checklist:
  - "Every entity has a named hLexicon term"
  - "Bounded-context map exists"

cross_references:
  - category: composition
    relation: "Entities expose composable verbs"
  - category: lifecycle
    relation: "Entity state persisted across lifecycle"
```

### 7.2 Composition Spec Template

```yaml
schema_version: "0.27.0"
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
schema_version: "0.27.0"
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
schema_version: "0.27.0"
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
schema_version: "0.27.0"
category: curation
domain_anchor: hkask

curation_model:
  decisions: [Merge, Revise, Defer, Discard]
  curator:
    type: Replicant
    authority: "Human-augmented — curator proposes, human decides"

coherence_metric:
  method: "Jaccard similarity of declared vs. registered verbs"
  threshold: 0.7
```

---

## 8. Testing Protocol

### Principles

1. **Spec-anchored:** Every `#[test]` carries a `// REQ:` tag referencing an MDS requirement.
2. **Public seam only:** Tests verify behavior through public interfaces, not implementation.
3. **Tracer bullet:** One RED→GREEN cycle per behavior. No horizontal slicing.
4. **Category coverage:** Each MDS category has at least one integration test.

### Category → Test Strategy

| Category | Test Strategy |
|----------|--------------|
| Domain | Entity definition + hLexicon term validation |
| Composition | Capability composition + interface equivalence verification |
| Trust | OCAP boundary enforcement + threat model audit |
| Lifecycle | Bootstrap + evolution + deprecation + CNS span emission |
| Curation | Coherence scoring + decision rationale documentation |

---

## 9. References

[^w3c-rdf]: W3C. (2014). *RDF 1.1 Concepts and Abstract Syntax*. <https://www.w3.org/TR/rdf11-concepts/>.
[^miller-robust]: Miller, M. S. (2006). *Robust Composition: Towards a Unified Approach to Access Control and Concurrency Control*. Johns Hopkins University.
[^cockburn-hexagonal]: Cockburn, A. (2005). *Hexagonal Architecture*. <https://alistair.cockburn.us/hexagonal-architecture/>.
[^shostack-threat]: Shostack, A. (2014). *Threat Modeling: Designing for Security*. Wiley.
[^ronacher-jinja2]: Ronacher, A. (2026). *Jinja2 Template Designer Reference*. <https://jinja.palletsprojects.com/>.

---

*MDS v0.27.0 — supersedes MDS. Five categories, six tools, one predicate.*
