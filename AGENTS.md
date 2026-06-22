# Agent Operating Guide — hKask

**hKask** (ℏKask) — A Minimal Viable Container for Agents | `kask` binary | `hkask-` crate prefix | v0.30.0

---

## Capability Catalog

Activate via `skill` tool when conditions are met. **46 total** — all currently **Templates** (one-shot). Skills (PDCA FlowDef loops with quality threshold + energy budget) are added as manifests are upgraded.

| Type | Invocation | Behavior | Exit |
|------|-----------|----------|------|
| **Template** | `kask run <name>` | One-shot prompt execution | Returns output |
| **Skill** | `kask run <name>` | Iterative PDCA cascade | Returns `converged \| maxed_out \| escalated` |

Templates are useful. Skills practice toward excellence. A Template may be composed INTO a Skill as a step within the PDCA loop, but it is not itself a Skill unless it has a FlowDef manifest with `convergence.threshold > 0`, `gas.cap > 0`, and a `loop` action.

### Guardrails (activate first)

| Name | Type | When to Activate |
|------|------|-----------------|
| **coding-guidelines** | Template | Before writing or reviewing any code. Surfaces assumptions, enforces simplicity, surgical changes, goal-driven execution. |

### Core Development

| Name | Type | When to Activate |
|------|------|-----------------|
| **bug-hunt** | Template | Bug hunting. Run expeditions against target crates to find threats to user-defined quality. |
| **tdd** | Template | Building features or fixing bugs. Vertical tracer-bullet RED→GREEN→REFACTOR. |
| **diagnose** | Template | Debugging hard bugs or performance regressions. Reproduce, anchor, hypothesise, instrument, fix, regression-test, verify. |
| **deep-module** | Template | Module design discipline. Apply the deletion test, enforce depth, interface minimalism (≤7 public functions). |
| **refactor-service-layer** | Template | Extracting duplicated business logic from CLI/API/MCP surfaces into `hkask-services`. |
| **improve-codebase-architecture** | Template | Finding deepening opportunities. Walk codebase for shallow modules, tight coupling, untested seams. |
| **strangler-fig** | Template | Incremental architectural migration. Introduce new alongside old, migrate one domain at a time. |
| **rust-expertise** | Template | Idiomatic Rust design: type-driven design, ownership as architecture, fearless refactoring. |

### Reasoning & Analysis

| Name | Type | When to Activate |
|------|------|-----------------|
| **pragmatic-semantics** | Template | Classifying statements by certainty level and constraint force. Distinguish IS from OUGHT. |
| **pragmatic-cybernetics** | Template | Cybernetic reasoning: feedback loops, variety engineering, system homeostasis. |
| **pragmatic-laziness** | Template | Procedural composition. Finds the path of least action through meaning-space. |
| **essentialist** | Template | Recursive eliminative interrogation. 3-gate challenge loop (Exist → Surface → Contract). |
| **constraint-forces** | Template | Classify constraints by force type (Prohibition, Guardrail, Guideline, Evidence, Hypothesis). |
| **review** | Template | Self-critique output for contradictions, unsupported claims, logical gaps. |
| **grill-me** | Template | Socratic questioning to stress-test understanding. Probes knowledge gaps. |
| **zoom-out** | Template | Broader context or higher-level perspective on unfamiliar code. |

### Kata & Coaching

| Name | Type | When to Activate |
|------|------|-----------------|
| **kata** | Template | Toyota Kata system — composes starter, improvement, and coaching. |
| **kata-coaching** | Template | The 5-question Coaching Kata dialogue for teaching scientific thinking. |
| **kata-improvement** | Template | The 4-step Improvement Kata: Understand Direction, Grasp Current Condition, Establish Target, PDCA iterate. |
| **kata-starter** | Template | Foundational kata practice routines: Five Questions Drill, PDCA Cycle, Observation Drill. |
| **improv** | Template | Agent interaction grammar — Plussing, Yes And, Yes But, Freestyling, Riffing. |

### Meta & Maintenance

| Name | Type | When to Activate |
|------|------|-----------------|
| **skill-maintenance** | Template | Audit hKask's skill architecture for staleness, coverage gaps, and quality degradation. |
| **skill-discovery** | Template | Find, evaluate, and install skills. Detect capability gaps and search for candidates. |
| **skill-manager** | Template | CRUD for the skill corpus. List, validate, build, install, and prune skills. |
| **skill-logic-audit** | Template | Adversarial audit of .j2 template logic against stated goals. |
| **skill-bundler** | Template | Orchestrate and compose multiple skills into a cohesive bundle. |
| **document-update** | Template | Documentation corpus maintenance. 7-task workflow: audit, merge, archive, portal refresh. |
| **handoff** | Template | Session handoff protocol. Captures what was done, what remains, key decisions. |
| **condenser-continuation** | Template | Resuming condenser implementation after context reset. |

### Specialized

| Name | Type | When to Activate |
|------|------|-----------------|
| **superforecasting** | Template | Calibrated probability forecasting (Tetlock). 8-stage pipeline. |
| **decision-journal** | Template | Kahneman-style decision journal. Record decisions, schedule revisit. |
| **mcda** | Template | Multi-Criteria Decision Analysis. Weight, score, rank alternatives. |
| **scenario-builder** | Template | Schwartz scenario planning. Focal question → STEEP → 2×2 → narratives. |
| **adversarial-red-team** | Template | Adversarial robustness testing. ATLAS/GARAK-aligned taxonomy. |
| **goal-analysis** | Template | Goal specification and verification. Extract goals, judge completion. |
| **magna-carta-verifier** | Template | Verify Magna Carta principles (P1–P4) are correctly implemented and enforced. |
| **structured-extraction** | Template | Extract structured data from unstructured text. Entity identification, relation extraction. |
| **chain-of-density** | Template | Iterative density-increase summarization (Gao et al. 2024). |
| **gentle-lovelace** | Template | 4-dimensional technical documentation quality evaluator. |
| **caveman** | Template | Ultra-compact compression mode. Drop filler, preserve technical substance. |
| **self-critique-revision** | Template | Iterative self-critique and revision cycle: draft → critique → revise. |
| **dokkodo-mindset** | Template | Perceptual filter based on Musashi's 21 Dokkodo precepts. |
| **falstaffian-perspective** | Template | Multi-iteration perspective generation through semantic shape transforms. |
| **logo-builder** | Template | Pragmatic logo design using LLM-assisted generation. 3-phase pipeline. |
| **qa-script-builder** | Template | Design and generate autonomous QA pipeline manifests. 5-phase pipeline. |

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
