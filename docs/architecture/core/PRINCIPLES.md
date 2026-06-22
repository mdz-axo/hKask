---
title: "hKask Architecture Principles"
audience: [architects, developers, agents]
last_updated: 2026-06-18
version: "0.30.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Architecture Principles

**Purpose:** Twelve principles governing hKask architecture, grounded in the Principle of Least Action (§0). The first four principles are the Magna Carta principles; all remaining principles flow from them. In the contract system (`FUNCTIONAL_SPECIFICATION.md` §5), each principle can serve as a **goal principle** (driving the explicit user functional expectation of a contract) or a **constraining principle** (shaping how the goal is delivered without overriding it).

**Related:** [`AGENTS.md`](../../../AGENTS.md), [`hKask-architecture-master.md`](../hKask-architecture-master.md), [`FUNCTIONAL_SPECIFICATION.md`](FUNCTIONAL_SPECIFICATION.md), [`TESTING_DISCIPLINE.md`](TESTING_DISCIPLINE.md)

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

#### P4 — Clear Boundaries (OCAP)
P1–P3 are enforced through explicit capability boundaries. No ambient authority and no admin bypass.

**P4.1 — Pod Boundary as OCAP Enforcement Perimeter (v0.29.0):** The pod boundary IS the OCAP enforcement perimeter. Each pod deploys with its own `DelegationToken`, its own `CapabilityChecker`, and its own MCP server bindings. Tool dispatch cannot cross pod boundaries structurally — a pod has no handle to another pod's MCP servers. `PerPodToolBinding` makes cross-pod dispatch an invalid state.

---

### 1.2 Operational Principles (How We Build)

#### P5 — Essentialism & Minimalism
Remove before adding. Every module must earn existence by reducing total system action.

**P5.1 — Single Source of Truth for Skills:** Every skill has exactly one canonical source: its registry crate (`manifest.yaml` + `*.j2` templates). The SKILL.md file is a generated companion for development tooling, derived from the registry — not independently authored. Maintaining parallel representations of the same skill semantics across two formats is a P5 violation. When registry and SKILL.md disagree, the registry is authoritative.

#### P6 — Space for Replicants & Bots
hKask exists as a generative container for bot and replicant agency under sovereignty and capability constraints.

**P6.1 — Per-Pod Deployment Model (v0.29.0):** Each human+replicant pair inhabits its own pod. The pod IS the deployment unit — not a cache entry in a shared manager. A pod owns its SQLCipher file (`{data_dir}/pods/{pod_id}.db`), its CNS runtime (per-pod variety counters), and its MCP server bindings (no cross-pod dispatch). `PodDeployment` makes shared state structurally impossible. See Pattern D.1 — AgentPod as Solid Pod Isomorphism.

#### P7 — Evolutionary Architecture
Types and seams should emerge from real usage, not speculative abstraction.

---

### 1.3 Regulatory Principles (How We Sustain)

#### P8 — Semantic Grounding
System claims must be grounded in traceable, provenance-aware representations.

#### P9 — Homeostatic Self-Regulation
The system must remain observable and self-correcting through cybernetic feedback loops.

**§9.1 — CNS Span Coverage (v0.30.0)**

CNS (Cybernetic Nervous System) spans are the primary observability primitive. Every subsystem must emit canonical `cns.*` spans for every security-sensitive, resource-sensitive, and correctness-sensitive operation. Essential domains carry typed `CnsSpan` enum variants (P8 — Semantic Grounding); performative spans (CLI, API middleware) use stringly-typed tracing targets.

