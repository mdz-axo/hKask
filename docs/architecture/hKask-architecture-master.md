---
title: "hKask Architecture Master"
audience: [architects, developers, agents]
last_updated: 2026-06-22
version: "0.31.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Architecture Master

**Purpose:** Index to the authoritative architecture documents and the four essential architectural patterns that constitute hKask's irreducible core.

**Project:** hKask (ℏKask - "A Minimal Viable Container for Agents") v0.30.0
**Binary:** `kask`  
**Crate prefix:** `hkask-`

---

## The Four Essential Patterns

hKask's architecture is governed by **four irreducible patterns** that compose into a single cybernetic whole. Remove any one and the system collapses into a qualitatively different — and non-viable — system. These patterns were identified through systematic pragmatics review (pragmatic-semantics + pragmatic-cybernetics + pragmatic-laziness + essentialist + coding-guidelines) and stress-tested via Socratic interrogation (grill-me).

### Pattern A: The Skills Model — WordAct / FlowDef / KnowAct

**What it is:** A tripartite template type system that governs how hKask composes behavior. Mirrors the structure of human cognition: speech acts (WordAct), procedural memory (FlowDef), and metacognition (KnowAct).

| Type | Format | Governs |
|------|--------|---------|
| **WordAct** | Jinja2 `.j2` | "What to say" — system prompts, persona definitions, performative utterances |
| **FlowDef** | YAML `.yaml` | "What to do" — `select → populate → execute` cascade, choice/escalate/abort/delegate verbs |
| **KnowAct** | Jinja2 `.j2` | "How to think" — pattern recognition, classification, reflection, calibration |

**Key properties:**
- Selection intelligence lives in **Jinja2/LLM**, not Rust code (P3 Generative Space)
- `ManifestExecutor` drives the cascade: render selector → LLM → parse JSON → follow chosen path
- Cascade is recursive — a FlowDef step can contain nested WordAct/KnowAct/FlowDef, bounded by matryoshka limit (7)
- Specifications are FlowDef manifests — not a separate type (unification principle)
- Energy-accounted and OCAP-gated: every execute step goes through `GovernedTool`
- **PDCA convergence**: Skills declare `convergence.threshold` (quality gate) and iterate via `loop` actions until metric ≤ threshold or `max_iterations` exhausted
- **Dual energy budget**: `gas` (compute cycles, caps loop iterations) and `rjoule` (inference energy, caps LLM spend). System constant: 1 rJ = 250,000 gas cycles (`RJOULE_TO_GAS`)
- **Conditional steps**: Steps may declare `condition` expressions (`"var"`, `"NOT var"`, `"a AND b"`, `"a OR b"`) evaluated against context
- **Compound convergence**: Bundle-delegated skills aggregate via `all_converged`, `min`, or `weighted_avg` methods
- **Timeout enforcement**: Per-step `timeout_seconds` enforced via `tokio::time::timeout` on inference calls

**Crates:** `hkask-templates`, `hkask-types` (lexicon, BundleManifest)

**If removed:** System becomes a tool executor with monitoring — can do things but can't compose behavior, select strategies, or render personas. P3 and P8 violated.

#### Skill Artifact Model: Single Source of Truth

The canonical source of truth for every skill is its **registry crate** (`registry/templates/<name>/manifest.yaml` + `*.j2` templates). This is the **primary runtime artifact** — it is what `ManifestExecutor` drives, what `SqliteRegistry` indexes, and what the cascade dispatches at inference time.

The **SKILL.md** file (`.agents/skills/<name>/SKILL.md`) is a **generated companion** — a markdown rendering of the registry's structure and intent, produced for the Zed coding agent during development. It is not a co-equal source of truth.

**Derivation rule:** SKILL.md is derived from `manifest.yaml` + `*.j2` templates, not independently authored. The derivation path is:

```
manifest.yaml + *.j2  ──[skill-translator reverse]──▶  SKILL.md
       ↑                                                    │
       └──────────── source of truth ──────────────────────┘
```

**Consequences:**
- A skill with only a registry crate is **complete** — the cascade can execute it. No SKILL.md is required for runtime correctness.
- A skill with only a SKILL.md is **incomplete** — it cannot execute in the cascade. The registry crate must be created.
- When both exist, the registry is authoritative. Any drift between SKILL.md and registry is a defect in SKILL.md, not in the registry.
- The skill health score no longer deducts for missing SKILL.md. It deducts for missing registry (critical: −0.50) and for content drift between layers (medium: −0.10).

**Motivation:** This decision eliminates the cross-layer consistency maintenance burden. Prior to v0.28.0, SKILL.md was treated as a co-equal artifact, requiring manual synchronization. The dual-source model violated P5 (Essentialism) by duplicating skill semantics across two independently-authored formats. The unified model aligns with P3 (Generative Space): selection intelligence lives in Jinja2/LLM, not in markdown instructions.

#### Template Header Standard

> **Incorporated from:** `docs/architecture/reference/template-header-standard.md`

Every Jinja2 template and YAML manifest carries a header identifying its functional role:

**Jinja2 template header format:**
```
{# Template: {path_from_templates_dir} #}
{# Functional Role: {WordAct|FlowDef|KnowAct} (description) #}
{# Implementation: Jinja2 prompt #}
{# Produces: {output_artifacts} #}
{# {template_title} #}
```

**YAML manifest header format:**
```
# {Manifest Name}
# ℏKask {version} — {description}
# Functional Role: FlowDef (process orchestration)
# Implementation: YAML manifest
# Steps: {step_list}
```

**Functional Role quick reference:**

| Role | Keyword | Test |
|------|---------|------|
| **WordAct** | action-word | Does it produce an output artifact (message, span, triple)? |
| **FlowDef** | flow-definition | Does it define a sequence of steps or stages? |
| **KnowAct** | knowledge-action | Does it produce a decision, judgment, or assessment? |

### Pattern B: The CNS Feedback Loop — Cybernetic Self-Regulation

**What it is:** The autonomic nervous system of hKask — a complete cybernetic system per Beer's Viable System Model (S1–S5). Not passive monitoring; active *regulation*.

```
Sensor (MCP dispatch, CNS spans) → Model (VarietyTracker, ν-event store, EnergyBudget)
    → Comparator (AlgedonicManager, SetPoints, Dampener)
    → Regulator (CurationLoop, CuratorAgent, BackpressureSignal)
    → Actuator (GovernedTool, OCAP dual gate, CircuitBreaker)
    → Sensor (loop closes)
```

**Key properties:**
- **Variety is the core metric.** Ashby's Law: `VarietyTracker` counts distinct states per domain over 60s window. Deficit = expected − observed. Drives all escalation.
- **Energy tracking subsumed rate limiting.** Least action principle as infrastructure: every operation costs gas (action in configuration space). Budget cap = max action per session.
- **Algedonic pathway is unidirectional.** Cybernetics *signals* Curation via alerts; Curation *regulates* Cybernetics through `CuratorDirective::CalibrateThreshold` on a direct `mpsc` channel → `CnsRuntime::calibrate_threshold()`.
- **28 canonical CNS span namespaces.** Every dimension observable: tools (11 MCP subsystems), inference, agent pods, gas, curation, sovereignty, specs, chat, memory, wallet (10 sub-spans), architecture (seam coverage/drift), contracts (proposed/accepted/rejected/violated/coverage), ACP (replicant memory, IDE connection).
- **Good Regulator contract enforced.** CNS variety counter IS the regulator's model. `DefaultSpecCurator` detects spec drift (model-reality divergence).

**Crates:** `hkask-cns`, `hkask-types` (CNS types, SpanNamespace)

**If removed:** System becomes a runaway agent platform — agents act without regulation, resources deplete without backpressure, failures accumulate without detection. P9 violated entirely. P5 loses CNS sensors.

### Pattern C: Agentic AI Mediation — Curator + 7R7

**What it is:** The meta-agent layer that maintains and curates the stack. Embodies the cybernetic separation of observation from decision.

| Component | Role | Authority |
|-----------|------|-----------|
| **7R7 Listener** | Passive observer — polls Matrix rooms, emits CNS spans | **Zero.** Does not classify, escalate, moderate, or judge. |
| **R7.3 Seam Watcher** | Public API contract observer — loads seam inventory, tracks per-crate test coverage as CNS variety dimensions, detects drift, emits algedonic alerts on degradation | **Zero.** Observes and reports. Does not write tests, modify code, or block builds. |
| **CurationLoop** | Pure regulatory — sense/compute/act cycle | **Regulatory.** Compares variety, emits directives. |
| **CuratorAgent** | Persona layer — metacognition, spec curation, human-facing reporting | **Decisional.** Formats directives, pursues goals, escalates to human. |

