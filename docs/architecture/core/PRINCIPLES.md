---
title: "hKask Architecture Principles"
audience: [architects, developers, agents]
last_updated: 2026-07-01
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Architecture Principles

**Purpose:** Twelve principles governing hKask architecture, grounded in the Principle of Least Action (§0). The first four principles are the Magna Carta principles; all remaining principles flow from them. In the contract system (`FUNCTIONAL_SPECIFICATION.md` §5), each principle can serve as a **goal principle** (driving the explicit user functional expectation of a contract) or a **constraining principle** (shaping how the goal is delivered without overriding it).

**Related:** [`AGENTS.md`](../../../AGENTS.md), [`hKask-architecture-master.md`](hKask-architecture-master.md), [`FUNCTIONAL_SPECIFICATION.md`](FUNCTIONAL_SPECIFICATION.md), [`TESTING_DISCIPLINE.md`](TESTING_DISCIPLINE.md)

**Cross-reference:** §1.6 Goal Principle Anchoring — links to `FUNCTIONAL_SPECIFICATION.md` §5.0 hierarchy diagram and `TESTING_DISCIPLINE.md` §1.2 `expect:` syntax.

---

## 0. Lazy Grounding: The Principle of Least Action

**hKask is grounded in laziness — the universe's, not ours.**

*Don't just do something, stand there.*

The Principle of Least Action says physical systems evolve through paths that minimize (or make stationary) action. Water, light, orbits, and fields do not "try harder"; they follow the path selected by minimum action.

This is the grounding model for hKask architecture:

1. **Least action is not always the obvious path.** Sometimes the straight line is worse than the cycloid; in architecture, short-term structural work can reduce total long-term complexity.
2. **Stationary action implies robustness.** Good designs tolerate small perturbations without catastrophic behavior.
3. **Global order emerges from local moves.** The system should evolve by disciplined, local, evidence-based changes rather than speculative master-planning.

Everything below is the architectural expression of this lazy-universe grounding.

---

## 1. The Twelve Principles

### 1.1 Magna Carta Principles (Foundational)

#### P1 — User Sovereignty
Users own their data and delegation boundaries. Data categorization, control, and portability are first-class guarantees.

#### P2 — Affirmative Consent
Default is deny. Access requires explicit, scoped, version-aware, and revocable consent.

#### P3 — Generative Space
Within user-defined boundaries, hKask remains maximally generative. No hidden or engineer-only control plane.

**P3.1 — Social Generativity (v0.31.0):** The Generative Space is socially generative — it operates within the social conventions of the jurisdiction where it is used. Criminal or systemically harmful use is not generative; it is destructive to the Generative Space itself. Core content safety controls (prompt injection, role override, secret leakage) are mandatory at every LLM boundary and cannot be disabled. These controls are implemented in `hkask-guard` and aligned with:

- **OWASP Top 10 for LLM Applications** (primary reference): LLM01 (Prompt Injection), LLM02 (Insecure Output Handling), LLM04 (Model DoS), LLM06 (Sensitive Information Disclosure)
- **NIST AI RMF 1.0** (2023): Technical controls for validity, reliability, security, and resiliency
- **ENISA Multilayer Framework** (2024): Security-by-design for AI systems
- **Martin et al. (2025)** arXiv:2603.29878: Few-shot pattern-based detection as primary defense
- **Zaratiana et al. (2026)** arXiv:2605.07982: Schema-conditioned classification for LLM safeguards

They are the floor, not the ceiling — the Generative Space requires a safe container.

#### P4 — Clear Boundaries (OCAP)
P1–P3 are enforced through explicit capability boundaries. No ambient authority and no admin bypass. Per **Miller's Object Capability model** (Miller, 2006): no ambient authority; every capability is an unforgeable reference; attenuation preserves safety.[^miller-ocap]

