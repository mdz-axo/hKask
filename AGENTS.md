# Agent Operating Guide — hKask

**hKask** (ℏKask) — A Minimal Viable Container for Replicants | `kask` binary | `hkask-` crate prefix | v0.31.0

---

## Capability Catalog

**39 Skills**, **2 Templates**, **1 Bundle**, **1 Legacy**. **43 capabilities total.**

| Type | Behavior |
|------|----------|
| **Skill** | PDCA FlowDef with convergence threshold + energy budget + loop action |
| **Template** | One-shot prompt execution, no registry manifest |
| **Bundle** | Composition orchestrator, delegates to sub-skills (non-PDCA) |

### Guardrails (activate first)

| Skill | When to Activate |
|-------|-----------------|
| **coding-guidelines** | Before writing or reviewing any code |

### Core Development

| Skill | When to Activate |
|-------|-----------------|
| **bug-hunt** | Bug hunting. Run expeditions against target crates. |
| **tdd** | Building features or fixing bugs. RED→GREEN→REFACTOR. |
| **diagnose** | Debugging hard bugs or performance regressions. |
| **deep-module** | Module design. Deletion test, interface minimalism. |
| **refactor-service-layer** | Extracting duplicated logic from CLI/API/MCP surfaces. |
| **improve-codebase-architecture** | Finding deepening opportunities in the codebase. |
| **strangler-fig** | Incremental architectural migration. |
| **idiomatic-rust** | Type-driven Rust design through Hoare's principles. |
| **diataxis-diagram** | Generate Mermaid diagrams (ERD, flowchart, state, sequence, class) from code with Diataxis quality evaluation. |

### Reasoning & Analysis

| Skill | When to Activate |
|-------|-----------------|
| **pragmatic-semantics** | Classify statements by certainty, constraint force, provenance. |
| **pragmatic-cybernetics** | Feedback loops, variety engineering, system homeostasis. |
| **pragmatic-laziness** | Find the path of least action through meaning-space. |
| **essentialist** | Recursive eliminative interrogation (Exist → Surface → Contract). |
| **review** | Self-critique for contradictions, unsupported claims, logical gaps. |
| **grill-me** | Socratic questioning to stress-test understanding. |
| **zoom-out** | Broader context on unfamiliar code. |
| **sequential-inquiry** | Dynamic chain-of-thought with automatic deep-dive delegation. |

### Kata & Coaching

| Skill | When to Activate |
|-------|-----------------|
| **kata** (Bundle) | Toyota Kata system — starter + improvement + coaching. |
| **kata-coaching** | 5-question Coaching Kata dialogue. |
| **kata-improvement** | 4-step Improvement Kata PDCA pattern. |
| **kata-starter** | Foundational kata practice routines. |
| **improv** | Agent interaction grammar (Plussing, Yes And, Freestyling, Riffing). |

### Meta & Maintenance

| Skill | When to Activate |
|-------|-----------------|
| **skill-maintenance** | Audit skill architecture for staleness, coverage gaps. |
| **skill-logic-audit** | Audit .j2 template logic against stated goals. |
| **skill-bundler** | Compose multiple skills into a cohesive bundle. |
| **handoff** | Session handoff — capture what was done, what remains. |

### Specialized

| Skill | When to Activate |
|-------|-----------------|
| **superforecasting** | Calibrated probability forecasting (Tetlock). |
| **mcda** | Multi-Criteria Decision Analysis. |
| **scenario-builder** | Schwartz scenario planning. |
| **hypothesis-framer** | Research question framing (FINER + PICO). |
| **adversarial-red-team** | Adversarial robustness testing. |
| **goal-analysis** | Goal specification and completion verification. |
| **magna-carta-verifier** | Verify Magna Carta principles enforcement. |
| **structured-extraction** | Extract structured data from unstructured text. |
| **caveman** | Multi-mode text compression. |
| **self-critique-revision** | Iterative self-critique and revision cycle. |
| **logo-builder** (Template) | Pragmatic logo design. |
| **qa-script-builder** (Template) | Design autonomous QA pipeline manifests. |

---

## Prohibitions

From Magna Carta (P1–P4) and P12. Violations **must be deleted**.

| # | Prohibition | Principle |
|---|-------------|-----------|
| 1 | No `todo!()`, `unimplemented!()`, `#[deprecated]`, unused traits, or stubs | P5 · P3 |
| 2 | No anonymous agency — every action has an authenticated author | P12 · P1 |
| 3 | No hidden parameters or admin-gated settings | P3 |
| 4 | No pass-through abstractions (deep-module discipline) | P5 · P7 |

See CI invariants job for current enforcement of hierarchy and constraint force classification.

---

## Tooling Policy

hKask is a Rust project. Python is **not** an acceptable project dependency. Ad-hoc Python scripts are permitted during exploration but must be deleted before work is complete. Generated artifacts (JSON manifests, inventories) must also be removed.

Preferred auxiliary tooling: shell (`bash`) under `scripts/`, Rust binaries or `build.rs` for source/Cargo metadata.

---

## Key Docs

- `.github/workflows/ci.yml` — CI pipeline (fmt, clippy, unused-deps, build, test, doc, invariants)
- `.github/workflows/audit.yml` — Weekly dependency audit (cargo-deny + cargo-audit)
- `crates/hkask-types/src/cns.rs` — CNS span registry
- `crates/hkask-types/src/lib.rs` — Foundation types
- `crates/hkask-ports/src/lib.rs` — Hexagonal port traits
- `crates/hkask-agents/src/curator_agent/metacognition.rs` — Curator metacognition
- Dependency governance: CI unused-deps job (`nightly -D unused_crate_dependencies`)
- Feature gating: `hkask-communication` matrix feature, `hkask-cli` communication/tui/api features

> Architecture docs (`docs/architecture/`) are under reconstruction. CI invariants and crate-level doc comments are the current authority for design constraints.


