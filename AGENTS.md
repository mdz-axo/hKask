# Agent Operating Guide — hKask

**hKask** (ℏKask) — A Minimal Viable Container for Agents | `kask` binary | `hkask-` crate prefix | v0.31.0

---

## Capability Catalog

**39 Skills** (PDCA FlowDef loops with quality threshold + energy budget), **2 Templates** (one-shot, no registry manifest), **1 Bundle** (composition-only, non-PDCA), **1 Legacy** (v0.21.4, pending upgrade). **43 capabilities in all.**

| Type | Invocation | Behavior | Exit |
|------|-----------|----------|------|
| **Template** | `kask run <name>` | One-shot prompt execution | Returns output |
| **Skill** | `kask run <name>` | Iterative PDCA cascade | Returns `converged \| maxed_out \| escalated` |
| **Bundle** | `kask run <name>` | Composition orchestration | Delegates to sub-skills |

A Skill has a FlowDef manifest with `convergence.threshold > 0`, `gas.cap > 0`, and a `loop` action. A Template may be composed INTO a Skill as a step within the PDCA loop. A Bundle composes Skills but is not itself a PDCA loop.

### Guardrails (activate first)

| Name | Type | When to Activate |
|------|------|-----------------|
| **coding-guidelines** | **Skill** | Before writing or reviewing any code. Surfaces assumptions, enforces simplicity, surgical changes, goal-driven execution. |

### Core Development

| Name | Type | When to Activate |
|------|------|-----------------|
| **bug-hunt** | **Skill** | Bug hunting. Run expeditions against target crates to find threats to user-defined quality. |
| **tdd** | **Skill** | Building features or fixing bugs. Vertical tracer-bullet RED→GREEN→REFACTOR. |
| **diagnose** | **Skill** | Debugging hard bugs or performance regressions. Reproduce, anchor, hypothesise, instrument, fix, regression-test, verify. |
| **deep-module** | **Skill** | Module design discipline. Apply the deletion test, enforce depth, interface minimalism (≤7 public functions). |
| **refactor-service-layer** | **Skill** | Extracting duplicated business logic from CLI/API/MCP surfaces into `hkask-services`. |
| **improve-codebase-architecture** | **Skill** | Finding deepening opportunities. Walk codebase for shallow modules, tight coupling, untested seams. |
| **strangler-fig** | **Skill** | Incremental architectural migration. Introduce new alongside old, migrate one domain at a time. |
| **idiomatic-rust** | **Skill** | Idiomatic Rust design: type-driven design, ownership as architecture, fearless refactoring. |

### Reasoning & Analysis

| Name | Type | When to Activate |
|------|------|-----------------|
| **pragmatic-semantics** | **Skill** | Classifying statements by certainty level, constraint force, and domain ontology anchoring. Distinguish IS from OUGHT, apply tier-specific confidence modifiers (FIBO +0.10, CogAT -0.10). Trace provenance, resolve conflicts via 5-tier OT ranking. |
| **pragmatic-cybernetics** | **Skill** | Cybernetic reasoning: feedback loops, variety engineering, system homeostasis. |
| **pragmatic-laziness** | **Skill** | Procedural composition. Finds the path of least action through meaning-space. |
| **essentialist** | **Skill** | Recursive eliminative interrogation. 3-gate challenge loop (Exist → Surface → Contract). |
| **review** | **Skill** | Self-critique output for contradictions, unsupported claims, logical gaps. |
| **grill-me** | **Skill** | Socratic questioning to stress-test understanding. Probes knowledge gaps. |
| **zoom-out** | **Skill** | Broader context or higher-level perspective on unfamiliar code. |
| **sequential-inquiry** | **Skill** | Dynamic chain-of-thought with branching, revision, hypothesis testing, and automatic deep-dive delegation to hypothesis-framer/mcda/diagnose. The engine decides at runtime whether delegation is needed — no pre-selection. Use for any structured reasoning task. |

### Kata & Coaching

| Name | Type | When to Activate |
|------|------|-----------------|
| **kata** | Bundle | Toyota Kata system — composes starter, improvement, and coaching. [pending v0.31.0 upgrade] |
| **kata-coaching** | **Skill** | The 5-question Coaching Kata dialogue for teaching scientific thinking. |
| **kata-improvement** | **Skill** | The 4-step Improvement Kata: Understand Direction, Grasp Current Condition, Establish Target, PDCA iterate. |
| **kata-starter** | **Skill** | Foundational kata practice routines: Five Questions Drill, PDCA Cycle, Observation Drill. |
| **improv** | **Skill** | Agent interaction grammar — Plussing, Yes And, Yes But, Freestyling, Riffing. |

### Meta & Maintenance

| Name | Type | When to Activate |
|------|------|-----------------|
| **skill-maintenance** | **Skill** | Audit hKask's skill architecture for staleness, coverage gaps, and quality degradation. Also: list, build, install, and discover skills. |
| **skill-logic-audit** | **Skill** | Adversarial audit of .j2 template logic against stated goals. |
| **skill-bundler** | **Skill** | Orchestrate and compose multiple skills into a cohesive bundle. |
| **handoff** | **Skill** | Session handoff protocol. Captures what was done, what remains, key decisions. |

### Specialized