**P4.1 — Pod Boundary as OCAP Enforcement Perimeter (v0.29.0):** The pod boundary IS the OCAP enforcement perimeter. Each pod deploys with its own `DelegationToken`, its own `CapabilityChecker`, and its own MCP server bindings. Tool dispatch cannot cross pod boundaries structurally — a pod has no handle to another pod's MCP servers. `PerPodToolBinding` makes cross-pod dispatch an invalid state.

---

### 1.2 Operational Principles (How We Build)

#### P2.1 — Shared vs Public Visibility (v0.31.0)
Shared data is **consent-bound** and must pass `require_sovereignty` + `require_capability` gates (P2/P4). Public data is **unrestricted** and requires no consent gate. Semantic memory defaults to **Shared**; only explicitly public artifacts (e.g., template registry) use **Public**.

#### P5 — Essentialism & Minimalism
Remove before adding. Every module must earn existence by reducing total system action.

**P5.1 — Single Source of Truth for Skills:** Every skill has exactly one canonical source: its registry crate (`manifest.yaml` + `*.j2` templates). The SKILL.md file is a generated companion for development tooling, derived from the registry — not independently authored. Maintaining parallel representations of the same skill semantics across two formats is a P5 violation. When registry and SKILL.md disagree, the registry is authoritative.

**P5.2 — 5W1H Ontological Core (v0.31.0):** Essentialism requires an anchor. The 5W1H framework — **Who, What, When, Where, Why, How** — is hKask's drop-dead-simple ontological core. Every artifact, module, representation, and claim in hKask must answer at least one of these six questions. An artifact that answers none is ontological noise and fails the minimalism test.

This is not abstract philosophy — it's an operational filter with teeth:

- **Who** — agent (generic), human user, userpod, role, owner (anchored by P12 authenticated host mandate)
- **What** — entity, artifact, resource, data, input, output, state
- **When** — time, sequence, ordering, duration, schedule, temporal scope
- **Where** — location, pod boundary, namespace, domain, spatial context
- **Why** — goal, purpose, intent, constraint motivation, principle anchoring (anchored by P1–P4 Magna Carta)
- **How** — method, mechanism, procedure, transformation, execution path

The 5W1H core is grounded in Ontology Design Pattern (ODP) methodology as described by Norouzi et al. (2025, arXiv:2509.23776): instead of navigating entire complex ontologies, hKask extracts compact, requirement-driven patterns. The 6 questions are the universal requirements — the minimal set that distinguishes "understood" from "not understood."

**P5.3 — Minimalist Test (the 5W1H gate):** Before any module, type, or abstraction is added, ask: which of the 5W1H does it answer? If the answer is "none," the addition is a P5 violation. If the answer is "it bridges to a domain ontology that answers one," the bridge itself must justify its existence by the same test. Bridges earn their keep by connecting a 5W1H question to domain-specific depth — they are not free passes.

**P5.4 — Dual-Axis Ontological Framework (v0.31.0):** hKask anchors on two complementary ontological axes — no single source of truth, by design.

| Axis | Master Ontology | Question | Domain |
|---|---|---|---|
| **Process (Flow)** | PKO (Procedural Knowledge Ontology) | How did this come to be? What flow is it part of? | Procedures, steps, executions, actions, transformations — the *verb* dimension |
| **State (Entity)** | Dublin Core + BIBO | What is this? What type, who made it, when? | Entities, resources, types, metadata, relationships — the *noun* dimension |

Every artifact in hKask has both a state identity and a process identity — it is simultaneously a noun AND a verb. This is the Planck constant at the architectural level: you cannot reduce one axis to the other. And per Heisenberg, the more precisely you measure state (DC typing), the less you can know about process position (PKO flow), and vice versa. You are always sampling, never arriving at truth. The bridges are sampling instruments, not truth claims.

**Every MCP server uses BOTH axes.** Domain-specific bridges (FIBO, GOLEM, CogAT, ML-Schema, OMC) are layered on top where DC+BIBO's state axis isn't specific enough for a domain. They are NOT alternatives to the dual-axis core — they supplement it.