**Key properties:**
- **Singleton invariant.** Exactly one `CuratorAgent` per system (VSM S4 — Intelligence). Multiple Curators would produce conflicting assessments.
- **Dual-presence in CLI/REPL.** Human replicant + Curator daemon co-present in the interaction loop. User speaks; Curator observes, surfaces CNS alerts, provides memory summaries.
- **Curator never bypasses OCAP.** Can recommend actions, cannot execute without capability tokens. No `sudo`.
- **Metacognitive override mechanism.** `MetacognitionLoop::act_on_throttle()` → `CuratorDirective::CalibrateThreshold` → `mpsc` channel → `CyberneticsLoop` → `CnsRuntime::calibrate_threshold()`. Curator adjusts CNS thresholds; human can override Curator.
- **Spec drift is a cybernetic signal.** `DefaultSpecCurator` detects when specs diverge from implementation → `SpecDriftAlert` → Conant-Ashby violation → revise spec, not suppress alert.
- **7R7 is a dumb pipe by design.** Transport moves messages; agents decide what they mean. Authority resides in agent layer, not transport layer.
- **R7.3 watches the public seam.** `SeamWatcher` loads the machine-readable public seam inventory (embedded JSON at compile time, file path override for development), registers per-crate coverage as CNS variety domains (`seam:{crate_name}`), runs periodic drift checks (default: 30 min), and emits algedonic alerts when coverage degrades. Coverage improvements emit positive `Notify` signals. The watcher is non-fatal — if no inventory is available, seam watching is silently disabled.

**Crates:** `hkask-agents` (curator, curator_agent), `hkask-communication` (listener), `hkask-cns` (seam_watcher), `hkask-mcp-cloud-gateway` (cloud transport adapter), `hkask-cli` (token issuance)

**If removed:** System becomes a headless automaton — runs, monitors itself, but nobody reads the monitors. CNS fires alerts into a void. P12 partially violated.

### Pattern D: Agent Creation with Sovereign Memory

**What it is:** The pod lifecycle — how agents come into existence with their own identity, capabilities, memory, and consent boundaries.

```
Creation (kask pod create) → Populated → Registered → Activated → Deactivated
                              ↓
                        Operating Modes: Chat (H2A) | Server (A2A)
                              ↓
                        Sovereign Memory: per-agent SQLCipher DB
                              ↓
                        Boundaries: OCAP dual gate + Visibility gating + ConsentManager
```

**Key properties:**
- **Deterministic key derivation.** `derive_ocap_secret(webid)` via HKDF-SHA256 from master key. ADR-027: restart-safe, per-agent isolation. No key material stored.
- **Mode mutual exclusion (initial).** Chat OR Server, not both. Safety boundary: prevents context leakage between human dialogue and tool serving (P11).
- **Server mode flow.** 4 gates: `kask login → pod assign → pod mode server → IDE spawns MCP binary → daemon auth → assignment → capability → serve`.
- **Dual memory encoding.** Every tool call → `record_experience()` → daemon `store_experience` → episodic (private) + semantic (public). Every 10 experiences → `generate_narrative()`.
- **No cross-agent memory access.** `EpisodicMemory::query_for_deduped` filters by `perspective == Some(agent_webid)`. Semantic memory is public. P11: right to choose public/private extends to agents.
- **Default is private — sovereignty fails closed.** `Visibility::Private` default. `ConsentManager` requires explicit affirmative consent for visibility transitions.

**Crates:** `hkask-agents` (pod), `hkask-memory`, `hkask-storage`, `hkask-keystore`

**If removed:** System becomes a library, not a platform — all infrastructure for agency exists but no agents to inhabit it. P6, P10, P11, P12 violated.

### Pattern D.1 — AgentPod as Solid Pod Isomorphism

**What it is:** The architectural grounding of the agent pod in the five invariants that define a Solid Pod: (1) per-user WebID-grounded identity, (2) self-contained storage, (3) capability-based access control, (4) interoperable data as linked-data triples, (5) the pod IS the deployment unit.

This isomorphism was the original architectural intent, as evidenced by the deployment model's backup design ("Backup as portable archive. Encrypted SQLCipher file. Export from one server, upload to another"). The migration from centralized `PodManager` to per-pod `PodDeployment` was completed in v0.30.0.

#### The Five Invariants Mapped onto hKask

| # | Solid Invariant | hKask Implementation | Status |
|---|----------------|---------------------|--------|
| 1 | WebID-grounded identity | `AgentPod.webid` + `derive_ocap_secret(webid)` (ADR-027) | ✓ |
| 2 | Self-contained storage (LDP) | `PerPodStorage` with per-pod SQLCipher file at `{data_dir}/agents/{sanitized_name}/pod.db` | ✓ |
| 3 | Capability-based access (WAC/ACP) | `DelegationToken` + `CapabilityChecker` + OCAP dual gate | ✓ |
| 4 | Interoperable linked-data triples | `Triple` struct with entity/attribute/value/confidence/visibility | ✓ |
| 5 | Pod IS the deployment unit | `PodDeployment` owns its storage, CNS, and tools directly. `PodDeployment` includes `pod_kind`, `semantic_index`, and per-pod CNS runtime. `PodManager` deleted. `PodFactory` is stateless. | ✓ |

#### Three-Tier Pod Architecture (v0.30.0)

hKask extends the Solid Pod isomorphism into three pod tiers:

| Tier | `PodKind` | Filename | Owner | Semantic Behavior |
|------|-----------|----------|-------|-------------------|
| **CuratorPod** | `Curator` | `agents/curator/pod.db` | System (singleton) | `SemanticIndex` owner — aggregates Public triples from all pods |
| **TeamPod** | `Team` | `agents/team.{name}/pod.db` | Shared bots | Bots share episodic storage; semantic published to Curator |
| **ReplicantPod** | `Replicant` | `agents/replicant.{name}/pod.db` | Human+replicant pair | Episodic private; semantic published to Curator |

**Startup order:** CuratorPod → TeamPods → ReplicantPods (on demand).
**Data flow:** `store_semantic()` writes locally → CNS event `cns.semantic.published` → `CuratorSync` polling loop opens source pod read-only → inserts Public triples into `SemanticIndex` with cursor tracking.
**Semantic recall:** `PodContext::recall_semantic()` routes through Curator's `SemanticIndex` for merged-lens view when Curator is active; falls back to local storage.
**Full spec:** [`MULTI_POD_ARCHITECTURE.md`](core/MULTI_POD_ARCHITECTURE.md)


#### PodDeployment — The Canonical Type (v0.30.0)

`PodDeployment` is now the canonical pod type. `PodManager` has been deleted.

```rust
pub struct PodDeployment {
    pub pod_id: PodID,
    pub pod: AgentPod,
    pub storage: PerPodStorage,  // Per-pod SQLCipher file at {data_dir}/agents/{sanitized_name}/pod.db
    pub cns: PerPodCnsRuntime,    // Per-pod variety counters at cns.agent_pod.{pod_id}.*
    pub tools: PerPodToolBinding, // Per-pod MCP server bindings
}
```

**PodFactory** is the canonical constructor (1 public method: `deploy`).
**ActivePods** is the runtime registry (lightweight HashMap, no shared storage).
**PodRegistry** is filesystem-based discovery (scans `{data_dir}/agents/{name}/pod.db`).

**Full analysis:** [`SOLID_POD_ISOMORPHISM.md`](core/SOLID_POD_ISOMORPHISM.md) (includes deployment types)

**Crates:** `hkask-agents` (pod, deployment), `hkask-storage`, `hkask-memory`, `hkask-keystore`

**If removed:** The system devolves into a shared multi-tenant service — agents are cache entries, not sovereign deployment units. P6, P11 violated (per-pod boundaries become advisory).


#### Agent Definition YAML (v0.30.0)

Every agent has a self-contained definition at `agents/{sanitized_name}/agent.yaml`.
This file is the canonical source for the agent's identity, charter, capabilities,
public/private directory declarations, and persona constraints.

