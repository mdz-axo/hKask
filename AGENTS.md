# Agent Operating Guide — hKask

**hKask** (ℏKask) — A Minimal Viable Container for Agents | `kask` binary | `hkask-` crate prefix | v0.30.0

---

## Skills (Must Load Before Work)

Activate the relevant skill via `skill` tool when its conditions are met. **45 skills total** across the corpus — these are the most frequently used.

### Guardrails (activate first)

| Skill | When to Activate |
|-------|-----------------|
| **coding-guidelines** | Before writing or reviewing any code. Surfaces assumptions, enforces simplicity, surgical changes, goal-driven execution. |

### Core Development

| Skill | When to Activate |
|-------|-----------------|
| **tdd** | Building features or fixing bugs. Vertical tracer-bullet RED→GREEN→REFACTOR. Anchored on the Testing Discipline (Property-Based Testing + CNS observability). |
| **diagnose** | Debugging hard bugs or performance regressions. Reproduce, anchor, hypothesise, instrument, fix, regression-test, verify. |
| **deep-module** | Module design discipline. Apply the deletion test, enforce depth (benefit/cost ratio), interface minimalism (≤7 public functions). |
| **refactor-service-layer** | Extracting duplicated business logic from CLI/API/MCP surfaces into `hkask-services`. Strangler fig pattern, deep-module discipline. |
| **improve-codebase-architecture** | Finding deepening opportunities. Walk codebase for shallow modules, tight coupling, untested seams. |
| **strangler-fig** | Incremental architectural migration. Introduce new alongside old, migrate one domain at a time, both paths delegate before deletion. |
| **rust-expertise** | Idiomatic Rust design: type-driven design, ownership as architecture, fearless refactoring, zero-cost abstraction. |

### Reasoning & Analysis

| Skill | When to Activate |
|-------|-----------------|
| **pragmatic-semantics** | Classifying statements by certainty level and constraint force. Distinguish IS from OUGHT, declarative from probabilistic. |
| **pragmatic-cybernetics** | Cybernetic reasoning: feedback loops, variety engineering, system homeostasis. Use when diagnosing CNS alerts or feedback failures. |
| **pragmatic-laziness** | Procedural composition. Finds the path of least action through meaning-space via a 3-phase lazy loop. |
| **essentialist** | Recursive eliminative interrogation. 3-gate challenge loop (Exist → Surface → Contract). Use when simplifying, deleting, or auditing complexity. |
| **constraint-forces** | Classify constraints by force type (Prohibition, Guardrail, Guideline, Evidence, Hypothesis). |
| **review** | Self-critique output for contradictions, unsupported claims, logical gaps, and confidence calibration. |
| **grill-me** | Socratic questioning to stress-test understanding. Probes knowledge gaps and challenges assumptions. |
| **zoom-out** | Broader context or higher-level perspective on unfamiliar code. |

### Kata & Coaching

| Skill | When to Activate |
|-------|-----------------|
| **kata** | Toyota Kata system — composes starter, improvement, and coaching into scientific capability development. |
| **kata-coaching** | The 5-question Coaching Kata dialogue for teaching scientific thinking. |
| **kata-improvement** | The 4-step Improvement Kata: Understand Direction, Grasp Current Condition, Establish Target, PDCA iterate. |
| **kata-starter** | Foundational kata practice routines: Five Questions Drill, PDCA Cycle, Observation Drill. |
| **improv** | Agent interaction grammar — Plussing, Yes And, Yes But, Freestyling, Riffing. Use `/improv` in REPL. |

### Meta & Maintenance

| Skill | When to Activate |
|-------|-----------------|
| **skill-maintenance** | Audit hKask's skill architecture for staleness, coverage gaps, and quality degradation. |
| **skill-discovery** | Find, evaluate, and install skills. Detect capability gaps and search for candidates. |
| **skill-manager** | CRUD for the skill corpus. List, validate, build, install, and prune skills. |
| **skill-logic-audit** | Adversarial audit of .j2 template logic against stated goals. Soundness filter, revision proposals. |
| **skill-bundler** | Orchestrate and compose multiple skills into a cohesive bundle with conflict resolution and cascade ordering. |
| **skill-translator** | Translate external skills into hKask registry crates and generate SKILL.md companions. |
| **document-update** | Systematic documentation corpus maintenance. 7-task workflow: README fact audit, docs inventory, identify merge candidates, extract & merge, archive, portal refresh. |
| **handoff** | Session handoff protocol. Captures what was done, what remains, key decisions, and next steps. |
| **condenser-continuation** | Resuming condenser implementation after context reset. Restores session state, prioritizes remaining tasks, verifies build health. |

### Specialized

| Skill | When to Activate |
|-------|-----------------|
| **superforecasting** | Calibrated probability forecasting (Tetlock). 8-stage pipeline: triage → Fermi → outside/inside view → evidence → synthesis → calibration → record. |
| **decision-journal** | Kahneman-style decision journal. Record decisions, define predictions, compute Brier scores, revisit outcomes. |
| **mcda** | Multi-Criteria Decision Analysis. Weight, score, rank alternatives with sensitivity analysis. |
| **scenario-builder** | Schwartz scenario planning. Focal question → key forces → STEEP → 2×2 axes → narratives → implications. |
| **adversarial-red-team** | Adversarial robustness testing. Select targets, generate inputs, evaluate resistance with ATLAS/GARAK taxonomy. |
| **goal-analysis** | Goal specification and verification. Extract goals, judge completion via semantic evaluation or command execution. |
| **magna-carta-verifier** | Verify Magna Carta principles (P1–P4) are correctly implemented and enforced. |
| **structured-extraction** | Extract structured data from unstructured text. Entity identification, relation extraction, schema mapping. |
| **chain-of-density** | Iterative density-increase summarization (Gao et al. 2024). Converges when density improvement falls below threshold. |
| **gentle-lovelace** | 4-dimensional technical documentation quality evaluator (Hopper, Lovelace, Schriver, Gentle). |
| **caveman** | Ultra-compact compression mode. Drop filler, articles, pleasantries while preserving technical substance. |
| **self-critique-revision** | Iterative self-critique and revision cycle: draft → critique → revise. |
| **dokkodo-mindset** | Perceptual filter based on Musashi's 21 Dokkodo precepts. Clarifies perception by removing attachment and bias. |
| **falstaffian-perspective** | Multi-iteration perspective generation through semantic shape transforms with inference enrichment. |
| **logo-builder** | Pragmatic logo design using LLM-assisted generation. 3-phase pipeline: discovery → formal generation → refinement. |
| **qa-script-builder** | Design and generate autonomous QA pipeline manifests. 4-phase pipeline: discover → design → generate → validate. Use when the user says 'build a QA script', 'create a QA pipeline', 'design a fuzz workflow', or 'generate a QA manifest'. |

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