| MCP Server | Process Axis | State Axis | Domain Bridge |
|---|---|---|---|
| **kata-kanban** | PKO | DC+BIBO | — |
| **docproc** | PKO | DC+BIBO | — |
| **research** | PKO | DC+BIBO | — |
| **skill** | PKO | DC+BIBO | — |
| **companies** | PKO | DC+BIBO | FIBO (financial concepts) |
| **replica** | PKO | DC+BIBO | GOLEM (narrative structure) |
| **memory** | PKO | DC+BIBO | CogAT (cognitive concepts) |
| **training** | PKO | DC+BIBO | ML-Schema (ML experiments) |
| **media** | PKO | DC+BIBO | OMC (media creation) |
| **condenser** | PKO | DC+BIBO | — (DC is the connective tissue for graph saliency) |
| **curator** | PKO | DC+BIBO | — (the curator IS the 5W1H core applied as Socratic inquiry) |
| **communication** | PKO | DC+BIBO | — (deferred) |
| **filesystem** | PKO | DC+BIBO | — |
| **codegraph** | PKO | DC+BIBO | — |
| **scenarios** | PKO | DC+BIBO | — |

**Bridge locations:**
- Process axis vocabulary: `crates/hkask-bridge-pko/` (shared crate)
- State axis vocabulary: `crates/hkask-bridge-dublincore/` (shared crate)
- Domain-specific bridges: server-local modules following the `fibo.rs` pattern

#### P6 — Space for UserPods
hKask exists as a generative container for **human user agency** (each user via their own userpod) and **AI tools** (skills + MCP servers), coordinated by the curator daemon, under sovereignty and capability constraints.

**P6.1 — Per-UserPod Deployment Model (v0.29.0):** Each user inhabits exactly one persistent userpod (1:1; multi-persona removed). The userpod IS the deployment unit — not a cache entry in a shared manager — and persists for the life of the account. A userpod owns its SQLCipher file (`{data_dir}/agents/{sanitized_name}/pod.db`), its Regulation runtime (per-pod variety counters), and its MCP server bindings (no cross-pod dispatch). The userpod makes shared state structurally impossible. See Pattern D.1 — UserPod as Solid Pod Isomorphism.

#### P7 — Evolutionary Architecture
Types and seams should emerge from real usage, not speculative abstraction.

---

### 1.3 Regulatory Principles (How We Sustain)

#### P8 — Semantic Grounding
System claims must be grounded in traceable, provenance-aware representations.

**P8.1 — Ontological Bridging (v0.31.0):** The 5W1H core (P5.2) is the default grounding level. Anchored beneath it are two complementary ontological axes — no single source of truth, by design.

**Dual-axis grounding:** Every artifact carries both a state identity (DC+BIBO — the noun) and a process identity (PKO — the verb). You cannot reduce one axis to the other, and per Heisenberg, the more precisely you sample one, the less you can know about the other. Bridging is always sampling, never arriving at truth. The bridges are sampling instruments calibrated to universal anchors (PKO namespace, DC namespace) but deployed from domain-specific perspectives.

**Every bridge follows the `fibo.rs` pattern:**

1. **Concept URI constants** — `pub const CONCEPT_NAME: OntologyConcept = "namespace:LocalName"`
2. **Field-to-concept mapping functions** — `pub fn internal_field_to_ontology(field: &str) -> Option<OntologyConcept>`
3. **No dependencies** — bridges are pure Rust with zero external crates beyond what the server already uses
4. **No reasoners, no OWL parsing, no graph databases** — bridges are thin vocabulary layers, not ontology engines

**Bridge hierarchy:**
- **Universal anchors:** `crates/hkask-bridge-pko/` (process axis) + `crates/hkask-bridge-dublincore/` (state axis) — shared vocabulary crates providing the canonical concept constants. Every server depends on both.
- **Domain supplements:** Server-local modules (FIBO in companies, GOLEM in replica, CogAT in memory, ML-Schema in training, OMC in media) — layered on top where DC+BIBO's state axis isn't specific enough for a domain. These are supplements, not alternatives.