**Creation paths:**
- **Onboarding (`register_replicant`):** Writes the full YAML during `kask chat` first run.
  Stored as `source_yaml` in the `agent_registry` SQL table so the REPL can load
  `persona_constraints` and `process_manifest` without re-reading from disk.
- **CLI registration (`agent_register`):** Writes YAML from WebID, agent type, and
  capabilities passed at the command line.
- **`ensure_agent_dirs` fallback:** Writes a minimal stub (directory declarations only)
  if no definition exists — prevents the full definition from being overwritten.

**YAML format:**
```yaml
agent:
  name: "Jacques (Zuck)"
  type: replicant
charter:
  description: "A helpful AI assistant"
capabilities:
  - tool:inference:call
  - tool:mcp:invoke
  - registry:episodic_memory:read
  - registry:episodic_memory:write
public_dirs:
  - artifacts
  - library
  - gallery
  - documents
  - adapters
private_dirs:
  - sessions
  - portfolios
```

**Loading order (REPL init, `/agent` command):**
1. Query `agent_registry.source_yaml` → `parse_agent_from_yaml()`
2. Fallback: read `agents/{name}/agent.yaml` from disk
3. If neither succeeds, agent runs without persona constraints or process manifest

This two-source approach ensures backward compatibility with pre-fix agents
(where `source_yaml` was a placeholder string) while maintaining filesystem
as the ground-truth canonical store.

**Relevant crates:** `hkask-services-onboarding` (creation), `hkask-cli` (CLI registration),
`hkask-types::agent_paths` (path resolution), `hkask-agents::yaml_parser` (parsing).


### How They Compose

```mermaid
graph TD
    subgraph Skills["Pattern A: Skills Model"]
        SK["WordAct / FlowDef / KnowAct<br/>select → populate → execute"]
    end

    subgraph CNS["Pattern B: CNS Feedback Loop"]
        CN["Variety → Algedonic → Backpressure\n28 canonical span namespaces"]
    end

    subgraph Curator["Pattern C: Agentic AI Mediation"]
        CU["CuratorAgent + 7R7 + R7.3 Seam Watcher<br/>observe → assess → intervene → escalate"]
    end

    subgraph Agents["Pattern D: Agent Creation + Sovereign Memory"]
        AG["Pod lifecycle + per-agent DB<br/>identity, capabilities, memory, consent"]
    end

    SK -->|"templates drive"| AG
    CN -->|"monitors"| AG
    CN -->|"algedonic signals"| CU
    CU -->|"CalibrateThreshold directive"| CN
    CU -->|"curates"| SK
    AG -->|"produces CNS spans"| CN
    SK -->|"renders Curator persona"| CU

    style Skills fill:#e1f5ff
    style CNS fill:#ffe1e1
    style Curator fill:#f3e1ff
    style Agents fill:#fff3e1
```

**The composition chain:**
1. **Skills drive Agents.** Pods created from FlowDef templates. Personas are WordAct. Cognitive strategies are KnowAct. Templates are the loom; agents are the fabric.
2. **CNS monitors Agents.** Every tool call, inference, memory operation emits CNS span. Variety counter tracks behavioral diversity. Algedonic alerts fire on deficit.
3. **CNS signals Curator.** AlgedonicManager → RuntimeAlert → NuEventStore → CurationLoop reads via cursor → CuratorAgent assesses via metacognition.
4. **Curator regulates CNS.** `CuratorDirective::CalibrateThreshold` on direct `mpsc` channel → `CyberneticsLoop` → `CnsRuntime::calibrate_threshold()`. Brain regulates autonomic nervous system.
5. **Curator curates Skills.** `DefaultSpecCurator` evaluates coherence, detects drift, recommends revisions. Ensures template DNA stays aligned with implemented system.
6. **Agents produce CNS data.** Agency produces observability; observability enables regulation; regulation ensures healthy agency. Virtuous cycle.


## Deployment Model

**Decision (2026-06-17):** hKask deploys as a single cloud server. There is no client binary. Users access hKask through a browser: OAuth sign-in (GitHub/Google), then an xterm.js terminal connected via WebSocket. The server spawns `kask repl` on a PTY and pipes I/O.

### Topology

```
CLOUD SERVER (single binary, all crates compiled)
  Caddy (Docker) - TLS + reverse proxy
  Conduit (Docker) - Matrix homeserver
  hkask-mcp-cloud-gateway - mTLS + DelegationToken transport for remote MCP/ACP clients
  hkask-api - OAuth, WebSocket /terminal, backup endpoints
  hkask-services-runtime - daemon orchestration
  hkask-mcp - MCP server runtime
  hkask-agents - bot/replicant lifecycle
  hkask-cns - cybernetic nervous system
  hkask-wallet + hkask-memory - wallet and memory subsystems
  Per-pod SQLCipher files (`{data_dir}/agents/{sanitized_name}/pod.db`) — one database per agent, three-tier (Curator/Team/Replicant)

Access (all via HTTPS/Caddy):
  Browser (xterm.js) - primary
  SSH (optional) - power users
  Matrix (Element) - chat clients
  mTLS (port 9443) - remote IDE agents and MCP servers
```

### Key Properties

- **Single binary.** All crates compiled. No Cargo features for client/server.
- **Browser-only access.** User visits a URL, signs in, gets a terminal. No install.
- **Per-pod storage.** Each agent owns its own SQLCipher file at `{data_dir}/agents/{sanitized_name}/pod.db`. No shared TripleStore. Data isolation is structural, not row-level.
- **Caddy + Conduit sidecars.** Docker containers. hKask generates config; user runs Docker.
- **Backup as portable archive.** Encrypted SQLCipher file. Export from one server, upload to another. No server-to-server protocol.
- **Wallet cloud-only.** Crypto operations never leave the server.

**Full plan:** `docs/plans/deployment-and-backup.md`

---

## User Roles

**Principle:** Two roles. One difference: what settings you can see.

| Role | Who | Privileges |
|------|-----|------------|
| **Admin** | One or more users. First admin runs `kask init`. | View/modify server config. Invite members. View all sessions. |
| **Member** | Users invited by an admin. | View/modify own settings. Cannot see server config or other users. |

**Design rules:**
- **Multiple admins.** Not a single root. Prevents bus-factor.
- **Invite flow.** `kask invite <email>` sends invitation. Invitee signs in via OAuth, auto-assigned Member role.
- **No role hierarchy beyond Admin/Member.** Third role must survive deletion test.
- **Role stored in `HumanUser.role`** (enum `Admin` | `Member`). Enforced by API middleware.
- **Admin-only endpoints:** `GET /api/v1/admin/config`, `POST /api/v1/admin/invite`, `GET /api/v1/admin/sessions`.

**CNS spans:** `RoleAssigned`, `InviteSent`, `InviteAccepted`.

### Identified Gaps (2026-06-17)

All gaps from 2026-06-15 are now closed. Current open gaps:

| Gap | Severity | Status | Description |
|-----|----------|--------|-------------|
| **Kata documentation narrative** | Low | **Open** | CNS narrative companion for kata coaching has not been commissioned. Decision deferred per Task 9. |
| **Skill ↔ MCP server documentation boundary** | Low | **Open** | Skills live in `.agents/skills/` (Zed agent layer) and `registry/templates/` (hKask runtime layer). MCP servers live in `mcp-servers/`. No unified "capability documentation" showing how a skill, its templates, and its MCP surface compose. Deferred per Task 9. |
| **utoipa annotation completeness** | Medium | **Open** | No `#[utoipa::path]` annotations found in `crates/`. The OpenAPI spec (`docs/generated/openapi.json`, 4454 lines) may be manually maintained. Unannotated endpoints are invisible to auto-generation. Task 6 audits this. |
| **Versioned documentation** | Low | **Open** | No versioning strategy for docs. As codebase evolves (kanban v2, kata refinements, additional MCP servers), documentation will drift again. Deferred per Task 9. |
| **LoRA store security model** | Medium | **Open** | Adapter ownership model (P12) is specified but threat model (adapter tampering, weight poisoning, provenance verification) is not documented. Deferred per Task 9. |
| **User roles undocumented** | Medium | **Resolved (2026-06-17)** | Two-role model (Admin/Member) with invite flow. Documented in User Roles section. |
---

## Document Hierarchy