| Domain | Target | Spans | Status | CnsSpan Variant |
|--------|--------|-------|--------|-----------------|
| Tool dispatch (all MCP servers) | `cns.tool.*` | ~170 | ✅ `ToolSpanGuard` per-tool | `Tool { subsystem }` |
| Inference (4 backends) | `cns.inference` | 18 | ✅ generate/generate_vision | `Inference` |
| Keystore | `cns.keystore` | 25 | ✅ resolve, store, derive, sign | `Keystore` |
| Adapter (LoRA) | `cns.adapter` | 23 | ✅ store/get_by_id/delete + router | `Adapter` |
| Backup | `cns.backup` | 20 | ✅ snapshot/restore/verify/prune | `Backup` |
| Condenser | `cns.condenser` | 3 | ✅ compression ratio + health | `Condenser` |
| Skill lifecycle | `cns.skill` | 5 | ✅ activate/load/discover/publish/validate | `Skill` |
| MCP server infra | `cns.mcp.*` | 47 | ✅ startup gates + daemon flow | *(stringly-typed)* |
| CLI command dispatch | `cns.cli` | 2 | ✅ command_invoked/completed | *(performative)* |
| API middleware | `cns.api` | 2 | ✅ per-request CNS span | *(performative)* |
| Kata coaching | `cns.kata` | 20 | ✅ PDCA cycles, automaticity | `Kata` |
| Agent pod | `cns.agent_pod` | — | ✅ pre-existing | `AgentPod` |
| Wallet | `cns.wallet.*` | — | ✅ pre-existing | `WalletBalance` etc. |
| Memory | `cns.memory.*` | — | ✅ pre-existing | `MemoryEncode` |
| Curation | `cns.curation` | — | ✅ pre-existing | `Curation` |

**§9.2 — Span Emission Pattern**

```rust
// CNS span emission — pre: {precondition}, post: cns.{domain} span emitted
tracing::info!(target: "cns.{domain}", operation = "{verb}", {key} = %{value}, ..., "CNS");
```

- Target: `"cns.{canonical_domain}"` — uses the `cns.*` namespace convention. Essential domains map to `CnsSpan` variants in `hkask-types::cns`; performative spans (CLI, API) use stringly-typed tracing targets.
- Message: Must be `"CNS"` — enables ν-event filtering
- Latency: Use `std::time::Instant`, emit as `latency_ms`
- Authority: Every span carries a `replicant` or `owner` WebID

---

### 1.4 Agent Principles (Nature of Agency)

#### P10 — Bot/Replicant Taxonomy
Bot and replicant roles are distinct and explicit, with clear interaction contracts and responsibilities.

#### P11 — Digital Public/Private Sphere
Agents and users can explicitly control what is private versus shared; visibility is consent-governed.

**P11.1 — SQLCipher File as Private Sphere Boundary (v0.29.0):** The pod's SQLCipher database file IS the private sphere boundary. Each pod owns its own encrypted file at `{data_dir}/pods/{pod_id}.db`. No cross-pod data access is structurally possible — a pod cannot accidentally query another pod's data because it has no connection handle to that file. Backup IS copying the SQLCipher file. This was already the backup model; the storage layer now matches.

#### P12 — Replicant Host Mandate
Every action has an accountable host identity. No anonymous agency.

**P12.1 — Surface-Host Mapping (v0.30.0):**

> **Incorporated from:** `docs/architecture/mandates/P12-replicant-host-mandate.md`

Every interaction with hKask carries a replicant identity. Three interaction surfaces map to three host classes:

| Surface | Host | WebID Source | Storage | Keychain |
|---------|------|-------------|---------|----------|
| **CLI / REPL** | Human replicant + Curator daemon | `kask login <name>` → UserStore session | `~/.config/hkask/agents/<replicant>.db` | OS keychain via `hkask-keystore` |
| **Daemon / System** | Curator daemon | `Curator` — hardcoded master system daemon | `~/.config/hkask/agents/curator.db` | System keychain |
| **API** | 7R7 bots | Bot-managed capability tokens | Per-bot DB within pod | Bot-attested HKDF keys |

**Dual-presence pattern:** The CLI/REPL surface hosts both the user's replicant AND the Curator daemon in a single loop. The user speaks; the Curator observes, surfaces CNS alerts, provides memory summaries, and can be addressed directly via `kask curator chat`. This is not two separate sessions — it is one conversation with two participants. The user's replicant is the sovereign host; the Curator daemon is the system's presence.

---