Bridges use the STAR extraction pattern (seed terms + direct logical entailments, no intermediate hierarchy) from Norouzi et al. (2025). Each bridge module is typically ≤150 lines.

The architectural invariant: **hKask never requires knowledge of a full domain ontology.** All interaction with domain ontologies flows through thin bridges. The dual-axis core (PKO + DC+BIBO) provides the minimum viable ontology for any server; domain bridges are opt-in specificity.

#### P9 — Homeostatic Self-Regulation
The system must remain observable and self-correcting through cybernetic feedback loops.

**§9.1 — Regulation Span Coverage (v0.31.0)**

Regulation (Cybernetic Nervous System) spans are the primary observability primitive. Every subsystem must emit canonical `reg.*` spans for every security-sensitive, resource-sensitive, and correctness-sensitive operation. Essential domains carry typed `RegulationSpan` enum variants (P8 — Semantic Grounding), are registered in `CANONICAL_NAMESPACES`, mapped to a `SpanCategory`, and connected to a cybernetic loop via ν-events. The `reg.*` prefix is reserved for these canonical spans — every `reg.*` tracing target MUST be registered. Performative telemetry (CLI, API middleware, and other observability logs) uses `hkask.*` tracing targets, NOT `reg.*`; those are deliberately NOT registered, NOT categorized, and NOT loop-connected — they are observability logs, not regulated variables. The two are distinguished by registry presence: `SpanNamespace::new` accepts only canonical spans.

| Domain | Target | Spans | Status | RegulationSpan Variant |
|--------|--------|-------|--------|-----------------|
| Tool dispatch (all MCP servers) | `reg.tool.*` | ~170 | ✅ `ToolSpanGuard` per-tool | `Tool { subsystem }` |
| Inference (8 providers: DeepInfra, fal.ai, Together, OpenRouter, KiloCode, Ollama, Cline, RunPod) | `reg.inference` | 53 | ✅ generate/generate_vision — 7 chat backends + RunPod (vision/OCR only) | `Inference` |
| Fusion (multi-model deliberation) | `reg.fusion` | 9 | ✅ orchestrate + per-round/mode events (convergence verdict, swap-revote, algo method) | `Fusion` |
| Keystore | `reg.keystore` | 25 | ✅ resolve, store, derive, sign | `Keystore` |
| Adapter (LoRA) | `reg.adapter` | 23 | ✅ store/get_by_id/delete + router | `Adapter` |
| Backup | `reg.backup` | 22 | ✅ snapshot/restore/verify/prune/delete_blob | `Backup` |
| Condenser | `reg.condenser` | 3 | ✅ compression ratio + health | `Condenser` |
| Skill lifecycle | `reg.skill` | 5 | ✅ activate/load/discover/publish/validate | `Skill` |
| MCP server infra | `reg.mcp.*` | 47 | ✅ startup gates + daemon flow | *(stringly-typed)* |
| CLI command dispatch | `reg.cli` | 2 | ✅ command_invoked/completed | *(performative)* |
| API middleware | `reg.api` | 2 | ✅ per-request Regulation span | *(performative)* |
| Kata coaching | `reg.kata` | 20 | ✅ PDCA cycles, automaticity | `Kata` |
| Agent pod | `reg.agent_pod` | — | ✅ revert, spawn_agent (via PodBackupOps) | `AgentPod` |
| Wallet | `reg.wallet.*` | — | ✅ pre-existing | `WalletBalance` etc. |
| Memory | `reg.memory.*` | — | ✅ pre-existing | `MemoryEncode` |
| Curation | `reg.curation` | — | ✅ pre-existing | `Curation` |
| Deployment sessions | `reg.deploy` | 2 | ✅ session_open/close | `SessionOpen`, `SessionClose` |
| Backup export lifecycle | `reg.deploy` | 3 | ✅ backup_export/auto_export/upload | `BackupExport`, `BackupAutoExport`, `BackupUpload` |