```
core/magna-carta.md  ←  Foundation (4 inviolable principles)
       ↓
core/PRINCIPLES.md  ←  12 principles (P1-P12), constraint forces, 5 anchors
       ↓
   core/MDS.md      ←  Minimal Domain Specification (5 categories, 12 tools)
       ↓
loop-architecture.md  ←  4-loop decomposition, RateLimiting→EnergyBudget
```

### Canonical Specifications

| Document | Purpose |
|----------|--------|
| [`core/magna-carta.md`](core/magna-carta.md) | User sovereignty charter — catch-and-release, affirmative consent, OCAP verification |
| [`core/PRINCIPLES.md`](core/PRINCIPLES.md) | 12 architecture principles (P1-P12), 5 anchors, anti-patterns |
| [`core/MDS.md`](core/MDS.md) | Minimal Domain Specification — 5 categories, 12 tools, completeness predicate |
| [`core/FUNCTIONAL_SPECIFICATION.md`](core/FUNCTIONAL_SPECIFICATION.md) | Functional specification — 26 domains, ER diagrams, goal-principle contract anchoring, user expectations |
| [`loop-architecture.md`](loop-architecture.md) | 4-loop architecture — RateLimiting→EnergyBudget subsumption, crate↔loop mapping |
| [`core/PRINCIPLES.md`](core/PRINCIPLES.md) | 12 architecture principles (P1-P12) incl. P12 replicant host mandate (surface-host mapping, dual-presence) |
| [`energy-gas-payments-api-keys.md`](energy-gas-payments-api-keys.md) | Energy, Gas, Payments & API Key System — economic layer, rJoules, wallets, key lifecycle |
| [`core/CNS-DOMAIN-SPECIFICATION.md`](core/CNS-DOMAIN-SPECIFICATION.md) | CNS Domain Specification — 6 sub-domains, 44 contracts, P4/P9/P12 governed membranes |

**Curator persona** — The Curator is the canonical system daemon. Direct, technical, concise voice with no preamble, emojis, or conversational filler. Full specification: [`reference/hKask-Curator-persona.md`](reference/hKask-Curator-persona.md).

| [`../plans/deployment-and-backup.md`](../plans/deployment-and-backup.md) | Deployment & Multi-User Plan |

### Supplementary Architecture Patterns

> **See also:** `specs/provider-intelligence.md`, `specs/rjoule-cost-system.md`, `self-healing.md`, `loop-architecture.md`, `energy-gas-payments-api-keys.md` for full specifications.

#### Provider Intelligence

Provider selection is intelligence-driven. Each inference provider (DeepInfra, Together AI, fal.ai, OpenRouter) carries metadata: model availability, pricing per token, latency profile, and reliability score. The inference router uses this intelligence to select the optimal provider for each request. Provider metadata is cached and refreshed periodically. See [`specs/provider-intelligence.md`](specs/provider-intelligence.md) for full specification.

#### rJoule Cost System

Energy consumption is tracked in rJoules — a normalized energy unit decoupled from any specific currency. Each inference operation consumes rJoules proportional to token count, model size, and provider efficiency. The cost system integrates with the triple-entry ledger (`hkask-ledger`) for immutable accounting. See [`specs/rjoule-cost-system.md`](specs/rjoule-cost-system.md) for full specification.

#### Self-Healing Architecture

The system detects and recovers from degraded states through CNS-observed self-healing loops:
- **Detection:** CNS variety counters + algedonic thresholds identify anomalies
- **Diagnosis:** Curator classifies anomaly type (stall, saturation, drift, corruption)
- **Recovery:** Automated restart, cache invalidation, connection reset, or escalation to human
- **Verification:** Post-recovery CNS span confirms restoration. See [`self-healing.md`](self-healing.md) for full specification.

#### 4-Loop Authority Model

Architecture decomposes into four authority loops with RateLimiting subsumed by EnergyBudget:
1. **Communication Loop** — Matrix transport, backpressure, queue depth monitoring
2. **Inference Loop** — Provider dispatch, model selection, rJoule accounting
3. **Curation Loop** — Specification drift detection, memory consolidation, catalog maintenance
4. **Regulation Loop** — CNS homeostatic control, algedonic escalation, variety engineering

Each loop maps to specific crates and CNS spans. See [`loop-architecture.md`](loop-architecture.md) for full decomposition.

#### Energy, Gas, and API Key System

The economic layer governs resource consumption across all surfaces:
- **Gas heuristic:** Estimates token cost before inference dispatch
- **Gas cap:** Hard limit per session, enforced by OCAP-gated tool dispatch
- **API key lifecycle:** Provision, rotation, revocation per provider and per user
- **Payment settlement:** rJoule → fiat conversion, provider billing integration

See [`energy-gas-payments-api-keys.md`](energy-gas-payments-api-keys.md) for full specification.

---

## REPL Architecture

The interactive REPL (`kask chat`) implements four features that govern inference behavior:

### Context Injection

Conversation history is appended as a **suffix** (after the cache breakpoint) so the KV cache prefix — system prompt + template — remains identical across turns. Controlled by `ReplSettings.context_turns` (default 3, 0 = no history).

### Unbounded Tool-Use Loop

The REPL loops tool calls until the model stops requesting them, gated by `ReplSettings.tool_loop_limit` (default 21). Each iteration checks the energy budget via `GovernedTool` before executing. If the limit is hit, the loop breaks and returns the partial response — the system tells the model it can continue by asking.

### Auto-Condense

At 87.5% of the model's context window, old session history is condensed via the condenser domain crate (`hkask-condenser`). The condenser summarizes older turns into a compact form, freeing context space for new messages. Controlled by `ReplSettings.auto_condense` (default on). When off, the user must condense manually.

### Model Awareness

On model switch (`/model`), the REPL fetches metadata from the provider's listing endpoint:
- `context_length` — the model's native context window size (used by auto-condense)
- `supports_thinking` — whether the model supports thinking/reasoning tokens
- `capabilities` — model feature list (vision, tools, etc.)

Populated into `ReplSettings.model_meta` as read-only fields. Unknown until the first model detail fetch succeeds.

### ReplSettings

User-configurable inference parameters exposed via three surfaces:

| Setting | Type | Range | Default | Description |
|---------|------|-------|---------|-------------|
| `tool_loop_limit` | usize | ≥1 | 21 | Max tool-call iterations per turn |
| `context_turns` | usize | ≥0 | 3 | Past turns in context (0 = no history) |
| `temperature` | f32 | 0.0–2.0 | 0.7 | Sampling temperature |
| `top_p` | f32 | 0.0–1.0 | 0.9 | Nucleus sampling |
| `top_k` | u32 | ≥1 | 40 | Top-k filtering |
| `min_p` | f32 | 0.0–1.0 | 0.0 | Min-p threshold (0.0 = disabled) |
| `typical_p` | f32 | 0.0–1.0 | 0.0 | Locally typical sampling (0.0 = disabled) |
| `max_tokens` | u32 | ≥1 | 512 | Max completion tokens per call |
| `seed` | u32 or `off` | — | random | Deterministic seed |
| `gas_heuristic` | u64 | ≥1 | 500 | Per-turn gas reservation |
| `gas_cap` | u64 | ≥1 | 10,000 | Total session energy budget cap |
| `auto_condense` | bool | — | true | Auto-condense at 87.5% of context window |
| `model_meta` | read-only | — | None | Model context_length, thinking, capabilities |

### Magna Carta P3 — Equal Surface Exposure

All ReplSettings fields are equally exposed across:
- **REPL:** `/repl` slash command (show/set individual fields)
- **CLI:** `kask settings show|set|reset` commands
- **API:** `GET /api/settings` and `PUT /api/settings` endpoints

All three surfaces read/write the same `~/.config/hkask/settings.json` file. No settings are hidden, admin-gated, or surface-restricted.

### Voice Interaction (Talk + Listen)

The REPL supports bidirectional voice interaction through the media MCP server (`hkask-mcp-media`):