| Name | Type | When to Activate |
|------|------|-----------------|
| **superforecasting** | **Skill** | Calibrated probability forecasting (Tetlock). 8-stage pipeline. |
| **mcda** | **Skill** | Multi-Criteria Decision Analysis. Weight, score, rank alternatives. |
| **scenario-builder** | **Skill** | Schwartz scenario planning. Focal question → STEEP → 2×2 → narratives. |
| **hypothesis-framer** | **Skill** | Research question framing and hypothesis formulation using FINER criteria and PICO process. Use when the user says 'help me frame my research idea', 'is my research question good', 'write a hypothesis', or 'develop study aims'. |
| **adversarial-red-team** | **Skill** | Adversarial robustness testing. ATLAS/GARAK-aligned taxonomy. |
| **goal-analysis** | **Skill** | Goal specification and verification. Extract goals, judge completion. |
| **magna-carta-verifier** | **Skill** | Verify Magna Carta principles (P1–P4) are correctly implemented and enforced. |
| **structured-extraction** | **Skill** | Extract structured data from unstructured text. Entity identification, relation extraction. |
| **caveman** | **Skill** | Multi-mode compression: caveman (stylistic) + dense (entity-preserving, Gao et al. 2024). |
| **self-critique-revision** | **Skill** | Iterative self-critique and revision cycle: draft → critique → revise. |
| **logo-builder** | Template | Pragmatic logo design using LLM-assisted generation. 3-phase pipeline. [no registry manifest] |
| **qa-script-builder** | Template | Design and generate autonomous QA pipeline manifests. 5-phase pipeline. [no registry manifest] |

### Infrastructure

| Crate | Capabilities |
|-------|-------------|
| `hkask-communication` | Matrix transport, agent registry, 7R7 listener, CNS bridge (communication events flow to curation), CAT engagement gate |

---

## Prohibitions

These derive from the Magna Carta (P1–P4) and P12 of [`docs/architecture/core/PRINCIPLES.md`](docs/architecture/core/PRINCIPLES.md). Violations compromise the system's core identity and **must be deleted**.

| # | Prohibition | Principle | Rationale |
|---|-------------|-----------|-----------|
| 1 | No visual UI, dashboards, Grafana, Prometheus, or monitoring stacks | P3 · §5 | Headless system — CLI/MCP/API only. CNS provides all observability programmatically. |
| 2 | No `todo!()`, `unimplemented!()`, `#[deprecated]`, unused traits, stubs, or feature flags | P5 · P3 | Stubs are debt against the Generative Space. Deprecated code earns deletion, not annotation. |
| 3 | No anonymous agency — every action has an authenticated author | P12 · P1 | Every operation carries a replicant host; every triple stores an `owner` WebID. No root, no `sudo`. |
| 4 | No hidden parameters or admin-gated settings | P3 | All generative settings are user-visible through CLI, API, and REPL. No privileged engineer access. |
| 5 | No pass-through abstractions (deep-module discipline) | P5 · P7 | Modules earn existence by the deletion test. Public surface ≤ 7 items; extras justified or removed. |

These are **Prohibitions**, not guidelines. See PRINCIPLES.md §2.1–2.4 for the full principle hierarchy (Prohibition → Guardrail → Guideline) and constraint force classification.

---

## Tooling Policy

hKask is a Rust project. Python is **not** an acceptable project dependency. Ad-hoc Python scripts may be used during exploration, but they must be deleted before the work is considered complete. Any generated artifacts produced by Python scripts (JSON manifests, inventories, etc.) must also be removed. The repository will be periodically scanned for Python files and purged as a coding violation.

Preferred auxiliary tooling:

- Shell (`bash`) for repository-level scripts under `scripts/`.
- Rust binaries or `build.rs` for anything that needs to parse source or Cargo metadata.

---

## Key Docs

| Topic | Location |
|-------|----------|
| Architecture master | `docs/architecture/hKask-architecture-master.md` |
| Principles (P1–P12) | `docs/architecture/core/PRINCIPLES.md` |
| MDS Specification | `docs/architecture/core/MDS.md` |
| Canonical CNS span registry | `crates/hkask-types/src/cns.rs` (`CnsSpan`) |
| Testing Discipline | `docs/architecture/core/TESTING_DISCIPLINE.md` |
| Foundation types | `crates/hkask-types/src/lib.rs` |
| OCAP delegation tokens | `crates/hkask-capability/src/lib.rs` |
| Hexagonal port traits | `crates/hkask-ports/src/lib.rs` |
| Wallet value types | `crates/hkask-wallet-types/src/lib.rs` |
| Curator metacognition | `crates/hkask-agents/src/curator_agent/metacognition.rs` |
| KnowAct templates (wired) | `registry/templates/curator/metacognition-diagnose.j2`, `metacognition-escalate.j2` |
| Template self-healing (design) | `docs/architecture/ADRs/template-self-healing.md` |
| Matrix administration (self-healing registration, CNS spans, design gaps) | `docs/architecture/ADRs/matrix-server-administration.md` |

---

## Constraint Verification

```bash
# Headless + monitoring stack violation (P3 + §5 anti-patterns)
grep -r "grafana\|prometheus\|dashboard\|visual.*ui\|web.*frontend" crates/ --include="*.rs"

# Stub / dead-code violation (P5)
grep -r "todo!\|unimplemented!\|#\[deprecated\]" crates/ --include="*.rs"

# Magna Carta compliance
kask sovereignty verify

# CNS span health
kask cns health
```
