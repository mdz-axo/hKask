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

### 1.5 Principle Roles in the Contract System

Each principle can serve in one of two roles within any given code contract:

| Role | Definition | Example |
|------|-----------|---------|
| **Goal Principle** | The principle whose user-visible guarantee the contract directly implements. Encoded in the contract ID prefix (`P{N}`). Answers "What does the user get?" | `P9-cns-energy-budget-can-proceed` — user gets a gas check that prevents runaway agents |
| **Constraining Principle** | A principle that shapes how the goal is delivered. Appears as `[P{N}] Constraining:` annotations in the contract body. Answers "What guardrails apply?" | `[P4]` on an energy budget contract — the cap enforces an OCAP boundary |

A single contract has exactly one goal principle (the ID prefix) and 1 to 11 constraining principles (body annotations). The goal principle encodes the explicit user functional expectation that the contract's tests verify. The constraining principles ensure the implementation respects all 11 other architectural constraints while achieving the goal.

**The Magna Carta principles (P1–P4) are the most common constraining principles** — most contracts serve goals from P5–P12 while being constrained by sovereignty, consent, generativity, and boundary requirements.

### 1.6 Goal Principle Anchoring (v0.28.0)

**Structural rule:** One of the 12 principles is designated the **goal principle** for a functional expectation; the other 11 may **constrain** it. The goal principle is the one the user's expectation directly expresses — it answers "What does the user functionally need?" The constraining principles answer "What limits how the goal is achieved?"

**Selection logic:**
- The goal principle is the principle whose user-visible guarantee the contract's tests directly verify.
- When the user expectation is *"I should be able to check whether an agent has enough gas"* → P9 (Homeostatic Self-Regulation) is the goal — the expectation directly expresses self-regulation.
- When the user expectation is *"My agents should operate within my sovereignty boundaries"* → P1 (User Sovereignty) is the goal.
- When the user expectation is *"I should be able to deploy hKask with a single binary"* → P5 (Essentialism) is the goal — the expectation directly expresses minimalism.

**Constraining principle interaction:**
- P4 OCAP boundaries may constrain P3 generative space: "Yes, the system is generative — but only within your capability tokens."
- P2 Affirmative Consent may constrain P6 Space for Replicants: "Yes, bots operate — but only with explicit, scoped consent."
- P9 Homeostatic Self-Regulation may constrain P3 generative expansiveness: "Yes, generate freely — but within your energy budget."

**Principle conflict resolution (implicit):** When constraining principles conflict, the higher-ranked principle dominates per Optimality Theory ranking — Magna Carta principles (P1–P4) outrank operational principles (P5–P7), which outrank regulatory principles (P8–P9), which outrank agent principles (P10–P12). Formalization of this conflict resolution as a decision procedure is deferred to future work (see `FUNCTIONAL_SPECIFICATION.md` §Future Work).

**Traceability:** This rule anchors the chain documented in `FUNCTIONAL_SPECIFICATION.md` §5.0:
```
UserFunctionalExpectation → GoalPrinciple → ConstrainingPrinciple → BehavioralContract → Pre/Post/Invariant
```
The user expectation (the OUGHT from the functional spec) is the structural origin point — not merely "kept in mind" but encoded as the `expect:` field on every contract and verified by the test suite.