**§9.2 — Span Emission Pattern**

```rust
// Regulation span emission — pre: {precondition}, post: reg.{domain} span emitted
tracing::info!(target: "reg.{domain}", operation = "{verb}", {key} = %{value}, ..., "Regulation");
```

- Target: `"reg.{canonical_domain}"` — uses the `reg.*` namespace convention. Essential domains map to `RegulationSpan` variants in `hkask-types::regulation`; performative spans (CLI, API) use stringly-typed tracing targets.
- Message: Must be `"Regulation"` — enables ν-event filtering
- Latency: Use `std::time::Instant`, emit as `latency_ms`
- Authority: Every span carries a `userpod` or `owner` WebID

---

### 1.4 Agent Principles (Nature of Agency)

#### P10 — User Agency
Users act as agents in the AI world through their userpod. Userpods present in A2A as agents (the generic "agent" concept is preserved); the hKask-specific bot/userpod role taxonomy is removed. User agency is bounded by sovereignty (P1) and capability (P4) — the userpod is the unit of agency, not a separate "userpod" or "bot" role.

#### P11 — Digital Public/Private Sphere
Users, via their userpods, can explicitly control what is private versus shared; visibility is consent-governed. (The generic "agent" concept remains for A2A interop.)

**P11.1 — SQLCipher File as Private Sphere Boundary (v0.29.0):** The pod's SQLCipher database file IS the private sphere boundary. Each pod owns its own encrypted file at `{data_dir}/agents/{sanitized_name}/pod.db`. No cross-pod data access is structurally possible — a pod cannot accidentally query another pod's data because it has no connection handle to that file. Backup IS copying the SQLCipher file. This was already the backup model; the storage layer now matches.

#### P12 — Authenticated Host Mandate
Every action has an accountable host identity. No anonymous agency.

**P12.1 — Surface-Host Mapping (v0.30.0):**

> **Incorporated from:** `docs/architecture/mandates/P12-authenticated-host-mandate.md`

Every interaction with hKask carries a userpod (or curator) host identity. Three interaction surfaces map to three host classes:

| Surface | Host | WebID Source | Storage | Keychain |
|---------|------|-------------|---------|----------|
| **CLI / REPL** | Human user (via userpod) + Curator daemon | `kask login <name>` → UserStore session | `~/.config/hkask/agents/<userpod>.db` | OS keychain via `hkask-keystore` |
| **Daemon / System** | Curator daemon | `Curator` — system daemon | `~/.config/hkask/agents/curator.db` | System keychain |
| **API** | Userpods | Userpod-managed capability tokens | Per-userpod DB | Userpod-attested HKDF keys |

**Dual-presence pattern:** The CLI/REPL surface hosts both the user's userpod AND the Curator daemon in a single loop. The user speaks; the Curator observes, surfaces Regulation alerts, provides memory summaries, and can be addressed directly via `kask curator chat`. This is not two separate sessions — it is one conversation with two participants. The user's userpod is the sovereign host; the Curator daemon is the system's presence.

[^dublin-core]: Dublin Core Metadata Initiative. *DCMI Metadata Terms*. ISO 15836. <https://www.dublincore.org/specifications/dublin-core/dcmi-terms/>.
[^bibo]: D'Arcus, B. & Giasson, F. *Bibliographic Ontology (BIBO)*. <https://bibliontology.com/>.
[^pko]: Carriero, V. A. et al. (2024). "The Procedural Knowledge Ontology (PKO)." ISWC 2024 / PERKS Project. <https://w3id.org/pko>.
[^miller-ocap]: Miller, M. S. (2006). *Robust Composition: Towards a Unified Approach to Access Control and Concurrency Control*. Johns Hopkins University.

---