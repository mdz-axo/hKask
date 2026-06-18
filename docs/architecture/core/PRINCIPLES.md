---
title: "hKask Architecture Principles"
audience: [architects, developers, agents]
last_updated: 2026-06-18
version: "0.28.0"
status: "Active"
domain: "Cross-cutting"
mds_categories: [domain, composition, trust, lifecycle, curation]
---

# hKask Architecture Principles

**Purpose:** Twelve principles governing hKask architecture, grounded in the Principle of Least Action (§0). The first four principles are the Magna Carta principles; all remaining principles flow from them.

**Related:** [`AGENTS.md`](../../../AGENTS.md), [`hKask-architecture-master.md`](../hKask-architecture-master.md)

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

---

### 1.2 Operational Principles (How We Build)

#### P5 — Essentialism & Minimalism
Remove before adding. Every module must earn existence by reducing total system action.

**P5.1 — Single Source of Truth for Skills:** Every skill has exactly one canonical source: its registry crate (`manifest.yaml` + `*.j2` templates). The SKILL.md file is a generated companion for development tooling, derived from the registry — not independently authored. Maintaining parallel representations of the same skill semantics across two formats is a P5 violation. When registry and SKILL.md disagree, the registry is authoritative.

#### P6 — Space for Replicants & Bots
hKask exists as a generative container for bot and replicant agency under sovereignty and capability constraints.

#### P7 — Evolutionary Architecture
Types and seams should emerge from real usage, not speculative abstraction.

---

### 1.3 Regulatory Principles (How We Sustain)

#### P8 — Semantic Grounding
System claims must be grounded in traceable, provenance-aware representations.

#### P9 — Homeostatic Self-Regulation
The system must remain observable and self-correcting through cybernetic feedback loops.

**§9.1 — CNS Span Coverage (v0.28.0)**

CNS (Cybernetic Nervous System) spans are the primary observability primitive. Every subsystem must emit canonical `cns.*` spans for every security-sensitive, resource-sensitive, and correctness-sensitive operation.

| Domain | Target | Spans | Status |
|--------|--------|-------|--------|
| Tool dispatch (all MCP servers) | `cns.tool.*` | ~170 | ✅ `ToolSpanGuard` per-tool |
| Inference (4 backends) | `cns.inference` | 18 | ✅ generate/generate_vision |
| Keystore | `cns.keystore` | 25 | ✅ resolve, store, derive, sign |
| Adapter (LoRA) | `cns.adapter` | 23 | ✅ store/get_by_id/delete + router |
| Backup | `cns.backup` | 20 | ✅ snapshot/restore/verify/prune |
| Condenser | `cns.condenser` | 3 | ✅ compression ratio + health |
| MCP server infra | `cns.mcp.*` | 47 | ✅ startup gates + daemon flow |
| CLI command dispatch | `cns.cli` | 2 | ✅ command_invoked/completed |
| API middleware | `cns.api` | 2 | ✅ per-request CNS span |
| Kata coaching | `cns.kata` | 20 | ✅ pre-existing |
| Agent pod | `cns.agent_pod` | — | ✅ pre-existing |
| Wallet | `cns.wallet.*` | — | ✅ pre-existing |
| Memory | `cns.memory.*` | — | ✅ pre-existing |
| Curation | `cns.curation` | — | ✅ pre-existing |

**§9.2 — Span Emission Pattern**

```rust
// REQ: P9-CNS-NNN pre: {precondition} post: cns.{domain} span emitted
tracing::info!(target: "cns.{domain}", operation = "{verb}", {key} = %{value}, ..., "CNS");
```

- Target: `"cns.{canonical_domain}"` — must match a `CnsSpan` variant in `hkask-types::cns`
- Message: Must be `"CNS"` — enables ν-event filtering
- Latency: Use `std::time::Instant`, emit as `latency_ms`
- Authority: Every span carries a `replicant` or `owner` WebID

---

### 1.4 Agent Principles (Nature of Agency)

#### P10 — Bot/Replicant Taxonomy
Bot and replicant roles are distinct and explicit, with clear interaction contracts and responsibilities.

#### P11 — Digital Public/Private Sphere
Agents and users can explicitly control what is private versus shared; visibility is consent-governed.

#### P12 — Replicant Host Mandate
Every action has an accountable host identity. No anonymous agency.