| Command | Behavior |
|---------|----------|
| `/talk on` | Enable speech output — after each agent response, a speech summarizer condenses the output into 1-3 spoken sentences via LLM, then plays through ffplay |
| `/talk off` | Disable speech output |
| `/talk voice [DESC]` | Set or show the TTS voice profile (calls `voice_design` on media server, maps to ElevenLabs presets) |
| `/listen start [SECONDS]` | Record audio from microphone (default 30s), transcribe with word-level timestamps via `transcribe_bundle`, save as `TranscriptBundle` JSON |
| `/listen stop` | Show info about the last recording |
| `/listen view [FILE]` | Open TUI transcript viewer with word-level highlighting synced to audio playback (Richmond Gold #B79163) |

**Architecture:** `/talk` calls the speech summarizer (inference port) → `generate_speech` (MCP media) → ffplay. `/listen` calls `audio_capture` → `transcribe_bundle` (MCP media). Both use `GovernedTool` for OCAP-gated MCP invocation. The `TranscriptViewer` renders `TranscriptBundle` JSON using ratatui + ffplay subprocess.

### Fusion — Multi-Model Deliberation

Fusion routes prompts through multiple models in parallel, then has a judge model synthesize their responses. Implemented via OpenRouter's Fusion plugin API (not a pre-created fusion group).

**Opt-in:** Fusion is disabled by default. Enable with `HKASK_FUSION_JUDGE` + `HKASK_FUSION_PANEL` env vars, or `/fusion on` in the REPL.

**Default model set:**
| Role | Model |
|------|-------|
| Judge/Fuser | `deepseek-v4-pro` |
| Panel | `Kimi2.7`, `Qwen3.7 Max`, `GLM5.2`, `Minimax3` |

**Routing:** When fusion is active, the `effective_model()` router returns `openrouter/fusion`. The dispatch injects a `plugins` array into the chat completion request with the panel models and judge. Panel models run in parallel with `web_search` and `web_fetch` tools; the judge compares their responses and returns structured analysis.

**Bypass:** Chat uses the user's chosen model directly (`bypass_fusion=true`). Skills and tool invocations route through fusion when active (`bypass_fusion=false`). The condenser, daemon narratives, and summarization always bypass fusion.

**Configuration:**
| Env Var | Purpose |
|---------|---------|
| `HKASK_FUSION_JUDGE` | Judge/fuser model |
| `HKASK_FUSION_PANEL` | Comma-separated panel models (1-8) |
| `HKASK_FUSION_OFF=1` | Disable fusion |

**REPL commands:** `/fusion` (status), `/fusion on`, `/fusion off`.

**Crate:** `hkask-inference` (`config.rs`, `inference_router.rs`, `chat_protocol.rs`).

---

## Service Layer

**Crate:** `hkask-services` — shared business logic for CLI and API surfaces.

**Canonical specification:** [`MDS-agent-service.md`](../specifications/specs/MDS-agent-service.md) — full domain spec, accessor methods, depth test results, and service boundary definitions.

### Summary

`AgentService` is the canonical service layer owning all shared infrastructure. All 28 fields are **private** and exposed through **individual named accessor methods** (replacing the earlier grouped-tuple pattern). `AgentService::build(config)` assembles all shared infrastructure once at startup. Both surfaces compose it and add only presentation-specific fields:

- `ReplState` = `AgentService` + REPL fields (prompt history, input state)
- `ApiState` = `Arc<AgentService>` + HTTP fields (router, OpenAPI spec) + surface-specific stores

**Database pattern:** A single `Arc<Mutex<Connection>>` is shared across all stores — in-memory (tests) or file-backed (production). `ServiceConfig` has three constructors: `from_env()` (production, env vars + keychain), `from_secrets()` (REPL onboarding), and `in_memory()` (tests, synthetic secrets). See [`MDS-agent-service.md`](../specifications/specs/MDS-agent-service.md) §4.2 for the full in-memory database pattern.

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

Domain crates **never** depend on `hkask-services`. MCP servers **never** depend on `hkask-services` for orchestration (P1 Prohibition — out-of-process isolation). Tri-surface MCP servers (those that are direct surfaces for a service) may import `hkask-services` for delegation only — see constraint 1 below.

### Key Constraints

1. **MCP servers should not depend on `hkask-services` for orchestration** — P1 Prohibition (out-of-process isolation). Exceptions: servers that are direct surfaces for a service (CLI/API/MCP tri-surface pattern). `hkask-mcp-replica` is a tri-surface for `ComposeService` + `EmbedService`. `hkask-mcp-spec` is a tri-surface for `SpecService` across all 12 tools. Pure business logic lives in `hkask-storage::spec_ops` (shared kernel). Neither server orchestrates — they delegate.
2. **Domain crates do NOT depend on `hkask-services`** — dependency direction is strictly surface → service → domain.


---

## Kanban Agent Coordination

**Crates:** `hkask-services-kanban` (types + service), `mcp-servers/hkask-mcp-kanban` (MCP surface)

**Tri-surface pattern:** CLI (`kask kanban`), REPL (`/kanban`), MCP (7 tools via `hkask-mcp-kanban`)

### Summary

Kanban provides headless task coordination for agents and replicants. Boards contain columns with WIP limits (Anderson, 2010), tasks flow through state transitions, and assignment requires agent consent (P1). Three skills compose the full workflow:

| Skill | Purpose | Steps | Manifest |
|-------|---------|-------|----------|
| **Kanban Task Decomposition** | Break projects into INVEST-compliant tasks with recomposition strategy | 4 | `registry/manifests/kanban-task-decomposition.yaml` |
| **Kanban Task Delegation** | Spawn sub-replicants with OCAP capability packages | 2 | `registry/manifests/kanban-task-delegation.yaml` |
| **Kanban Task Management** | Monitor, coordinate, verify, de-jam | 6 | `registry/manifests/kanban-task-management.yaml` |

### Key Features

- **WIP limits** per column (Anderson §4: "limit WIP to expose problems")
- **CNS behavioral contracts**: task assignment uses `expect:` + `[P{N}]` with pre/post conditions
- **Kata integration**: coaching, improvement, and starter katas available as task primitives
- **Capability packages**: reusable OCAP delegation bundles stored as YAML
- **Board templates**: `software-project`, `writing-project`, `scientific-research`, `investment-research`
- **De-jamming**: auto-detects and fixes stuck tasks, stale assignments, unverified completions
- **LLM-mediated verification**: two-step prompt → JSON → structured pass/fail
- **Persistence**: boards and tasks stored as RDF triples via `TripleStore` (MDS §2)

### Dependency Direction

```mermaid
graph TD
    CLI["hkask-cli"] --> SVC["hkask-services"]
    MCP["hkask-mcp-kanban"] --> SVC
    SVC --> TYPES["hkask-services-kanban"]
    SVC --> STORAGE["hkask-storage (TripleStore)"]
    SVC -.-> AGENTS["hkask-agents (ActivePods)"]
```

`hkask-mcp-kanban` depends on `hkask-services` — permitted as a tri-surface for KanbanService.

Kanban operations emit observability through `CnsSpan::Tool { subsystem: ToolSubsystem::Kanban }`.

See also: `docs/user-guides/kanban-user-guide.md`

---

## Kata — Cybernetic Capability Development

**Crates:** `hkask-services` (KataEngine), `hkask-cns` (variety counters, algedonic alerts), `hkask-storage` (KataHistoryStore)

**Skills:** `.agents/skills/kata-starter/`, `.agents/skills/kata-improvement/`, `.agents/skills/kata-coaching/`, `.agents/skills/kata/` (bundle)

**Templates:** 23 Jinja2 templates across 4 skill directories, 5 YAML manifests, registered in `registry/templates/bootstrap-registry.yaml`

**MCP surface:** Kanban MCP (`hkask-mcp-kanban`) — kata cycles execute as kanban tasks (tri-surface: CLI, REPL, MCP)

### Summary

Kata implements the Toyota Kata methodology (Rother, 2009) as a cybernetic capability development system. Three independently usable skills compose through a bundle orchestrator, with CNS observing every practice, PDCA iteration, and coaching session. The kanban MCP surface provides task-based execution for kata experiments.

| Skill | Purpose | Steps | Templates | Manifest |
|-------|---------|-------|-----------|----------|
| **kata-starter** | Build foundational scientific thinking habits | 5 | 5 (4 FlowDef, 1 KnowAct) | `starter-kata.yaml` |
| **kata-improvement** | 4-step PDCA scientific pattern for capability gaps | 4 | 5 (1 FlowDef, 4 WordAct) | `improvement-kata.yaml` |
| **kata-coaching** | 5-question dialogue for teaching scientific thinking | 5 | 6 (1 FlowDef, 5 WordAct) | `coaching-kata.yaml` |
| **kata** (bundle) | Full orchestration: routing, habit monitoring, iteration | 7 | 7 (6 KnowAct, 1 WordAct) | `kata-pattern.yaml` |

### Key Features

- **PDCA cycle with before/after metrics:** The `KataEngine` captures `metric_before` from CNS counters, executes the 4-step PDCA pattern, captures `metric_after`, and computes an `ImprovementSignal` (Positive/Negative/Stalled/NotMeasured)
- **Automaticity tracking:** Practice history stored in `data/kata-history.json` + SQLite (`KataHistoryStore`). Automaticity linearly approaches 1.0 over 21 consecutive practice days. 3+ day gaps trigger habit decay alerts
- **CNS variety counters:** `kata.practices.completed`, `kata.automaticity.score`, `kata.habit.formation` — baseline 5/week, +0.05/week, 1 per 21 days
- **Algedonic alerts:** Variety deficits exceeding threshold (default 100) emit `kata.algedonic` warnings
- **OCAP consent gates:** kata-starter (self-consent), kata-improvement (Curator), kata-coaching (Learner) — per P2 Affirmative Consent
- **Memory integration:** Every step produces a `StepExperience` recorded to episodic memory via dual-encoding pipeline
- **Kanban integration:** PDCA experiments map to kanban tasks; coaching 5 questions map to task fields; improvement cycles tracked as task state transitions

### Kata-Kanban-CNS Integration

> **Incorporated from:** `docs/architecture/kata-kanban-integration.md`

#### Coaching Loop

```mermaid
sequenceDiagram
    participant Coach
    participant KataEngine
    participant KanbanService
    participant CNS

    Coach->>KataEngine: Q1: What is the target condition?
    KataEngine->>KanbanService: Read task title, description, criteria
    KanbanService-->>KataEngine: Task { title, description, criteria }
    KataEngine->>CNS: Emit cns.kata.coaching.q1

    Coach->>KataEngine: Q4: What is your next step? What do you expect?
    KataEngine->>KanbanService: Update task specification, transition → InProgress
    KanbanService->>CNS: Emit cns.tool.kanban (task moved)

    Coach->>KataEngine: Q5: How quickly can we go and see?
    KataEngine->>KanbanService: Transition → Review, task_verify()
    KanbanService-->>KataEngine: TaskVerified { passed, evidence }

    alt passed
        KanbanService->>KanbanService: Task → Done
    else failed
        KanbanService->>KanbanService: Task → InProgress (rework)
        KataEngine->>CNS: cns.kata.improv.effectiveness (degradation)
    end
```

#### PDCA → Kanban State Mapping

| PDCA Phase | Kanban Status | CNS Event |
|------------|---------------|-----------|
| **Plan** | `Backlog` | task created |
| **Do** | `InProgress` | task moved |
| **Check** | `Review` | task verified |
| **Act** | `Done` | task completed |

Transitions: `Backlog → InProgress` (Q4), `InProgress → Review` (Q5), `Review → Done` (pass), `Review → InProgress` (fail).

#### CNS Span Trace

```
cns.kata.coaching.q1 → cns.tool.kanban (task created)
cns.kata.coaching.q4 → cns.tool.kanban (Backlog → InProgress)
cns.kata.coaching.q5 → cns.tool.kanban (InProgress → Review)
                   → cns.tool.kanban (TaskVerified)
                   → cns.tool.kanban (Review → Done or InProgress)
cns.kata.improv.effectiveness → variety_feedback → CNS homeostatic loop
```

#### Error Recovery

| Failure | Recovery |
|---------|----------|
| Verification failure | Task re-enters Q4-Q5 loop; after 3 consecutive failures, `kanban unjam` escalates |
| Task stall (>24h) | `kanban unjam` scans, CNS variety-deficit alert fires |
| Improv degradation | Switch improv mode, reduce scope, or return to Starter Kata drills |
| CNS span loss | Buffered in `CyberneticsLoop::process_inbox()`, retried, buffer overflow drops oldest |

---

## LoRA Adapter Lifecycle & Inference Composition

**Crates:** `hkask-adapter` (lifecycle, routing, store), `hkask-types` (CNS spans), `hkask-services` (orchestration)

**MCP surface:** Training via `hkask-mcp-training` (17 tools, 5 providers)

**Status:** Active — 48 tests, 45 public functions (17 exposed in lib.rs)

### Summary

`hkask-adapter` manages the full lifecycle of trained LoRA adapters — from training provenance through cloud deployment to cost-tracked inference and teardown. Every operation is OCAP-gated (P4). Every state transition emits a CNS span (P9). Every adapter has an owner WebID (P12).

| Component | Type | Purpose |
|-----------|------|---------|
| **Expertise** | Domain type | Semantic grounding: what the adapter was trained on (MdsDomain, TrainingProvenance) |
| **TrainedLoRAAdapter** | Domain type | Content-addressed, owner-scoped adapter with 12 fields (id, name, owner WebID, base_model, source, expertise, status, etc.) |
| **AdapterStore** | Persistence | SQLite CRUD via `define_store!` — store, retrieve, list, delete adapters |
| **AdapterRouter** | Composition | Composes adapter + base model + provider via `AdapterPort` trait (6 OCAP-gated methods) |
| **EndpointLifecycle** | Lifecycle | 5-phase state machine: Provisioning → Ready → Active → Draining → Terminated |
| **EndpointGuard** | Teardown | RAII guard ensuring resource cleanup (P5 — no leaked endpoints) |
| **CostModel** | Pricing | Per-provider transparent pricing (P2 affirmative consent) |

### Adapter Lifecycle State Machine

```mermaid
stateDiagram-v2
    [*] --> Provisioning: create_endpoint()
    Provisioning --> Ready: provider confirms deployment
    Ready --> Active: start_inference()
    Active --> Draining: drain_endpoint()
    Draining --> Terminated: all requests complete
    Active --> Terminated: force_terminate()
    Provisioning --> Terminated: deployment failed
    Terminated --> [*]: EndpointGuard drops

    note right of Active
        Cost accrual (P9)
        Budget enforcement
        CNS: Inference
        CNS: Gas
    end note

    note right of Draining
        No new requests accepted
        In-flight requests complete
        CNS: Inference
    end note
```

### Key Features

- **Content-addressed storage:** Adapters identified by content hash + owner WebID — no anonymous artifacts (P12)
- **Provider abstraction:** `AdapterSource` enum supports HuggingFace repos (extensible); providers: Together AI (real HTTP upload + inference), Runpod (vLLM skeleton), Baseten (vLLM skeleton)
- **Transparent pricing:** `CostModel` per provider — user sees cost before deployment (P2)
- **Budget enforcement:** `EndpointLifecycle` checks cost accrual against budget; `EndpointCostBudgetWarning` CNS span on threshold breach (P9)
- **OCAP-gated composition:** `AdapterPort` trait exposes 6 methods, each requiring a capability token (P4)
- **RAII teardown:** `EndpointGuard` ensures endpoints are terminated even on panic (P5)

### Dependency Direction

```mermaid
graph TD
    TRAIN["hkask-mcp-training"] --> ADAPTER["hkask-adapter"]
    ADAPTER --> TYPES["hkask-types (CNS spans, WebID)"]
    ADAPTER --> STORAGE["hkask-storage (define_store!)"]
    ADAPTER --> INFERENCE["hkask-inference (provider routing)"]
```

Adapter and endpoint operations emit observability through `CnsSpan::Tool { subsystem: ToolSubsystem::Training }`, `CnsSpan::Inference`, and `CnsSpan::Gas`.

See also: `docs/user-guides/lora-adapter-store-guide.md`, `docs/guides/lora-training-guide.md`

---

## Daemon & Replicant Server Mode

**Crates:** `hkask-mcp` (daemon transport), `hkask-services` (daemon handler), `hkask-agents` (AgentMode)

### Summary

Replicants can operate in **server mode**, presenting as MCP servers to IDEs (Zed, VSCode) and other hKask agents. The daemon — a Unix domain socket at `~/.config/hkask/daemon.sock` — mediates authentication, role assignment, capability verification, and dual memory encoding between out-of-process MCP binaries and the in-process agent stack.

### Architecture

```mermaid
graph TD
    subgraph "hKask System (Background Service)"
        AS["AgentService::build()"]
        AP["ActivePods"]
        US["UserStore"]
        IP["InferencePort"]
        DH["ServiceDaemonHandler"]
        DL["DaemonListener<br/>(~/.config/hkask/daemon.sock)"]
        AS --> AP
        AS --> US
        AS --> IP
        AS --> DH
        DH --> DL
    end

    subgraph "MCP Binary (Out-of-Process)"
        BIN["hkask-mcp-research"]
        BIN -->|"1. auth_query"| DL
        BIN -->|"2. assignment_query"| DL
        BIN -->|"3. capability_query"| DL
        BIN -->|"4. store_experience"| DL
    end

    subgraph "Callers"
        IDE["Zed IDE"]
        HK["hKask Agent"]
    end

    IDE -->|"stdio MCP"| BIN
    HK -->|"GovernedTool"| BIN

    DH -->|"check_auth"| US
    DH -->|"check_assignment"| PM
    DH -->|"check_capability"| PM
    DH -->|"store_experience → dual encoding"| PM
    DH -->|"every 10: generate_narrative"| IP
```

### Startup Flow

1. `kask login <replicant>` — authenticate (creates session in UserStore)
2. `kask pod assign <replicant> <role>` — assign MCP role (P4 Gate 2: sovereignty/consent)
3. `kask pod mode <replicant> server -r <role>` — enter server mode (P4 Gate 1: OCAP)
4. IDE spawns MCP binary with `HKASK_REPLICANT=<replicant>`
5. Binary connects to daemon → auth → assignment → capability → serve

### Memory Flow

- Tool calls → `record_experience()` (fire-and-forget from MCP binary)
- Daemon `store_experience` → dual encoding: episodic (first-person, private) + semantic (third-person, public)
- Every 10 experiences → `generate_narrative()` → inference analyzes session log → stores observations as episodic "narrative"/"thought"
- Existing consolidation pipeline extracts semantic knowledge from both streams

### Agent Modes

| Mode | Behavior | Mutual Exclusion |
|------|----------|-----------------|
| **Chat** | Conversational loop, calls tools via GovernedTool | Cannot coexist with Server (initially) |
| **Server** | Presents as MCP server(s), handles incoming tool calls, records episodic memories | Cannot coexist with Chat (initially) |

Concurrent chat+server mode planned for future release (3-6 months).

### Key Constraints

1. **P4 Dual Gate:** Every MCP server startup requires both capability verification (OCAP token) and assignment verification (sovereignty/consent).
2. **P2 Affirmative Consent:** Passphrase entry via `kask login` creates session. Daemon checks session existence — no passphrase stored with MCP binary.
3. **Out-of-process isolation:** MCP binaries communicate with hKask only through the daemon socket. No direct access to ActivePods, memory, or inference.
4. **Mode mutual exclusion (initial):** An agent can be in Chat mode OR Server mode, not both.

---

## ACP Replicant — IDE Agent Presence

**Crate:** `hkask-acp`, **Protocol:** [Agent Client Protocol](https://agentclientprotocol.com) (ACP)

### Summary

hKask agents can present themselves in any ACP-compatible IDE (Zed, VS Code with extensions, JetBrains) via the `hkask-acp` replicant. ACP is an open standard (agentclientprotocol.com) for bidirectional agent↔editor communication — distinct from hKask's internal A2A (Agent-to-Agent) protocol used for inter-agent template dispatch.

The ACP replicant runs as a subprocess spawned by the IDE, communicating via JSON-RPC 2.0 over stdio. It connects to the same daemon socket as MCP servers (`~/.config/hkask/daemon.sock`) for authentication, capability verification, and memory encoding. Inference is routed through hKask's centralized `InferenceRouter`.

### Architecture

```mermaid
graph TD
    subgraph "IDE (Zed, VS Code, etc.)"
        USER["User"]
        ACP_CLIENT["ACP Client"]
        USER --> ACP_CLIENT
    end

    subgraph "hkask-acp (Subprocess)"
        AGENT["HkaskAcpAgent"]
        TRANSPORT["StdioTransport<br/>(JSON-RPC 2.0)"]
        INF["InferenceRouter"]
        AGENT --> TRANSPORT
        AGENT --> INF
    end

    subgraph "hKask System"
        DL["DaemonListener<br/>(daemon.sock)"]
        MEM["MemoryStore<br/>(episodic)"]
    end

    ACP_CLIENT -->|"stdio JSON-RPC"| TRANSPORT
    AGENT -->|"auth + capability + store_experience"| DL
    DL --> MEM
```

### ACP Protocol vs MCP vs A2A

| Protocol | Direction | Purpose | Implementation |
|----------|-----------|---------|---------------|
| **ACP** (Agent Client Protocol) | Bidirectional IDE ↔ Agent | Streaming agent presence in editor: session lifecycle, content streaming, tool progress, permission requests, plan communication | `hkask-acp` (JSON-RPC 2.0 over stdio) |
| **MCP** (Model Context Protocol) | IDE → Server | Tool invocation: request/response tool calls | `hkask-mcp-*` (10 servers) |
| **A2A** (Agent-to-Agent) | Agent ↔ Agent | Inter-agent template dispatch, memory artifact routing, capability delegation | `hkask-agents::a2a` (A2ARuntime) |

### Prompt Turn Lifecycle

```text
initialize → session/new → session/prompt → [streaming loop] → stop_reason
                                              │
                                              ├─ agent_message_chunk
                                              ├─ tool_call (pending)
                                              ├─ tool_call_update (in_progress)
                                              ├─ tool_call_update (completed)
                                              └─ usage_update
```

The replicant streams inference output as `session/update` notifications while the prompt is processing. The final response carries a structured `StopReason` (`end_turn`, `max_tokens`, `cancelled`).

### How It Reuses Existing Infrastructure

| Capability | Reused Component |
|-----------|-----------------|
| Identity | `WebID` (same identity across REPL, ACP, and MCP surfaces) |
| Authentication | `DaemonClient::auth_query()` (P4 Gate 1) |
| Capability tokens | `verify_startup_gates()` → `A2ARuntime` (P4 Gate 2/3) |
| Memory | `DaemonClient::store_experience()` → dual episodic/semantic encoding |
| Inference | `InferenceRouter` (same provider dispatch as REPL) |
| Observability | CNS spans: `cns.acp.bridge.latency`, `cns.acp.replicant.memory_size`, `cns.acp.ide.connection_state` |
| Accountability | Every memory triple carries the replicant's `WebID` as `owner` (P12) |

### Key Constraints

1. **P2 Affirmative Consent:** The ACP replicant never initiates without user invocation. Sessions are created by the IDE (user action), not by the replicant.
2. **1:1 session isolation:** One ACP replicant process = one IDE connection. Concurrent multi-IDE support is gated on usage data (P7 — Evolutionary Architecture).
3. **Surface-independent identity:** An agent registered in hKask uses the same `WebID`, capability tokens, and memory store whether it's accessed via REPL (`kask chat`), ACP (IDE), or MCP (tools).

---

## Deployment

**Authoritative model:** See Deployment Model section above. hKask deploys as a single cloud server. There is no client binary. Users access hKask through a browser terminal (xterm.js + WebSocket). SSH is optional for power users.

### Cloud Server Deployment

The production deployment is a headless Ubuntu cloud server:

1. **Single binary.** All crates compiled. No Cargo features for client/server.
2. **Browser-first interaction.** Primary interface is xterm.js terminal via browser. Secondary: SSH (`kask repl`). MCP servers (for IDE integration) connect via the REST API or SSH-tunneled socket.
3. **No local GPU inference.** The inference router (`hkask-inference`) routes all requests to cloud providers (DeepInfra, Together AI, fal.ai, OpenRouter).
4. **API keys in OS keychain.** Provider API keys are stored in the OS keychain (Linux Secret Service or flat-file fallback), not in environment variables or plaintext files.
5. **Encrypted database at rest.** All persistent state uses SQLCipher with a passphrase-derived key.
6. **Multi-tenant.** Multiple users per server. Data scoped by `owner_webid`. OAuth (GitHub/Google) sign-in.
7. **Caddy + Conduit sidecars.** Docker containers for TLS termination and Matrix homeserver.

### Provider Configuration

API keys resolve through a 2-tier chain at startup:

| Tier | Source | Security | Persistence |
|------|--------|----------|-------------|
| 1 | OS Keychain | Encrypted at rest by OS | Survives reboot |
| 2 | Environment variable | Plaintext in process memory only | Session-only |

Provider selection via `HKASK_DEFAULT_PROVIDER`:

| Value | Provider | Use Case |
|-------|----------|----------|
| `DI` | DeepInfra | Primary cloud provider |
| `TG` | Together AI | Cloud inference + fine-tuning |
| `FA` | fal.ai | Specialized vision/OCR/media models |
| `OR` | OpenRouter | Multi-provider unified API (chat, embeddings, vision) |

### Setup Flow

```bash
# One-time setup on cloud server
cp .env.example .env
kask keystore load --path .env --shred
kask matrix deploy-sidecar --domain my-server.example.com
cd ~/.config/hkask/sidecar && docker compose up -d
kask init --profile server
```

### Security Properties

| Property | Mechanism |
|----------|-----------|
| No plaintext secrets on disk | Keys live in OS keychain; source file shredded after load |
| No secrets in environment | `InferenceConfig` reads from keychain at startup |
| Affirmative consent before deletion | `--shred` requires explicit confirmation |
| Graceful degradation | Missing keys → backend unavailable (logged), not crash |
| Multi-user isolation | All data scoped by `owner_webid`; OAuth identity verification |

## Deep-Module Audit — Public Surface Justifications

> **Incorporated from:** `docs/architecture/PUBLIC_SURFACE_JUSTIFICATIONS.md`

**Threshold:** ≤7 public items per crate (Ousterhout deep-module discipline, P5). Exceptions must pass the deletion test with documented rationale.

| Crate | Pub Items | Key Concerns | Justification |
|-------|-----------|-------------|---------------|
| `hkask-services` | 66 | 28 private `AgentService` fields, domain submodules | Strangler fig consolidation target. Each submodule ≤7 functions individually. |
| `hkask-services-backup` | 12 | BackupService, GitCASPort, encryption config | Extracted for parallel compilation. Each concern independently testable. |
| `hkask-types` | 50 | CNS span registry (28 variants), WebID, RDF types | Canonical type crate. CNS spans alone justify the surface — each span defines vocabulary. |
| `hkask-test-harness` | 42 | Contract verification, proptest strategies | Testing infrastructure. Each strategy is test-only. |
| `hkask-storage` | 39 | `define_store!` macro, TripleStore, vector store | Persistence orchestration. Each store follows same deep pattern. |
| `hkask-agents` | 26 | ActivePods, AgentRegistry, capability delegation | Multi-concern crate. Each concern independently testable. |
| `hkask-cns` | 25 | CyberneticsLoop, VarietyTracker, AlgedonicManager | Regulatory surface. Each component is a distinct feedback loop. |
| `hkask-improv` | 19 | 5 improv modes, kata improv, ensemble coordination | Each mode is a distinct interaction grammar. |
| `hkask-templates` | 22 | Jinja2 rendering, registry, template types | Template engine. Registry, rendering, classification are distinct concerns. |
| `hkask-wallet` | 22 | WalletManager, rJoule, multi-chain bridges | Domain boundary. Keys, balances, deposits are distinct operations. |
| `hkask-inference` | 18 | InferenceRouter, provider backends, budget tracking | Provider abstraction. Each backend scales with provider support. |
| `hkask-adapter` | 17 | Expertise, AdapterStore, AdapterRouter, EndpointLifecycle | Multi-concern spanning types, persistence, lifecycle, routing. |
| `hkask-mcp` | 17 | MCP gateway, capability verification, transport | Protocol surface. Gateway, transport, governance are distinct layers. |
| `hkask-api` | 16 | HTTP router, OpenAPI, endpoint handlers | API surface. Each endpoint group is a resource. |
| `hkask-memory` | 14 | Episodic/semantic memory, narrative generation | Memory subsystem. Each memory type is distinct. |
| `hkask-keystore` | 11 | Argon2id, OS keychain, SQLCipher | Security crate. Derivation, storage, encryption are distinct concerns. |

**Deletion test:** Every crate above passes — delete it and its complexity reappears duplicated. Public surface reflects breadth of domain concerns, not shallow design.

## API Documentation (utoipa)

> **Incorporated from:** `docs/architecture/reference/utoipa-implementation.md`

API documentation is auto-generated at build time from type annotations via utoipa. Dependencies: `utoipa 5.5` (with `axum_extras`, `uuid`, `chrono` features), `utoipa-axum 0.2`. All request/response types derive `ToSchema`; endpoints are registered via `OpenApiRouter::new().route()` with utoipa-axum auto-discovery from handler signatures and `ToSchema` derives. Generated artifacts: `docs/generated/openapi.json` (OpenAPI 3.1), `docs/generated/cli-reference.md`. CLI: `kask docs openapi`, `kask docs cli`, `kask docs all`.

**60 registered endpoints** across templates, bots, pods, MCP, CNS, chat, models, curator, ACP, bundles, specs, episodic, sovereignty, consolidation, git, goals, settings, wallet. All endpoints are registered via `OpenApiRouter::new().route()` and appear in the generated spec via utoipa-axum auto-discovery. MCP tools are discovered dynamically at runtime and are not part of the OpenAPI spec.

## Reference Artifacts

Detailed lookup tables and diagrams in `reference/`:

| Artifact | Purpose |
|----------|---------|

| [`reference/hKask-Curator-persona.md`](reference/hKask-Curator-persona.md) | Curator persona specification |


---

## Decision Records

| ADR | Topic |
|-----|-------|
| [`ADRs/ADR-031-consolidation-authorization.md`](ADRs/ADR-031-consolidation-authorization.md) | Consolidation authorization via master passphrase derivation |
| [`ADRs/ADR-035-replicant-server-mode.md`](ADRs/ADR-035-replicant-server-mode.md) | Replicant server mode — AgentMode (Chat/Server), daemon socket transport, dual memory encoding, narrative generation |

**Archived (2026-06-17):** ADR-030, ADR-032–034, ADR-036–038 (7 Draft ADRs, never adopted). **Archived (retroactive, 2026-06-15):** ADR-024–027. Recoverable via git history.

---

## Specifications

| Document | Purpose |
|----------|---------|
| [`../specifications/specs/REQUIREMENTS.md`](../specifications/specs/REQUIREMENTS.md) | 22 implemented + 5 deferred goal specs |


---

*Verification commands:* `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`. See [`MDS.md`](core/MDS.md) §9.3 for the verification gate.

---

## Document Structure

```
docs/architecture/
├── hKask-architecture-master.md                # THIS FILE (index)
├── loop-architecture.md                        # 4-loop authority model
├── energy-gas-payments-api-keys.md             # Energy, gas, payments
├── self-healing.md                             # Self-healing patterns
├── matrix-integration-architecture.md          # Matrix transport, Conduit
├── specs/
│   ├── hkask-ledger.md                         # Specification (triple-entry ledger)
│   ├── provider-intelligence.md                # Specification (provider intelligence)
│   └── rjoule-cost-system.md                   # Specification (rJoule cost system)
├── core/
│   ├── magna-carta.md                          # Foundation (4 principles)
│   ├── PRINCIPLES.md                           # P1-P12 incl. P12 mandate
│   ├── MDS.md                                  # 5 categories, 12 tools
│   ├── TESTING_DISCIPLINE.md                   # Testing + QA operations
│   ├── CNS-DOMAIN-SPECIFICATION.md             # CNS 6 sub-domains
│   ├── FUNCTIONAL_SPECIFICATION.md             # AgentService functional spec
│   ├── SOLID_POD_ISOMORPHISM.md                # Pod drift analysis
│   └── MULTI_POD_ARCHITECTURE.md               # 3-tier pod structure
├── ADRs/
│   ├── _TEMPLATE.md                            # ADR template
│   ├── ADR-031-consolidation-authorization.md  # Active
│   └── ADR-035-replicant-server-mode.md        # Active
└── reference/
    └── hKask-Curator-persona.md                # Persona spec
```

**Total:** 20 architecture documents (8 core + 2 root + 3 specs + 2 ADRs + 1 reference + 4 framework).

**Archived (2026-06-22):** QA_PLAN (merged into TESTING_DISCIPLINE), P12 mandate (merged into PRINCIPLES), OPEN_QUESTIONS_POD (merged into OPEN_QUESTIONS).

---

*ℏKask - A Minimal Viable Container for Agents — v0.30.0*
